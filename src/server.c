#ifndef _WIN32
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif
#ifndef _FILE_OFFSET_BITS
#define _FILE_OFFSET_BITS 64
#endif
#endif

#ifdef _WIN32
#include <winsock2.h>
#include <windows.h>
#include <io.h>
#include <wchar.h>
#else
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/types.h>
#include <netinet/in.h>
#include <fcntl.h>
#include <poll.h>
#include <time.h>
#include <unistd.h>
#endif

#include <sys/stat.h>
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define DMMT_HEADER_LIMIT 8192
#define DMMT_PATH_LIMIT 4096
#define DMMT_FILE_PATH_LIMIT (DMMT_PATH_LIMIT + 32)

#ifdef _WIN32
typedef SOCKET dmmt_socket;
typedef struct _stati64 dmmt_file_info;
#define DMMT_INVALID_SOCKET INVALID_SOCKET
#define DMMT_IS_DIRECTORY(mode) (((mode) & _S_IFMT) == _S_IFDIR)
#define DMMT_IS_REGULAR(mode) (((mode) & _S_IFMT) == _S_IFREG)
#else
typedef int dmmt_socket;
typedef struct stat dmmt_file_info;
#define DMMT_INVALID_SOCKET (-1)
#define DMMT_IS_DIRECTORY(mode) S_ISDIR(mode)
#define DMMT_IS_REGULAR(mode) S_ISREG(mode)
#endif

uintptr_t dmmt_server_open(uint16_t port, int *error);
uintptr_t dmmt_server_accept(uintptr_t listener, int *error);
void dmmt_server_close(uintptr_t socket);
int dmmt_server_handle(uintptr_t client);
int dmmt_server_send(uintptr_t client, const unsigned char *data, size_t length);
uint16_t dmmt_server_port(uintptr_t listener, int *error);

static int dmmt_last_error(void) {
#ifdef _WIN32
    return WSAGetLastError();
#else
    return errno;
#endif
}

static void dmmt_set_error(int *error, int code) {
    if (error != NULL) {
        *error = code;
    }
#ifdef _WIN32
    WSASetLastError(code);
#else
    errno = code;
#endif
}

static int dmmt_interrupted(int code) {
#ifdef _WIN32
    return code == WSAEINTR;
#else
    return code == EINTR;
#endif
}

static int dmmt_timed_out(int code) {
#ifdef _WIN32
    return code == WSAETIMEDOUT || code == WSAEWOULDBLOCK;
#else
    return code == EAGAIN || code == EWOULDBLOCK || code == ETIMEDOUT;
#endif
}

static int dmmt_socket_from_handle(uintptr_t handle, dmmt_socket *socket) {
#ifdef _WIN32
    if (handle == UINTPTR_MAX) {
        dmmt_set_error(NULL, WSAENOTSOCK);
        return -1;
    }
#else
    if (handle > (uintptr_t)INT_MAX) {
        dmmt_set_error(NULL, EBADF);
        return -1;
    }
#endif
    *socket = (dmmt_socket)handle;
    return 0;
}

static void dmmt_close_native(dmmt_socket socket) {
#ifdef _WIN32
    closesocket(socket);
#else
    close(socket);
#endif
}

static int dmmt_set_nonblocking(dmmt_socket socket, int enabled) {
#ifdef _WIN32
    u_long mode = enabled ? 1UL : 0UL;
    return ioctlsocket(socket, (long)FIONBIO, &mode) == SOCKET_ERROR ? -1 : 0;
#else
    int flags = fcntl(socket, F_GETFL, 0);
    if (flags < 0) {
        return -1;
    }
    if (enabled) {
        flags |= O_NONBLOCK;
    } else {
        flags &= ~O_NONBLOCK;
    }
    return fcntl(socket, F_SETFL, flags);
#endif
}

static int dmmt_milliseconds(uint64_t *milliseconds) {
#ifdef _WIN32
    *milliseconds = (uint64_t)GetTickCount64();
#else
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
        return -1;
    }
    *milliseconds = (uint64_t)now.tv_sec * UINT64_C(1000) +
                    (uint64_t)now.tv_nsec / UINT64_C(1000000);
#endif
    return 0;
}

