use std::ffi::c_int;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, TryAcquireError, broadcast, watch};

const MAX_CLIENTS: usize = 128;
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

unsafe extern "C" {
    fn dmmt_server_open(port: u16, error: *mut c_int) -> usize;
    fn dmmt_server_accept(listener: usize, error: *mut c_int) -> usize;
    fn dmmt_server_close(socket: usize);
    fn dmmt_server_handle(client: usize) -> c_int;
    fn dmmt_server_send(client: usize, data: *const u8, length: usize) -> c_int;
    #[cfg(test)]
    fn dmmt_server_port(listener: usize, error: *mut c_int) -> u16;
}

struct Socket(usize);

impl Socket {
    fn bind(port: u16) -> io::Result<Self> {
        let mut error = 0;
        let socket = unsafe { dmmt_server_open(port, &mut error) };
        Self::from_handle(socket, error)
    }

    fn accept(&self) -> io::Result<Self> {
        let mut error = 0;
        let socket = unsafe { dmmt_server_accept(self.0, &mut error) };
        Self::from_handle(socket, error)
    }

    fn from_handle(socket: usize, error: c_int) -> io::Result<Self> {
        if socket == usize::MAX {
            Err(io::Error::from_raw_os_error(error))
        } else {
            Ok(Self(socket))
        }
    }

    fn send(&self, data: &[u8]) -> bool {
        unsafe { dmmt_server_send(self.0, data.as_ptr(), data.len()) == 0 }
    }

    #[cfg(test)]
    fn port(&self) -> u16 {
        let mut error = 0;
        let port = unsafe { dmmt_server_port(self.0, &mut error) };
        assert_eq!(error, 0);
        assert_ne!(port, 0);
        port
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { dmmt_server_close(self.0) }
    }
}

fn sse_frame(data: &str) -> String {
    let data = data.replace("\r\n", "\n").replace('\r', "\n");
    format!("data: {}\n\n", data.replace('\n', "\ndata: "))
}

async fn serve_client(
    client: Socket,
    mut events: broadcast::Receiver<String>,
    keep_alive: Duration,
) {
    let mut client = match tokio::task::spawn_blocking(move || {
        if unsafe { dmmt_server_handle(client.0) } == 1 {
            Some(client)
        } else {
            None
        }
    })
    .await
    {
        Ok(Some(client)) => client,
        _ => return,
    };

    loop {
        let frame = match tokio::time::timeout(keep_alive, events.recv()).await {
            Ok(Ok(data)) => sse_frame(&data),
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) => break,
            Err(_) => ": keep-alive\n\n".to_string(),
        };
        match tokio::task::spawn_blocking(move || {
            let sent = client.send(frame.as_bytes());
            (client, sent)
        })
        .await
        {
            Ok((socket, true)) => client = socket,
            _ => break,
        }
    }
}

struct Shutdown(watch::Sender<bool>);

impl Drop for Shutdown {
    fn drop(&mut self) {
        let _ = self.0.send(true);
    }
}

fn retry_accept(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock
            | io::ErrorKind::TimedOut
            | io::ErrorKind::Interrupted
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::NetworkDown
            | io::ErrorKind::NetworkUnreachable
            | io::ErrorKind::HostUnreachable
    )
}

async fn serve_listener(
    listener: Socket,
    events: broadcast::Sender<String>,
    max_clients: usize,
) -> io::Result<()> {
    let runtime = tokio::runtime::Handle::current();
    let (shutdown, stopping) = watch::channel(false);
    let _shutdown = Shutdown(shutdown);
    tokio::task::spawn_blocking(move || -> io::Result<()> {
        let clients = Arc::new(Semaphore::new(max_clients));
        while !*stopping.borrow() {
            let permit = match Arc::clone(&clients).try_acquire_owned() {
                Ok(permit) => permit,
                Err(TryAcquireError::NoPermits) => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(TryAcquireError::Closed) => return Ok(()),
            };
            let client = match listener.accept() {
                Ok(client) => client,
                Err(error) if retry_accept(&error) => continue,
                Err(error) => return Err(error),
            };
            let receiver = events.subscribe();
            let mut stopping = stopping.clone();
            runtime.spawn(async move {
                let _permit = permit;
                if *stopping.borrow() {
                    return;
                }
                tokio::select! {
                    _ = stopping.changed() => {}
                    _ = serve_client(client, receiver, KEEP_ALIVE_INTERVAL) => {}
                }
            });
        }
        Ok(())
    })
    .await
    .map_err(io::Error::other)?
}

