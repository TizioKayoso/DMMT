# Dynamic Minecraft Mapping Tool (DMMT)

> [!WARNING]
> **Work in Progress / Early Development**
> This repository is currently in early development. Features, internal APIs, and configurations may undergo significant changes as development progresses.

**Dynamic Minecraft Mapping Tool (DMMT)** is a real-time, high-performance web-based map renderer for Minecraft worlds written in Rust, C, and HTML/JavaScript. It parses Minecraft Anvil region files (`.mca`) directly from a server world directory, renders tile pyramids as `.webp` images, tracks player locations, and streams real-time updates to an interactive Leaflet frontend using Server-Sent Events (SSE).
DMMT is meant to be a performance-focused alternative to other mapping tools, offering fewer configuration options in exchange.

---

## Features

- **Multithreaded Region Parsing:** Utilizes `rayon`, `fastnbt`, and `flate2` to decompress and parse Anvil `.mca` files in parallel across available CPU cores.
- **Dual Map Views:** Renders two map styles:
  - **Color Map:** Top-down block rendering mapped to realistic RGB colors[cite: 1, 2, 3].
  - **Height Map:** Relative terrain topography and banded ocean depth rendering[cite: 2, 3].
- **Live World File Monitoring:** Continuously watches the `minecraft/` world folder for `.mca` file updates and automatically re-renders modified regions.
- **Real-Time Player Tracking:** Reads player position data from `playerdata/*.dat` files and resolves UUIDs against `usercache.json` to display live player markers on the map[cite: 2, 3].
- **Multi-Dimension Support:** Supports rendering and switching between Overworld, The Nether (with Y-ceiling height filtering above Y=85), and The End[cite: 2, 3].
- **Pyramid Tile Generation:** Builds multi-zoom WebP tile pyramids (`tiles/` and `tiles_height/`) for web map navigation.
- **Live SSE Frontend Updates:** Uses Server-Sent Events to push tile refresh notifications (`tile_update`) and player coordinate updates (`player_update`) without reloading the page[cite: 2, 3].
- **Customizable Block Colors:** Loads custom block-to-RGB mappings from an optional `blocks.json` file, falling back to a missing texture color (`[255, 0, 255]`) for unmapped blocks.

---

## Tech Stack

- **Backend (Rust):**
  - **Native C HTTP Server:** `src/server.c` handles sockets, HTTP requests, and static files, linked into the Rust executable through `build.rs`. `src/server.rs` bridges live SSE events to connected clients without Axum or Tower HTTP.
  - **[Tokio](https://tokio.rs/):** Async runtime powering background watchers, event broadcasting, and SSE forwarding.
  - **[Rayon](https://github.com/rayon-rs/rayon):** Parallel chunk processing and image scaling.
  - **[fastnbt](https://github.com/owencaamp/fastnbt) & [flate2](https://github.com/rust-lang/flate2-rs):** NBT parsing and Zlib/Gzip decompression for region and player files.
  - **[image](https://github.com/image-rs/image):** Image buffer operations and WebP tile saving.
- **Frontend:**
  - **[Leaflet.js](https://leafletjs.com/):** Interactive web map rendering using custom simple coordinate systems (`L.CRS.Simple`).
  - **Server-Sent Events (SSE):** Event stream subscription for real-time tile cache-busting and marker updates[cite: 3].

---


## Repository Structure

```text
.
├── build.rs
├── src/
│   ├── block_to_rgb.rs
│   ├── main.rs
│   ├── server.c
│   └── server.rs
├── public/
│   └── index.html
└── blocks.json
```

## Building and Running

Build with `cargo build --release` and run with `cargo run --release` from the repository root. A C compiler is required in addition to the Rust toolchain: GCC or Clang on Unix, or the matching MSVC/MinGW compiler on Windows. Cargo compiles and links `src/server.c` automatically; Windows builds also link Winsock (`ws2_32`).

The server listens on `0.0.0.0:8080`, serving the frontend from `public/`, map tiles from `/tiles/` and `/tiles_height/`, and live events from `/sse`. The asset directories are relative to the working directory and should contain only trusted files and links. It supports GET and HEAD, disables caching, limits active connections to 128, and sends SSE keep-alives every 15 seconds. Responses are sent without gzip compression.

Run `cargo test` to check static files, binary tiles, HTTP errors, path validation, and live SSE delivery.