static dmmt_socket dmmt_accept_ready(dmmt_socket listener) {
    uint64_t start;
    if (dmmt_milliseconds(&start) != 0) {
        return DMMT_INVALID_SOCKET;
    }
    for (;;) {
        uint64_t now;
        uint64_t elapsed;
        dmmt_socket client;
        int remaining;
        int ready;
        if (dmmt_milliseconds(&now) != 0) {
            return DMMT_INVALID_SOCKET;
        }
        elapsed = now - start;
        if (elapsed >= UINT64_C(250)) {
#ifdef _WIN32
            dmmt_set_error(NULL, WSAEWOULDBLOCK);
#else
            dmmt_set_error(NULL, EAGAIN);
#endif
            return DMMT_INVALID_SOCKET;
        }
        remaining = (int)(UINT64_C(250) - elapsed);
#ifdef _WIN32
        {
            fd_set readers;
            struct timeval timeout;
            FD_ZERO(&readers);
            FD_SET(listener, &readers);
            timeout.tv_sec = 0;
            timeout.tv_usec = (long)remaining * 1000L;
            ready = select(0, &readers, NULL, NULL, &timeout);
        }
#else
        {
            struct pollfd descriptor;
            descriptor.fd = listener;
            descriptor.events = POLLIN;
            descriptor.revents = 0;
            ready = poll(&descriptor, 1, remaining);
            if (ready > 0 && (descriptor.revents & POLLNVAL) != 0) {
                dmmt_set_error(NULL, EBADF);
                return DMMT_INVALID_SOCKET;
            }
        }
#endif
        if (ready < 0) {
            if (dmmt_interrupted(dmmt_last_error())) {
                continue;
            }
            return DMMT_INVALID_SOCKET;
        }
        if (ready == 0) {
#ifdef _WIN32
            dmmt_set_error(NULL, WSAEWOULDBLOCK);
#else
            dmmt_set_error(NULL, EAGAIN);
#endif
            return DMMT_INVALID_SOCKET;
        }
        client = accept(listener, NULL, NULL);
        if (client != DMMT_INVALID_SOCKET || !dmmt_interrupted(dmmt_last_error())) {
            return client;
        }
    }
}

static int dmmt_configure_client(dmmt_socket socket) {
    if (dmmt_set_nonblocking(socket, 0) != 0) {
        return -1;
    }
#ifdef _WIN32
    DWORD timeout = 10000;
    if (setsockopt(socket, SOL_SOCKET, SO_RCVTIMEO,
                   (const char *)&timeout, (int)sizeof(timeout)) == SOCKET_ERROR ||
        setsockopt(socket, SOL_SOCKET, SO_SNDTIMEO,
                   (const char *)&timeout, (int)sizeof(timeout)) == SOCKET_ERROR) {
        return -1;
    }
#else
    struct timeval timeout;
    timeout.tv_sec = 10;
    timeout.tv_usec = 0;
    if (setsockopt(socket, SOL_SOCKET, SO_RCVTIMEO,
                   &timeout, (socklen_t)sizeof(timeout)) < 0 ||
        setsockopt(socket, SOL_SOCKET, SO_SNDTIMEO,
                   &timeout, (socklen_t)sizeof(timeout)) < 0) {
        return -1;
    }
#ifdef SO_NOSIGPIPE
    {
        int enabled = 1;
        if (setsockopt(socket, SOL_SOCKET, SO_NOSIGPIPE,
                       &enabled, (socklen_t)sizeof(enabled)) < 0) {
            return -1;
        }
    }
#endif
#endif
    return 0;
}

