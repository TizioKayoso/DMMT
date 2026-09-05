use axum::{
    Router,
    extract::State,
    http::{HeaderValue, header::CACHE_CONTROL},
    response::sse::{Event, Sse},
    routing::get,
};
use flate2::read::ZlibDecoder;
use futures::stream::{Stream, StreamExt};
use image::{Rgb, RgbImage};
use rayon::prelude::*;
use serde::Deserialize;
use std::convert::Infallible;
use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
};

mod block_to_rgb;
use crate::block_to_rgb::{Block, block_to_rgb, parse_block_name};

struct AppState {
    tx: broadcast::Sender<String>,
}
struct ProcessedChunk {
    reg_x: i32,
    reg_z: i32,
    cx: usize,
    cz: usize,
    colors: [[[u8; 3]; 16]; 16],
    world_heights: [[i16; 16]; 16],
    water_depths: [[i16; 16]; 16],
}

#[derive(Deserialize, Debug)]
struct ChunkNBT {
    #[serde(rename = "DataVersion")]
    _data_version: i32,
    sections: Option<Vec<ChunkSection>>,
}

#[derive(Deserialize, Debug)]
struct ChunkSection {
    #[serde(rename = "Y")]
    y: i8,
    block_states: Option<Blockstates>,
}

#[derive(Deserialize, Debug)]
struct Blockstates {
    palette: Vec<BlockState>,
    data: Option<fastnbt::LongArray>,
}

#[derive(Deserialize, Debug)]
struct BlockState {
    #[serde(rename = "Name")]
    name: String,
}

struct ParsedSection<'a> {
    y: i8,
    palette: Vec<Block>,
    data: Option<&'a fastnbt::LongArray>,
}

fn depth_to_water_color(depth: i16) -> [u8; 3] {
    let band_size = 3;
    let banded_depth = ((depth - 1) / band_size) * band_size + 1;
    let d = banded_depth.clamp(1, 30) as f32;
    let t = (d - 1.0) / 29.0;

    let r = (90.0 - t * 80.0) as u8;
    let g = (200.0 - t * 160.0) as u8;
    let b = (245.0 - t * 135.0) as u8;
    [r, g, b]
}

fn relative_height_to_color(diff: i16, max_diff: i16) -> [u8; 3] {
    let range = max_diff.max(1) as f32;
    let t = (diff as f32 / range).clamp(0.0, 1.0);

    if t < 0.1 {
        let f = t / 0.1;
        let r = (30.0 + f * (50.0 - 30.0)) as u8;
        let g = (30.0 + f * (70.0 - 30.0)) as u8;
        let b = (30.0 + f * (40.0 - 30.0)) as u8;
        [r, g, b]
    } else if t < 0.5 {
        let f = (t - 0.1) / 0.4;
        let r = (50.0 + f * (90.0 - 50.0)) as u8;
        let g = (70.0 + f * (140.0 - 70.0)) as u8;
        let b = (40.0 + f * (60.0 - 40.0)) as u8;
        [r, g, b]
    } else {
        let f = (t - 0.5) / 0.5;
        let r = (90.0 + f * (220.0 - 90.0)) as u8;
        let g = (140.0 + f * (220.0 - 140.0)) as u8;
        let b = (60.0 + f * (220.0 - 60.0)) as u8;
        [r, g, b]
    }
}

fn find_mca_files(dir: &Path, files: &mut Vec<(i32, i32, PathBuf)>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_mca_files(&path, files);
            } else if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("r.") && filename.ends_with(".mca") {
                    let parts: Vec<&str> = filename.split('.').collect();
                    if parts.len() == 4 {
                        if let (Ok(rx), Ok(rz)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>())
                        {
                            files.push((rx, rz, path));
                        }
                    }
                }
            }
        }
    }
}

fn get_block_at(section: &ParsedSection, x: usize, y: usize, z: usize) -> Block {
    if section.palette.is_empty() {
        return Block::Air;
    }

    let data = match section.data {
        Some(d) => d,
        None => return section.palette[0],
    };

    let bits_per_block = std::cmp::max(4, (section.palette.len() as f32).log2().ceil() as usize);
    let blocks_per_entry = 64 / bits_per_block;
    let block_index = (y * 256) + (z * 16) + x;
    let entry_index = block_index / blocks_per_entry;
    let bit_offset = (block_index % blocks_per_entry) * bits_per_block;
    let mask = (1usize << bits_per_block) - 1;

    if entry_index >= data.len() {
        return Block::Air;
    }

    let palette_index = ((data[entry_index] as u64) >> bit_offset) as usize & mask;
    section
        .palette
        .get(palette_index)
        .copied()
        .unwrap_or(Block::Air)
}

