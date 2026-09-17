# Dynamic Minecraft Mapping Tool (DMMT)

> [!WARNING]
> **Work in Progress / Early Development**
> This repository is currently in early development. Features, internal APIs, and configurations may undergo significant changes as development progresses.

**Dynamic Minecraft Mapping Tool (DMMT)** is a real-time, high-performance web-based map renderer for Minecraft worlds written in Rust and HTML/JavaScript. It parses Minecraft Anvil region files (`.mca`) directly from a server world directory, renders tile pyramids as `.webp` images, tracks player locations, and streams real-time updates to an interactive Leaflet frontend using Server-Sent Events (SSE).
DMMT is meant to be a performance focused alternative to other mapping tools, offering low options in exchange.

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
  - **[Axum](https://github.com/tokio-rs/axum):** Web framework for handling HTTP routes, static file serving, and SSE streaming.
  - **[Tokio](https://tokio.rs/):** Async runtime powering background watchers and event broadcasting.
  - **[Rayon](https://github.com/rayon-rs/rayon):** Parallel chunk processing and image scaling.
  - **[fastnbt](https://github.com/owencaamp/fastnbt) & [flate2](https://github.com/rust-lang/flate2-rs):** NBT parsing and Zlib/Gzip decompression for region and player files.
  - **[image](https://github.com/image-rs/image):** Image buffer operations and WebP tile saving.
- **Frontend:**
  - **[Leaflet.js](https://leafletjs.com/):** Interactive web map rendering using custom simple coordinate systems (`L.CRS.Simple`).
  - **Server-Sent Events (SSE):** Event stream subscription for real-time tile cache-busting and marker updates[cite: 3].

---