uintptr_t dmmt_server_open(uint16_t port, int *error) {
    dmmt_socket listener;
    struct sockaddr_in address;
    int enabled = 1;
    int code;
#ifdef _WIN32
    WSADATA data;
    code = WSAStartup(MAKEWORD(2, 2), &data);
    if (code != 0) {
        dmmt_set_error(error, code);
        return UINTPTR_MAX;
    }
#endif
    listener = socket(AF_INET, SOCK_STREAM, 0);
    if (listener == DMMT_INVALID_SOCKET) {
        code = dmmt_last_error();
        goto failure;
    }
#ifdef _WIN32
    if (setsockopt(listener, SOL_SOCKET, SO_EXCLUSIVEADDRUSE,
                   (const char *)&enabled, (int)sizeof(enabled)) == SOCKET_ERROR) {
#else
    if (setsockopt(listener, SOL_SOCKET, SO_REUSEADDR,
                   &enabled, (socklen_t)sizeof(enabled)) < 0) {
#endif
        code = dmmt_last_error();
        goto failure;
    }
    memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_ANY);
    address.sin_port = htons(port);
    if (bind(listener, (const struct sockaddr *)&address,
#ifdef _WIN32
             (int)sizeof(address)
#else
             (socklen_t)sizeof(address)
#endif
             ) != 0 || listen(listener, 128) != 0) {
        code = dmmt_last_error();
        goto failure;
    }
    if (dmmt_set_nonblocking(listener, 1) != 0) {
        code = dmmt_last_error();
        goto failure;
    }
    if (error != NULL) {
        *error = 0;
    }
    return (uintptr_t)listener;

failure:
    if (listener != DMMT_INVALID_SOCKET) {
        dmmt_close_native(listener);
    }
#ifdef _WIN32
    WSACleanup();
#endif
    dmmt_set_error(error, code);
    return UINTPTR_MAX;
}

uintptr_t dmmt_server_accept(uintptr_t listener, int *error) {
    dmmt_socket native_listener;
    dmmt_socket client;
    int code;
    if (dmmt_socket_from_handle(listener, &native_listener) != 0) {
        dmmt_set_error(error, dmmt_last_error());
        return UINTPTR_MAX;
    }
#ifdef _WIN32
    {
        WSADATA data;
        code = WSAStartup(MAKEWORD(2, 2), &data);
        if (code != 0) {
            dmmt_set_error(error, code);
            return UINTPTR_MAX;
        }
    }
#endif
    client = dmmt_accept_ready(native_listener);
    if (client == DMMT_INVALID_SOCKET || dmmt_configure_client(client) != 0) {
        code = dmmt_last_error();
        if (client != DMMT_INVALID_SOCKET) {
            dmmt_close_native(client);
        }
#ifdef _WIN32
        WSACleanup();
#endif
        dmmt_set_error(error, code);
        return UINTPTR_MAX;
    }
    if (error != NULL) {
        *error = 0;
    }
    return (uintptr_t)client;
}

void dmmt_server_close(uintptr_t socket) {
    dmmt_socket native_socket;
    if (dmmt_socket_from_handle(socket, &native_socket) != 0) {
        return;
    }
    dmmt_close_native(native_socket);
#ifdef _WIN32
    WSACleanup();
#endif
}

uint16_t dmmt_server_port(uintptr_t listener, int *error) {
    dmmt_socket native_listener;
    struct sockaddr_in address;
#ifdef _WIN32
    int length = (int)sizeof(address);
#else
    socklen_t length = (socklen_t)sizeof(address);
#endif
    if (dmmt_socket_from_handle(listener, &native_listener) != 0 ||
        getsockname(native_listener, (struct sockaddr *)&address, &length) != 0) {
        dmmt_set_error(error, dmmt_last_error());
        return 0;
    }
    if (address.sin_family != AF_INET) {
#ifdef _WIN32
        dmmt_set_error(error, WSAEAFNOSUPPORT);
#else
        dmmt_set_error(error, EAFNOSUPPORT);
#endif
        return 0;
    }
    if (error != NULL) {
        *error = 0;
    }
    return ntohs(address.sin_port);
}

int dmmt_server_send(uintptr_t client, const unsigned char *data, size_t length) {
    dmmt_socket socket;
    if (dmmt_socket_from_handle(client, &socket) != 0) {
        return -1;
    }
    if (data == NULL && length != 0) {
#ifdef _WIN32
        dmmt_set_error(NULL, WSAEINVAL);
#else
        dmmt_set_error(NULL, EINVAL);
#endif
        return -1;
    }
    while (length != 0) {
        size_t amount = length > (size_t)INT_MAX ? (size_t)INT_MAX : length;
#ifdef _WIN32
        int sent = send(socket, (const char *)data, (int)amount, 0);
#else
        ssize_t sent = send(socket, data, amount,
#ifdef MSG_NOSIGNAL
                            MSG_NOSIGNAL
#else
                            0
#endif
                            );
#endif
        if (sent < 0) {
            if (dmmt_interrupted(dmmt_last_error())) {
                continue;
            }
            return -1;
        }
        if (sent == 0) {
#ifdef _WIN32
            dmmt_set_error(NULL, WSAECONNRESET);
#else
            dmmt_set_error(NULL, EPIPE);
#endif
            return -1;
        }
        data += (size_t)sent;
        length -= (size_t)sent;
    }
    return 0;
}