fn process_region(
    dim_name: &str,
    dim_dir: &Path,
    render_tx: &broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = Instant::now();
    let mut header = [0u8; 4096];
    if file.read_exact(&mut header).is_err() {
        return chunks;
    }

    for cz in 0..32 {
        for cx in 0..32 {
            let header_offset = ((cz * 32) + cx) * 4;
            let sector_offset = u32::from_be_bytes([
                0,
                header[header_offset],
                header[header_offset + 1],
                header[header_offset + 2],
            ]);
            if sector_offset == 0 {
                continue;
            }

            if file
                .seek(SeekFrom::Start((sector_offset as u64) * 4096))
                .is_err()
            {
                continue;
            }

            let mut length_bytes = [0u8; 4];
            if file.read_exact(&mut length_bytes).is_err() {
                continue;
            }
            let chunk_length = u32::from_be_bytes(length_bytes);

            let mut compression_type = [0u8; 1];
            if file.read_exact(&mut compression_type).is_err() {
                continue;
            }

            let mut compressed_bytes = vec![0u8; (chunk_length - 1) as usize];
            if file.read_exact(&mut compressed_bytes).is_err() {
                continue;
            }

            let mut decoder = ZlibDecoder::new(&compressed_bytes[..]);
            let mut decompressed_nbt = Vec::new();

            if decoder.read_to_end(&mut decompressed_nbt).is_ok() {
                if let Ok(chunk) = fastnbt::from_bytes::<ChunkNBT>(&decompressed_nbt) {
                    if let Some(mut sections) = chunk.sections {
                        sections.sort_by(|a, b| b.y.cmp(&a.y));

                        let parsed_sections: Vec<ParsedSection> = sections
                            .iter()
                            .map(|s| {
                                let palette = s
                                    .block_states
                                    .as_ref()
                                    .map(|bs| {
                                        bs.palette
                                            .iter()
                                            .map(|b| parse_block_name(&b.name))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                let data = s.block_states.as_ref().and_then(|bs| bs.data.as_ref());
                                ParsedSection {
                                    y: s.y,
                                    palette,
                                    data,
                                }
                            })
                            .collect();

                        let mut colors = [[[30u8, 30, 30]; 16]; 16];
                        let mut world_heights = [[-64i16; 16]; 16];
                        let mut water_depths = [[0i16; 16]; 16];

                        for x in 0..16 {
                            for z in 0..16 {
                                let mut top_y = None;
                                let mut is_water = false;
                                let mut depth = 0i16;

                                'column: for section in &parsed_sections {
                                    for y_rel in (0..16).rev() {
                                        let world_y = (section.y as i16 * 16) + y_rel as i16;
                                        if is_nether && world_y > 85 {
                                            continue;
                                        }

                                        let block = get_block_at(section, x, y_rel, z);
                                        if block != Block::Air {
                                            if top_y.is_none() {
                                                top_y = Some(world_y);
                                                colors[z][x] = block_to_rgb(block);
                                                if block == Block::Water
                                                    || block == Block::BubbleColumn
                                                {
                                                    is_water = true;
                                                } else {
                                                    break 'column;
                                                }
                                            }

                                            if is_water {
                                                if block == Block::Water
                                                    || block == Block::BubbleColumn
                                                {
                                                    depth += 1;
                                                } else {
                                                    break 'column;
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(y) = top_y {
                                    world_heights[z][x] = y;
                                    water_depths[z][x] = if is_water { depth.max(1) } else { 0 };
                                }
                            }
                        }

                        chunks.push(ProcessedChunk {
                            reg_x: rx_coord,
                            reg_z: rz_coord,
                            cx,
                            cz,
                            colors,
                            world_heights,
                            water_depths,
                        });
                    }
                }
            }
        }
    }
    chunks
}

fn generate_tile_pyramid(
    img: &RgbImage,
    base_folder: &Path,
    max_zoom_out: i32,
    render_tx: &broadcast::Sender<String>,
    dim_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tile_size = 256u32;

    for zoom in (max_zoom_out..=0).rev() {
        let scale_factor = 1 << (-zoom as u32);
        let target_w = (img.width() / scale_factor).max(1);
        let target_h = (img.height() / scale_factor).max(1);

        let resized_img = if zoom == 0 {
            img.clone()
        } else {
            image::imageops::resize(
                img,
                target_w,
                target_h,
                image::imageops::FilterType::Nearest,
            )
        };

        let tiles_x = (target_w as f32 / tile_size as f32).ceil() as u32;
        let tiles_z = (target_h as f32 / tile_size as f32).ceil() as u32;

        let zoom_out_dir = base_folder.join(zoom.to_string());
        for tx in 0..tiles_x {
            std::fs::create_dir_all(zoom_out_dir.join(tx.to_string()))?;
        }

        let mut tile_coords = Vec::new();
        for tz in 0..tiles_z {
            for tx in 0..tiles_x {
                tile_coords.push((tx, tz));
            }
        }

        tile_coords.par_iter().for_each(|&(tx, tz)| {
            let mut tile = RgbImage::new(tile_size, tile_size);
            let start_x = tx * tile_size;
            let start_z = tz * tile_size;
            let mut is_empty = true;

            for z in 0..tile_size {
                for x in 0..tile_size {
                    let global_x = start_x + x;
                    let global_z = start_z + z;

                    if global_x < target_w && global_z < target_h {
                        let pixel = resized_img.get_pixel(global_x, global_z);
                        tile.put_pixel(x, z, *pixel);
                        if pixel.0 != [30, 30, 30] {
                            is_empty = false;
                        }
                    } else {
                        tile.put_pixel(x, z, Rgb([30, 30, 30]));
                    }
                }
            }

            if !is_empty {
                let tile_path = zoom_out_dir
                    .join(tx.to_string())
                    .join(format!("{}.webp", tz));
                tile.save(tile_path).unwrap();
                let update_msg = format!(
                    r#"{{\"dim\": \"overworld\", \"z\": {}, \"x\": {}, \"y\": {}}}"#,
                    dim_name, zoom, tx, tz
                );
                let _ = render_tx.send(update_msg);
            }
        });
    }
    Ok(())
}

fn process_dimension(
    dim_name: &str,
    dim_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = Instant::now();
    let mut region_files = Vec::new();

    find_mca_files(dim_dir, &mut region_files);
    if region_files.is_empty() {
        return Ok(());
    }

    println!("Processing dimension: '{}'", dim_name);

    let min_rx = region_files.iter().map(|&(x, _, _)| x).min().unwrap();
    let max_rx = region_files.iter().map(|&(x, _, _)| x).max().unwrap();
    let min_rz = region_files.iter().map(|&(_, z, _)| z).min().unwrap();
    let max_rz = region_files.iter().map(|&(_, z, _)| z).max().unwrap();

    let total_width = ((max_rx - min_rx + 1) as usize) * 512;
    let total_height = ((max_rz - min_rz + 1) as usize) * 512;
    let is_nether = dim_name == "the_nether";
    let processed_chunks: Vec<ProcessedChunk> = region_files
        .into_par_iter()
        .flat_map(|(rx, rz, path)| process_region(rx, rz, &path, is_nether))
        .collect();

    let mut color_img =
        RgbImage::from_pixel(total_width as u32, total_height as u32, Rgb([30, 30, 30]));
    let mut world_y_grid = vec![-64i16; total_width * total_height];
    let mut water_depth_grid = vec![0i16; total_width * total_height];
    let mut valid_grid = vec![false; total_width * total_height];

    for chunk in processed_chunks {
        let reg_offset_x = ((chunk.reg_x - min_rx) as usize) * 512;
        let reg_offset_z = ((chunk.reg_z - min_rz) as usize) * 512;

        for z in 0..16 {
            for x in 0..16 {
                let global_x = reg_offset_x + (chunk.cx * 16 + x);
                let global_z = reg_offset_z + (chunk.cz * 16 + z);
                let idx = global_z * total_width + global_x;

                world_y_grid[idx] = chunk.world_heights[z][x];
                water_depth_grid[idx] = chunk.water_depths[z][x];
                valid_grid[idx] = true;
                color_img.put_pixel(global_x as u32, global_z as u32, Rgb(chunk.colors[z][x]));
            }
        }
    }

    let (min_land_y, max_land_y) = (0..total_height)
        .into_par_iter()
        .fold(
            || (i16::MAX, i16::MIN),
            |(mut min_y, mut max_y), z| {
                let row_offset = z * total_width;
                for x in 0..total_width {
                    let idx = row_offset + x;
                    if valid_grid[idx] && water_depth_grid[idx] == 0 {
                        let y = world_y_grid[idx];
                        if y < min_y {
                            min_y = y;
                        }
                        if y > max_y {
                            max_y = y;
                        }
                    }
                }
                (min_y, max_y)
            },
        )
        .reduce(
            || (i16::MAX, i16::MIN),
            |(min_a, max_a), (min_b, max_b)| (min_a.min(min_b), max_a.max(max_b)),
        );

    let min_land_y = if min_land_y == i16::MAX {
        -64
    } else {
        min_land_y
    };
    let max_land_y = if max_land_y == i16::MIN {
        320
    } else {
        max_land_y
    };
    let max_diff = (max_land_y - min_land_y).max(1);

    let mut height_img =
        RgbImage::from_pixel(total_width as u32, total_height as u32, Rgb([30, 30, 30]));

    let height_pixels: Vec<Vec<[u8; 3]>> = (0..total_height)
        .into_par_iter()
        .map(|z| {
            let row_offset = z * total_width;
            let mut row_colors = vec![[30, 30, 30]; total_width];
            for x in 0..total_width {
                let idx = row_offset + x;
                if valid_grid[idx] {
                    let depth = water_depth_grid[idx];
                    if depth > 0 {
                        row_colors[x] = depth_to_water_color(depth);
                    } else {
                        let current_y = world_y_grid[idx];
                        let diff_from_lowest = (current_y - min_land_y).max(0);
                        row_colors[x] = relative_height_to_color(diff_from_lowest, max_diff);
                    }
                }
            }
            row_colors
        })
        .collect();

    for (z, row) in height_pixels.into_iter().enumerate() {
        for (x, color) in row.into_iter().enumerate() {
            height_img.put_pixel(x as u32, z as u32, Rgb(color));
        }
    }

    let color_out = Path::new("tiles").join(dim_name);
    let height_out = Path::new("tiles_height").join(dim_name);

    generate_tile_pyramid(&color_img, &color_out, -3, render_tx, dim_name)?;
    generate_tile_pyramid(&height_img, &height_out, -3, render_tx, dim_name)?;

    println!("Finished '{}' in {:.2?}", dim_name, start_time.elapsed());
    Ok(())
}

fn run_generator(
    render_tx: &broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let base_dir = if Path::new("minecraft").exists() {
        Path::new("minecraft")
    } else if Path::new("region").exists() {
        Path::new("region")
    } else {
        println!("Waiting for world directory ('minecraft' or 'region') to be mounted...");
        return Ok(());
    };

    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dim_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dim_name == "creative_world" || dim_name.starts_with('.') {
                        continue;
                    }
                    if let Err(e) = process_dimension(dim_name, &path, render_tx) {
                        eprintln!("Error processing dimension '{}': {}", dim_name, e);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(data) => Some(Ok(Event::default().data(data))),
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let num_threads = env::var("MAX_THREADS")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<usize>()
        .unwrap_or(0);
    println!(
        "Initialing thread pool with {} threads (0 = all avaiable cores)",
        num_threads
    );
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();
    let (tx, _rx) = broadcast::channel::<String>(100);
    let app_state = Arc::new(AppState { tx: tx.clone() });
    let render_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            println!("Starting map generation cycle...");
            if let Err(e) = run_generator(&render_tx) {
                eprintln!("Error during map generation: {}", e);
            }
            println!("Sleeping for 30 minutes...");
            tokio::time::sleep(Duration::from_secs(30 * 60)).await;
        }
    });

    let app = Router::new()
        .route("/sse", get(sse_handler))
        .nest_service("/tiles", ServeDir::new("tiles"))
        .nest_service("/tiles_height", ServeDir::new("tiles_height"))
        .fallback_service(ServeDir::new("public"))
        .layer(CompressionLayer::new())
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=604800"),
        ))
        .with_state(app_state);

    println!("Starting web server on http://localhost:8080");
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
