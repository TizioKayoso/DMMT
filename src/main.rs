use flate2::read::ZlibDecoder;
use image::{Rgb, RgbImage};
use rayon::prelude::*;
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;

mod block_to_rgb;
use crate::block_to_rgb::block_to_rgb;

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
    data_version: i32,
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
    if diff <= 5 {
        return [34, 168, 76];
    }

    let band_size = 5;
    let banded_diff = 5 + ((diff - 5) / band_size) * band_size;
    let range = (max_diff - 5).max(1) as f32;
    let t = ((banded_diff - 5) as f32 / range).clamp(0.0, 1.0);

    if t < 0.33 {
        let factor = t / 0.33;
        let r = (34.0 + factor * (212.0 - 34.0)) as u8;
        let g = (168.0 + factor * (201.0 - 168.0)) as u8;
        let b = (76.0 + factor * (116.0 - 76.0)) as u8;
        [r, g, b]
    } else if t < 0.66 {
        let factor = (t - 0.33) / 0.33;
        let r = (212.0 + factor * (143.0 - 212.0)) as u8;
        let g = (201.0 + factor * (89.0 - 201.0)) as u8;
        let b = (116.0 + factor * (43.0 - 116.0)) as u8;
        [r, g, b]
    } else {
        let factor = (t - 0.66) / 0.34;
        let r = (143.0 + factor * (245.0 - 143.0)) as u8;
        let g = (89.0 + factor * (245.0 - 89.0)) as u8;
        let b = (43.0 + factor * (245.0 - 43.0)) as u8;
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

fn process_region(rx_coord: i32, rz_coord: i32, path: &Path) -> Vec<ProcessedChunk> {
    let mut chunks = Vec::new();
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return chunks,
    };

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

                        let mut colors = [[[30u8, 30, 30]; 16]; 16];
                        let mut world_heights = [[-64i16; 16]; 16];
                        let mut water_depths = [[0i16; 16]; 16];

                        for x in 0..16 {
                            for z in 0..16 {
                                let mut top_y = None;
                                let mut is_water = false;
                                let mut depth = 0i16;

                                'column: for section in &sections {
                                    for y_rel in (0..16).rev() {
                                        let block_name = get_block_at(section, x, y_rel, z);
                                        if block_name != "minecraft:air"
                                            && block_name != "minecraft:cave_air"
                                            && block_name != "minecraft:void_air"
                                        {
                                            if top_y.is_none() {
                                                let world_y =
                                                    (section.y as i16 * 16) + y_rel as i16;
                                                top_y = Some(world_y);
                                                colors[z][x] = block_to_rgb(block_name);
                                                if block_name == "minecraft:water"
                                                    || block_name == "minecraft:bubble_column"
                                                {
                                                    is_water = true;
                                                } else {
                                                    break 'column;
                                                }
                                            }

                                            if is_water {
                                                if block_name == "minecraft:water"
                                                    || block_name == "minecraft:bubble_column"
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    let mut region_files = Vec::new();
    find_mca_files(Path::new("."), &mut region_files);

    if region_files.is_empty() {
        println!("Nessun file region (.mca) trovato nelle sottocartelle!");
        return Ok(());
    }

    let min_rx = region_files.iter().map(|&(x, _, _)| x).min().unwrap();
    let max_rx = region_files.iter().map(|&(x, _, _)| x).max().unwrap();
    let min_rz = region_files.iter().map(|&(_, z, _)| z).min().unwrap();
    let max_rz = region_files.iter().map(|&(_, z, _)| z).max().unwrap();

    let regions_width = (max_rx - min_rx + 1) as usize;
    let regions_height = (max_rz - min_rz + 1) as usize;

    let total_width = regions_width * 512;
    let total_height = regions_height * 512;

    println!(
        "Trovati {} file region. Mappa globale dimensionata a {}x{} blocchi.",
        region_files.len(),
        total_width,
        total_height
    );

    // Process all regions in parallel using Rayon work-stealing thread pool
    let processed_chunks: Vec<ProcessedChunk> = region_files
        .into_par_iter()
        .flat_map(|(rx, rz, path)| process_region(rx, rz, &path))
        .collect();

    let mut color_img =
        RgbImage::from_pixel(total_width as u32, total_height as u32, Rgb([30, 30, 30]));

    // Flat 1D vectors for cache locality instead of Vec<Vec<T>>
    let mut world_y_grid = vec![-64i16; total_width * total_height];
    let mut water_depth_grid = vec![0i16; total_width * total_height];

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

                color_img.put_pixel(global_x as u32, global_z as u32, Rgb(chunk.colors[z][x]));
            }
        }
    }

    let mut min_land_y = i16::MAX;
    let mut max_land_y = i16::MIN;

    for z in 0..total_height {
        let row_offset = z * total_width;
        for x in 0..total_width {
            let idx = row_offset + x;
            if water_depth_grid[idx] == 0 {
                let y = world_y_grid[idx];
                if y < min_land_y {
                    min_land_y = y;
                }
                if y > max_land_y {
                    max_land_y = y;
                }
            }
        }
    }

    if min_land_y == i16::MAX {
        min_land_y = -64;
    }
    if max_land_y == i16::MIN {
        max_land_y = 320;
    }

    let max_diff = (max_land_y - min_land_y).max(1);
    let mut height_img =
        RgbImage::from_pixel(total_width as u32, total_height as u32, Rgb([34, 168, 76]));

    for z in 0..total_height {
        let row_offset = z * total_width;
        for x in 0..total_width {
            let idx = row_offset + x;
            let depth = water_depth_grid[idx];
            if depth > 0 {
                let blue_rgb = depth_to_water_color(depth);
                height_img.put_pixel(x as u32, z as u32, Rgb(blue_rgb));
            } else {
                let current_y = world_y_grid[idx];
                let diff_from_lowest = (current_y - min_land_y).max(0);
                let land_rgb = relative_height_to_color(diff_from_lowest, max_diff);
                height_img.put_pixel(x as u32, z as u32, Rgb(land_rgb));
            }
        }
    }

    println!("Slicing map into 256x256 web tiles...");
    let tile_size = 256;
    let tiles_x = (total_width as f32 / tile_size as f32).ceil() as u32;
    let tiles_z = (total_height as f32 / tile_size as f32).ceil() as u32;

    // We will save these under tiles/0/ (where 0 is our base zoom level)
    let base_out_dir = Path::new("tiles").join("0");

    // 1. Pre-create the X-coordinate directory structure so our parallel threads don't trip over each other
    for tx in 0..tiles_x {
        std::fs::create_dir_all(base_out_dir.join(tx.to_string()))?;
    }

    // 2. Create a flat list of all the tile coordinates we need to generate
    let mut tile_coords = Vec::new();
    for tz in 0..tiles_z {
        for tx in 0..tiles_x {
            tile_coords.push((tx, tz));
        }
    }

    // 3. Process and save all tiles in parallel using Rayon!
    tile_coords.par_iter().for_each(|&(tx, tz)| {
        let mut tile = RgbImage::new(tile_size, tile_size);
        let start_x = tx * tile_size;
        let start_z = tz * tile_size;

        let mut is_empty = true; // Optimization flag

        for z in 0..tile_size {
            for x in 0..tile_size {
                let global_x = start_x + x;
                let global_z = start_z + z;

                // Ensure we don't read out of bounds of our global map
                if global_x < total_width as u32 && global_z < total_height as u32 {
                    let pixel = color_img.get_pixel(global_x, global_z);
                    tile.put_pixel(x, z, *pixel);

                    // If the pixel isn't our dark background color, the tile isn't empty
                    if pixel.0 != [30, 30, 30] {
                        is_empty = false;
                    }
                } else {
                    // Out of bounds (edges of the map) get the background color
                    tile.put_pixel(x, z, Rgb([30, 30, 30]));
                }
            }
        }

        // 4. Only save the tile to disk if it actually contains map data
        if !is_empty {
            // Saves in standard web map format: tiles/zoom/x/z.png
            let tile_path = base_out_dir
                .join(tx.to_string())
                .join(format!("{}.png", tz));
            tile.save(tile_path).unwrap();
        }
    });

    println!("Finished generating web tiles!");

    let elapsed = start_time.elapsed();
    println!("Execution finished in {:.2?}", elapsed);

    Ok(())
}

fn get_block_at<'a>(section: &'a ChunkSection, x: usize, y: usize, z: usize) -> &'a str {
    let block_states = match &section.block_states {
        Some(bs) => bs,
        None => return "minecraft:air",
    };

    let palette = &block_states.palette;
    if palette.is_empty() {
        return "minecraft:air";
    }

    let data = match &block_states.data {
        Some(d) => d,
        None => return &palette[0].name,
    };

    let bits_per_block = std::cmp::max(4, (palette.len() as f32).log2().ceil() as usize);
    let blocks_per_entry = 64 / bits_per_block;
    let block_index = (y * 256) + (z * 16) + x;
    let entry_index = block_index / blocks_per_entry;
    let bit_offset = (block_index % blocks_per_entry) * bits_per_block;
    let mask = (1usize << bits_per_block) - 1;

    if entry_index >= data.len() {
        return "minecraft:air";
    }

    let palette_index = ((data[entry_index] as u64) >> bit_offset) as usize & mask;
    palette
        .get(palette_index)
        .map(|b| b.name.as_str())
        .unwrap_or("minecraft:air")
}