static int dmmt_receive(dmmt_socket socket, char *buffer, size_t length) {
    for (;;) {
#ifdef _WIN32
        int received = recv(socket, buffer, (int)length, 0);
#else
        ssize_t received = recv(socket, buffer, length, 0);
#endif
        if (received >= 0) {
            return (int)received;
        }
        if (!dmmt_interrupted(dmmt_last_error())) {
            return -1;
        }
    }
}

static const char *dmmt_reason(int status) {
    switch (status) {
        case 200: return "OK";
        case 400: return "Bad Request";
        case 403: return "Forbidden";
        case 404: return "Not Found";
        case 405: return "Method Not Allowed";
        case 408: return "Request Timeout";
        case 414: return "URI Too Long";
        case 431: return "Request Header Fields Too Large";
        default: return "Internal Server Error";
    }
}

static int dmmt_send_headers(uintptr_t client, const char *version, int status,
                             const char *type, uintmax_t length, int streaming) {
    char header[512];
    char length_header[80];
    int count;
    if (streaming) {
        length_header[0] = '\0';
    } else {
        count = snprintf(length_header, sizeof(length_header),
                         "Content-Length: %" PRIuMAX "\r\n", length);
        if (count < 0 || (size_t)count >= sizeof(length_header)) {
            return -1;
        }
    }
    count = snprintf(header, sizeof(header),
                     "%s %d %s\r\n"
                     "Content-Type: %s\r\n"
                     "%s%s"
                     "Cache-Control: no-cache, no-store, must-revalidate\r\n"
                     "Connection: close\r\n"
                     "\r\n",
                     version, status, dmmt_reason(status), type, length_header,
                     status == 405 ? "Allow: GET, HEAD\r\n" : "");
    if (count < 0 || (size_t)count >= sizeof(header)) {
        return -1;
    }
    return dmmt_server_send(client, (const unsigned char *)header, (size_t)count);
}

static int dmmt_send_error(uintptr_t client, const char *version, int status,
                           int head_only) {
    char body[96];
    int count = snprintf(body, sizeof(body), "%d %s\n", status, dmmt_reason(status));
    if (count < 0 || (size_t)count >= sizeof(body) ||
        dmmt_send_headers(client, version, status, "text/plain; charset=utf-8",
                          (uintmax_t)count, 0) != 0) {
        return -1;
    }
    if (head_only) {
        return 0;
    }
    return dmmt_server_send(client, (const unsigned char *)body, (size_t)count);
}

static int dmmt_read_headers(dmmt_socket socket, char *buffer) {
    size_t used = 0;
    buffer[0] = '\0';
    while (used < DMMT_HEADER_LIMIT) {
        size_t start = used;
        size_t i;
        int received = dmmt_receive(socket, buffer + used, DMMT_HEADER_LIMIT - used);
        if (received < 0) {
            return dmmt_timed_out(dmmt_last_error()) ? 408 : -1;
        }
        if (received == 0) {
            return 400;
        }
        used += (size_t)received;
        buffer[used] = '\0';
        for (i = start; i < used; ++i) {
            unsigned char c = (unsigned char)buffer[i];
            if (c == 127 || (c < 32 && c != '\r' && c != '\n' && c != '\t') ||
                (c == '\n' && (i == 0 || buffer[i - 1] != '\r')) ||
                (i != 0 && buffer[i - 1] == '\r' && c != '\n')) {
                return 400;
            }
            if (i >= 3 && memcmp(buffer + i - 3, "\r\n\r\n", 4) == 0) {
                buffer[i + 1] = '\0';
                return 0;
            }
        }
    }
    return 431;
}

static int dmmt_is_token(unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
           (c >= '0' && c <= '9') ||
           (c != 0 && strchr("!#$%&'*+-.^_`|~", (int)c) != NULL);
}

static int dmmt_equal_ascii(const char *text, size_t length, const char *expected) {
    size_t i;
    if (strlen(expected) != length) {
        return 0;
    }
    for (i = 0; i < length; ++i) {
        unsigned char c = (unsigned char)text[i];
        if (c >= 'A' && c <= 'Z') {
            c = (unsigned char)(c + ('a' - 'A'));
        }
        if (c != (unsigned char)expected[i]) {
            return 0;
        }
    }
    return 1;
}