pub async fn serve(port: u16, events: broadcast::Sender<String>) -> io::Result<()> {
    let listener = Socket::bind(port)?;
    println!("Starting web server on http://localhost:{port}");
    serve_listener(listener, events, MAX_CLIENTS).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpStream;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(root: &str) -> Self {
            let name = format!(
                "dmmt-server-test-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            );
            let path = PathBuf::from(root).join(name);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn url(&self, file: &str) -> String {
            format!("/{}/{}", self.0.to_str().unwrap().replace('\\', "/"), file)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn connect(port: u16) -> TcpStream {
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
    }

    fn response_at(port: u16, request: &str) -> (String, Vec<u8>) {
        let mut stream = connect(port);
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let separator = response
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let headers = String::from_utf8(response[..separator].to_vec()).unwrap();
        (headers, response[separator + 4..].to_vec())
    }

    fn accept_client(listener: &Socket) -> Socket {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            match listener.accept() {
                Ok(client) => return client,
                Err(error) if retry_accept(&error) && std::time::Instant::now() < deadline => {}
                Err(error) => panic!("Test client accept failed: {error}"),
            }
        }
    }

    fn exchange(request: &str) -> (String, Vec<u8>) {
        let listener = Socket::bind(0).unwrap();
        let port = listener.port();
        let worker = thread::spawn(move || {
            let client = accept_client(&listener);
            assert_eq!(unsafe { dmmt_server_handle(client.0) }, 0);
        });
        let response = response_at(port, request);
        worker.join().unwrap();
        response
    }

    fn request(method: &str, path: &str) -> String {
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n\r\n")
    }

    fn read_headers(reader: &mut BufReader<TcpStream>) -> String {
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            headers.push_str(&line);
            if line == "\r\n" {
                return headers;
            }
        }
    }

    fn read_event(reader: &mut BufReader<TcpStream>) -> String {
        let mut event = String::new();
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            event.push_str(&line);
            if line == "\n" {
                return event;
            }
        }
    }

    #[test]
    fn serves_index_with_cache_busting_query() {
        let (headers, body) = exchange(&request("GET", "/?t=123"));
        assert!(headers.starts_with("HTTP/1.1 200 OK"));
        assert!(headers.contains("Content-Type: text/html; charset=utf-8"));
        assert!(headers.contains("Cache-Control: no-cache, no-store, must-revalidate"));
        assert!(headers.contains("Connection: close"));
        assert_eq!(body, include_bytes!("../public/index.html"));
    }

    #[test]
    fn serves_binary_tiles_from_both_roots() {
        let content = b"RIFF\0\xff\x80WEBP";
        for root in ["tiles", "tiles_height"] {
            let fixture = Fixture::new(root);
            fs::write(fixture.0.join("0.webp"), content).unwrap();
            let (headers, body) = exchange(&request("GET", &fixture.url("0.webp?t=42")));
            assert!(headers.starts_with("HTTP/1.1 200 OK"));
            assert!(headers.contains("Content-Type: image/webp"));
            assert!(headers.contains(&format!("Content-Length: {}", content.len())));
            assert_eq!(body, content);
        }
    }

    #[test]
    fn serves_encoded_public_paths_and_large_files() {
        let fixture = Fixture::new("public");
        let content: Vec<u8> = (0..100_000).map(|value| (value % 256) as u8).collect();
        fs::write(fixture.0.join("test file.bin"), &content).unwrap();
        let path = format!(
            "/{}/test%20file.bin?cache=1",
            fixture.0.file_name().unwrap().to_str().unwrap()
        );
        let (headers, body) = exchange(&request("GET", &path));
        assert!(headers.starts_with("HTTP/1.1 200 OK"));
        assert!(headers.contains("Content-Type: application/octet-stream"));
        assert_eq!(body, content);
    }

    #[test]
    fn serves_unicode_public_paths() {
        let fixture = Fixture::new("public");
        let content = b"unicode asset\0\xff";
        fs::write(fixture.0.join("café-地图-🗺.bin"), content).unwrap();
        let path = format!(
            "/{}/caf%C3%A9-%E5%9C%B0%E5%9B%BE-%F0%9F%97%BA.bin",
            fixture.0.file_name().unwrap().to_str().unwrap()
        );
        let (headers, body) = exchange(&request("GET", &path));
        assert!(headers.starts_with("HTTP/1.1 200 OK"));
        assert_eq!(body, content);
    }

    #[test]
    fn head_responses_have_headers_without_bodies() {
        let (headers, body) = exchange(&request("HEAD", "/"));
        assert!(headers.starts_with("HTTP/1.1 200 OK"));
        assert!(headers.contains(&format!(
            "Content-Length: {}",
            include_bytes!("../public/index.html").len()
        )));
        assert!(body.is_empty());
        for path in ["/sse", "/dmmt-missing-file"] {
            let (headers, body) = exchange(&request("HEAD", path));
            assert!(headers.contains("Content-Length:"));
            assert!(body.is_empty());
        }
    }

    #[test]
    fn reports_missing_files_and_unsupported_methods() {
        let (headers, _) = exchange(&request("GET", "/dmmt-missing-file"));
        assert!(headers.starts_with("HTTP/1.1 404 Not Found"));
        let (headers, _) = exchange(&request("POST", "/"));
        assert!(headers.starts_with("HTTP/1.1 405 Method Not Allowed"));
        assert!(headers.contains("Allow: GET, HEAD"));
    }

    #[test]
    fn rejects_traversal_and_malformed_paths() {
        for path in [
            "/../Cargo.toml",
            "/%2e%2e/Cargo.toml",
            "/tiles/%2e%2e/Cargo.toml",
            "/tiles_height/../Cargo.toml",
            "/..%5cCargo.toml",
            "/%00",
            "/%3a",
            "/%ZZ",
            "/%",
        ] {
            let (headers, _) = exchange(&request("GET", path));
            assert!(headers.starts_with("HTTP/1.1 400 Bad Request"), "{path}");
        }
    }

    #[test]
    fn rejects_malformed_and_oversized_requests() {
        for request in [
            "GET / HTTP/1.1\r\n\r\n",
            "GET / HTTP/1.1\r\nHost: one\r\nHost: two\r\n\r\n",
            "GET / HTTP/1.9\r\nHost: localhost\r\n\r\n",
            "GET / HTTP/1.1\nHost: localhost\n\n",
        ] {
            let (headers, _) = exchange(request);
            assert!(headers.starts_with("HTTP/1.1 400 Bad Request"));
        }
        let (headers, _) = exchange(&request("GET", &format!("/{}", "x".repeat(4096))));
        assert!(headers.starts_with("HTTP/1.1 414 URI Too Long"));
        let mut oversized = "GET / HTTP/1.1\r\nHost: localhost\r\nX-Padding: ".to_string();
        oversized.push_str(&"x".repeat(8192 - oversized.len()));
        let (headers, _) = exchange(&oversized);
        assert!(headers.starts_with("HTTP/1.1 431 Request Header Fields Too Large"));
    }

    #[test]
    fn supports_http_1_0() {
        let (headers, body) = exchange("GET / HTTP/1.0\r\n\r\n");
        assert!(headers.starts_with("HTTP/1.0 200 OK"));
        assert_eq!(body, include_bytes!("../public/index.html"));
    }

    #[test]
    fn formats_multiline_sse_data() {
        assert_eq!(
            sse_frame("first\r\nsecond\rthird"),
            "data: first\ndata: second\ndata: third\n\n"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn streams_live_events_without_blocking_static_requests() {
        let listener = Socket::bind(0).unwrap();
        let port = listener.port();
        let (events, _) = broadcast::channel(10);
        let server_events = events.clone();
        let runtime = tokio::runtime::Handle::current();
        let server = tokio::task::spawn_blocking(move || {
            let mut clients = Vec::new();
            for _ in 0..2 {
                let client = accept_client(&listener);
                clients.push(runtime.spawn(serve_client(
                    client,
                    server_events.subscribe(),
                    KEEP_ALIVE_INTERVAL,
                )));
            }
            clients
        });
        let mut stream = connect(port);
        stream.write_all(request("GET", "/sse").as_bytes()).unwrap();
        let mut reader = BufReader::new(stream);
        let headers = read_headers(&mut reader);
        assert!(headers.starts_with("HTTP/1.1 200 OK"));
        assert!(headers.contains("Content-Type: text/event-stream"));
        assert!(!headers.contains("Content-Length:"));
        let (headers, body) = response_at(port, &request("GET", "/"));
        assert!(headers.starts_with("HTTP/1.1 200 OK"));
        assert_eq!(body, include_bytes!("../public/index.html"));
        let clients = server.await.unwrap();
        for data in [
            r#"{"type":"tile_update","dim":"overworld","z":0,"x":1,"y":2}"#,
            r#"{"type":"player_update","players":[]}"#,
        ] {
            events.send(data.to_string()).unwrap();
            assert_eq!(read_event(&mut reader), format!("data: {data}\n\n"));
        }
        drop(events);
        let mut remainder = String::new();
        reader.read_to_string(&mut remainder).unwrap();
        assert!(remainder.is_empty());
        for client in clients {
            client.await.unwrap();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sends_sse_keep_alives() {
        let listener = Socket::bind(0).unwrap();
        let port = listener.port();
        let (events, _) = broadcast::channel(10);
        let receiver = events.subscribe();
        let server = tokio::task::spawn_blocking(move || accept_client(&listener));
        let mut stream = connect(port);
        stream.write_all(request("GET", "/sse").as_bytes()).unwrap();
        let client = server.await.unwrap();
        let task = tokio::spawn(serve_client(client, receiver, Duration::from_millis(10)));
        let mut reader = BufReader::new(stream);
        read_headers(&mut reader);
        assert_eq!(read_event(&mut reader), ": keep-alive\n\n");
        drop(events);
        task.await.unwrap();
    }

    #[test]
    fn reports_port_binding_errors() {
        let listener = Socket::bind(0).unwrap();
        assert!(Socket::bind(listener.port()).is_err());
    }

    #[test]
    fn idle_accepts_time_out_and_can_be_retried() {
        let listener = Socket::bind(0).unwrap();
        let error = listener.accept().err().unwrap();
        assert!(retry_accept(&error));
        assert!(retry_accept(&io::Error::from(
            io::ErrorKind::ConnectionAborted
        )));
        assert!(retry_accept(&io::Error::from(
            io::ErrorKind::ConnectionReset
        )));
        assert!(!retry_accept(&io::Error::from(io::ErrorKind::InvalidInput)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancellation_releases_an_idle_listener() {
        let listener = Socket::bind(0).unwrap();
        let port = listener.port();
        let (events, _) = broadcast::channel(10);
        let server = tokio::spawn(serve_listener(listener, events, MAX_CLIENTS));
        tokio::time::sleep(Duration::from_millis(20)).await;
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(2), async move {
            loop {
                if let Ok(listener) = Socket::bind(port) {
                    drop(listener);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancellation_closes_sse_when_client_slots_are_full() {
        let listener = Socket::bind(0).unwrap();
        let port = listener.port();
        let (events, _) = broadcast::channel(10);
        let server = tokio::spawn(serve_listener(listener, events, 1));
        let mut stream = connect(port);
        stream.write_all(request("GET", "/sse").as_bytes()).unwrap();
        let mut reader = BufReader::new(stream);
        assert!(read_headers(&mut reader).contains("Content-Type: text/event-stream"));
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
        let mut remainder = String::new();
        reader.read_to_string(&mut remainder).unwrap();
        assert!(remainder.is_empty());
    }
}