static int dmmt_parse_request(char *buffer, char **method, char **target,
                              const char **version) {
    char *line_end = strstr(buffer, "\r\n");
    char *space;
    char *cursor;
    int hosts = 0;
    if (line_end == NULL) {
        return 400;
    }
    *line_end = '\0';
    *method = buffer;
    space = strchr(buffer, ' ');
    if (space == NULL || space == buffer) {
        return 400;
    }
    for (cursor = buffer; cursor < space; ++cursor) {
        if (!dmmt_is_token((unsigned char)*cursor)) {
            return 400;
        }
    }
    *space = '\0';
    *target = space + 1;
    space = strchr(*target, ' ');
    if (space == NULL || space == *target) {
        return 400;
    }
    for (cursor = *target; cursor < space; ++cursor) {
        unsigned char c = (unsigned char)*cursor;
        if (c <= 32 || c == 127 || c == '#') {
            return 400;
        }
    }
    *space = '\0';
    if (strcmp(space + 1, "HTTP/1.0") != 0 && strcmp(space + 1, "HTTP/1.1") != 0) {
        return 400;
    }
    *version = space + 1;
    cursor = line_end + 2;
    while (cursor[0] != '\r' || cursor[1] != '\n') {
        char *end = strstr(cursor, "\r\n");
        char *colon;
        char *value;
        char *p;
        if (end == NULL) {
            return 400;
        }
        colon = (char *)memchr(cursor, ':', (size_t)(end - cursor));
        if (colon == NULL || colon == cursor) {
            return 400;
        }
        for (p = cursor; p < colon; ++p) {
            if (!dmmt_is_token((unsigned char)*p)) {
                return 400;
            }
        }
        if (dmmt_equal_ascii(cursor, (size_t)(colon - cursor), "host")) {
            value = colon + 1;
            while (value < end && (*value == ' ' || *value == '\t')) {
                ++value;
            }
            p = end;
            while (p > value && (p[-1] == ' ' || p[-1] == '\t')) {
                --p;
            }
            if (++hosts != 1 || value == p) {
                return 400;
            }
            while (value < p) {
                if (*value == ' ' || *value == '\t') {
                    return 400;
                }
                ++value;
            }
        }
        cursor = end + 2;
    }
    return strcmp(*version, "HTTP/1.1") == 0 && hosts != 1 ? 400 : 0;
}

static int dmmt_hex(unsigned char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return c - 'A' + 10;
    }
    return -1;
}

static int dmmt_decode_path(const char *target, char *path) {
    size_t input_length = strcspn(target, "?");
    size_t used = 0;
    size_t i;
    const char *segment;
    const char *p;
    if (target[0] != '/') {
        return 400;
    }
    if (input_length > DMMT_PATH_LIMIT) {
        return 414;
    }
    for (i = 0; i < input_length; ++i) {
        unsigned char c = (unsigned char)target[i];
        if (c == '%') {
            int high;
            int low;
            if (input_length - i < 3) {
                return 400;
            }
            high = dmmt_hex((unsigned char)target[i + 1]);
            low = dmmt_hex((unsigned char)target[i + 2]);
            if (high < 0 || low < 0) {
                return 400;
            }
            c = (unsigned char)(high * 16 + low);
            i += 2;
        }
        if (c < 32 || c == 127 || c == '\\' || c == ':') {
            return 400;
        }
#ifdef _WIN32
        if (strchr("\"<>|*?", (int)c) != NULL) {
            return 400;
        }
#endif
        path[used++] = (char)c;
    }
    path[used] = '\0';
    segment = path + 1;
    for (p = segment;; ++p) {
        if (*p == '/' || *p == '\0') {
            size_t length = (size_t)(p - segment);
            if ((length == 1 && segment[0] == '.') ||
                (length == 2 && segment[0] == '.' && segment[1] == '.')) {
                return 400;
            }
#ifdef _WIN32
            if (length != 0 && (segment[length - 1] == '.' || segment[length - 1] == ' ')) {
                return 400;
            }
#endif
            if (*p == '\0') {
                break;
            }
            segment = p + 1;
        }
    }
    return 0;
}

static const char *dmmt_mime(const char *path) {
    static const struct {
        const char *extension;
        const char *type;
    } types[] = {
        {"html", "text/html; charset=utf-8"},
        {"htm", "text/html; charset=utf-8"},
        {"css", "text/css; charset=utf-8"},
        {"js", "text/javascript; charset=utf-8"},
        {"mjs", "text/javascript; charset=utf-8"},
        {"json", "application/json"},
        {"map", "application/json"},
        {"png", "image/png"},
        {"jpg", "image/jpeg"},
        {"jpeg", "image/jpeg"},
        {"gif", "image/gif"},
        {"webp", "image/webp"},
        {"avif", "image/avif"},
        {"svg", "image/svg+xml"},
        {"ico", "image/vnd.microsoft.icon"},
        {"bmp", "image/bmp"},
        {"tif", "image/tiff"},
        {"tiff", "image/tiff"},
        {"txt", "text/plain; charset=utf-8"},
        {"xml", "application/xml"},
        {"wasm", "application/wasm"},
        {"pdf", "application/pdf"},
        {"woff", "font/woff"},
        {"woff2", "font/woff2"},
        {"ttf", "font/ttf"},
        {"otf", "font/otf"},
        {"mp4", "video/mp4"},
        {"webm", "video/webm"}
    };
    const char *extension = strrchr(path, '.');
    size_t i;
    if (extension != NULL) {
        ++extension;
        for (i = 0; i < sizeof(types) / sizeof(types[0]); ++i) {
            if (dmmt_equal_ascii(extension, strlen(extension), types[i].extension)) {
                return types[i].type;
            }
        }
    }
    return "application/octet-stream";
}

#ifdef _WIN32
static int dmmt_wide_path(const char *path, wchar_t *wide_path) {
    size_t length;
    for (length = 0; length < DMMT_FILE_PATH_LIMIT; ++length) {
        if (path[length] == '\0') {
            break;
        }
    }
    if (length == DMMT_FILE_PATH_LIMIT) {
        errno = ENAMETOOLONG;
        return -1;
    }
    if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, path, (int)(length + 1),
                            wide_path, DMMT_FILE_PATH_LIMIT) == 0) {
        errno = GetLastError() == ERROR_INSUFFICIENT_BUFFER ? ENAMETOOLONG : EILSEQ;
        return -1;
    }
    return 0;
}
#endif

static int dmmt_stat_path(const char *path, dmmt_file_info *info) {
#ifdef _WIN32
    wchar_t wide_path[DMMT_FILE_PATH_LIMIT];
    if (dmmt_wide_path(path, wide_path) != 0) {
        return -1;
    }
    return _wstati64(wide_path, info);
#else
    return stat(path, info);
#endif
}

static int dmmt_stat_file(FILE *file, dmmt_file_info *info) {
#ifdef _WIN32
    return _fstati64(_fileno(file), info);
#else
    return fstat(fileno(file), info);
#endif
}

static FILE *dmmt_open_file(const char *path) {
#ifdef _WIN32
    wchar_t wide_path[DMMT_FILE_PATH_LIMIT];
    if (dmmt_wide_path(path, wide_path) != 0) {
        return NULL;
    }
    return _wfopen(wide_path, L"rb");
#else
    int flags = O_RDONLY | O_NONBLOCK;
    int descriptor;
    FILE *file;
#ifdef O_CLOEXEC
    flags |= O_CLOEXEC;
#endif
    descriptor = open(path, flags);
    if (descriptor < 0) {
        return NULL;
    }
    file = fdopen(descriptor, "rb");
    if (file == NULL) {
        int code = errno;
        close(descriptor);
        errno = code;
    }
    return file;
#endif
}

static int dmmt_file_error(void) {
#ifdef _WIN32
    if (errno == EILSEQ) {
        return 400;
    }
#endif
    if (errno == EACCES || errno == EPERM) {
        return 403;
    }
    if (errno == ENOENT || errno == ENOTDIR || errno == ENAMETOOLONG) {
        return 404;
    }
    return 500;
}

static int dmmt_stream_file(uintptr_t client, FILE *file, uintmax_t remaining) {
    unsigned char buffer[16384];
    while (remaining != 0) {
        size_t amount = remaining > sizeof(buffer) ? sizeof(buffer) : (size_t)remaining;
        size_t count = fread(buffer, 1, amount, file);
        if (count == 0) {
            if (ferror(file) && errno == EINTR) {
                clearerr(file);
                continue;
            }
            return -1;
        }
        if (dmmt_server_send(client, buffer, count) != 0) {
            return -1;
        }
        remaining -= (uintmax_t)count;
    }
    return 0;
}

int dmmt_server_handle(uintptr_t client) {
    dmmt_socket socket;
    char request[DMMT_HEADER_LIMIT + 1];
    char path[DMMT_PATH_LIMIT + 1];
    char file_path[DMMT_FILE_PATH_LIMIT];
    char *method = NULL;
    char *target = NULL;
    const char *version = "HTTP/1.1";
    const char *root = "public";
    const char *relative;
    dmmt_file_info info;
    FILE *file;
    uintmax_t length;
    int status;
    int head_only;
    int directory_path;
    int count;
    int result;
    if (dmmt_socket_from_handle(client, &socket) != 0) {
        return -1;
    }
    status = dmmt_read_headers(socket, request);
    if (status < 0) {
        return -1;
    }
    head_only = strncmp(request, "HEAD ", 5) == 0;
    if (status != 0) {
        return dmmt_send_error(client, version, status, head_only);
    }
    status = dmmt_parse_request(request, &method, &target, &version);
    if (status != 0) {
        return dmmt_send_error(client, version, status, head_only);
    }
    if (strcmp(method, "GET") != 0 && strcmp(method, "HEAD") != 0) {
        return dmmt_send_error(client, version, 405, 0);
    }
    head_only = strcmp(method, "HEAD") == 0;
    status = dmmt_decode_path(target, path);
    if (status != 0) {
        return dmmt_send_error(client, version, status, head_only);
    }
    if (strcmp(path, "/sse") == 0) {
        result = dmmt_send_headers(client, version, 200, "text/event-stream", 0, !head_only);
        return result != 0 ? -1 : (head_only ? 0 : 1);
    }
    relative = path + 1;
    if (strncmp(relative, "tiles/", 6) == 0 || strcmp(relative, "tiles") == 0) {
        root = "tiles";
        relative += 5;
    } else if (strncmp(relative, "tiles_height/", 13) == 0 ||
               strcmp(relative, "tiles_height") == 0) {
        root = "tiles_height";
        relative += 12;
    }
    if (*relative == '/') {
        ++relative;
    }
    count = snprintf(file_path, sizeof(file_path), "%s/%s", root, relative);
    if (count < 0 || (size_t)count >= sizeof(file_path)) {
        return dmmt_send_error(client, version, 414, head_only);
    }
    directory_path = file_path[count - 1] == '/';
    while (count > 0 && file_path[count - 1] == '/') {
        file_path[--count] = '\0';
    }
    if (dmmt_stat_path(file_path, &info) != 0) {
        return dmmt_send_error(client, version, dmmt_file_error(), head_only);
    }
    if (DMMT_IS_DIRECTORY(info.st_mode)) {
        size_t used = (size_t)count;
        count = snprintf(file_path + used, sizeof(file_path) - used, "/index.html");
        if (count < 0 || (size_t)count >= sizeof(file_path) - used) {
            return dmmt_send_error(client, version, 414, head_only);
        }
        if (dmmt_stat_path(file_path, &info) != 0) {
            return dmmt_send_error(client, version, dmmt_file_error(), head_only);
        }
    } else if (directory_path) {
        return dmmt_send_error(client, version, 404, head_only);
    }
    if (!DMMT_IS_REGULAR(info.st_mode)) {
        return dmmt_send_error(client, version, 404, head_only);
    }
    file = dmmt_open_file(file_path);
    if (file == NULL) {
        return dmmt_send_error(client, version, dmmt_file_error(), head_only);
    }
    if (dmmt_stat_file(file, &info) != 0) {
        status = dmmt_file_error();
        fclose(file);
        return dmmt_send_error(client, version, status, head_only);
    }
    if (!DMMT_IS_REGULAR(info.st_mode) || info.st_size < 0) {
        fclose(file);
        return dmmt_send_error(client, version, 404, head_only);
    }
    length = (uintmax_t)info.st_size;
    result = dmmt_send_headers(client, version, 200, dmmt_mime(file_path), length, 0);
    if (result == 0 && !head_only) {
        result = dmmt_stream_file(client, file, length);
    }
    fclose(file);
    return result;
}
