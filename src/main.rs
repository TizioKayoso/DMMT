use fastnbt::LongArray;
use flate2::read::ZlibDecoder;
use image::imageops::{FilterType, resize};
use image::{Rgb, RgbImage};
use serde::Deserialize;
use std::io::Read;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::mpsc;

mod block_to_rgb;
use crate::block_to_rgb::block_to_rgb;

struct ProcessedChunk {
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
    data: Option<LongArray>,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open("r.0.0.mca").await?;
    let mut header = [0u8; 4096];
    file.read_exact(&mut header).await?;

    println!("Header loaded. Spawning async reader and parsers...");

    let (tx, mut rx) = mpsc::channel::<ProcessedChunk>(64);

    tokio::spawn(async move {
        for cz in 0..32 {
            for cx in 0..32 {
                let header_offset = ((cz * 32) + cx) * 4;
                let offset_bytes = [
                    0,
                    header[header_offset],
                    header[header_offset + 1],
                    header[header_offset + 2],
                ];
                let sector_offset = u32::from_be_bytes(offset_bytes);
                if sector_offset == 0 {
                    continue; // Chunk vuoto
                }

                if file
                    .seek(std::io::SeekFrom::Start((sector_offset as u64) * 4096))
                    .await
                    .is_err()
                {
                    continue;
                }

                let mut length_bytes = [0u8; 4];
                if file.read_exact(&mut length_bytes).await.is_err() {
                    continue;
                }
                let chunk_length = u32::from_be_bytes(length_bytes);

                let mut compression_type = [0u8; 1];
                if file.read_exact(&mut compression_type).await.is_err() {
                    continue;
                }

                let mut compressed_bytes = vec![0u8; (chunk_length - 1) as usize];
                if file.read_exact(&mut compressed_bytes).await.is_err() {
                    continue;
                }

                let tx_clone = tx.clone();

                tokio::task::spawn_blocking(move || {
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
                                                            || block_name
                                                                == "minecraft:bubble_column"
                                                        {
                                                            is_water = true;
                                                        } else {
                                                            break 'column;
                                                        }
                                                    }

                                                    if is_water {
                                                        if block_name == "minecraft:water"
                                                            || block_name
                                                                == "minecraft:bubble_column"
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
                                            water_depths[z][x] =
                                                if is_water { depth.max(1) } else { 0 };
                                        }
                                    }
                                }

                                let _ = tx_clone.blocking_send(ProcessedChunk {
                                    cx,
                                    cz,
                                    colors,
                                    world_heights,
                                    water_depths,
                                });
                            }
                        }
                    }
                });
            }
        }
    });

    let mut color_img = RgbImage::from_pixel(512, 512, Rgb([30, 30, 30]));
    let mut world_y_grid = vec![vec![-64i16; 512]; 512];
    let mut water_depth_grid = vec![vec![0i16; 512]; 512];

    println!("Waiting for chunks... Creating color image live!");

    while let Some(chunk) = rx.recv().await {
        for z in 0..16 {
            for x in 0..16 {
                let pixel_x = chunk.cx * 16 + x;
                let pixel_z = chunk.cz * 16 + z;

                world_y_grid[pixel_z][pixel_x] = chunk.world_heights[z][x];
                water_depth_grid[pixel_z][pixel_x] = chunk.water_depths[z][x];

                color_img.put_pixel(pixel_x as u32, pixel_z as u32, Rgb(chunk.colors[z][x]));
            }
        }
    }

    println!("All chunks processed. Assembling the height image...");

    let mut min_land_y = i16::MAX;
    let mut max_land_y = i16::MIN;

    for z in 0..512 {
        for x in 0..512 {
            if water_depth_grid[z][x] == 0 {
                let y = world_y_grid[z][x];
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
    let mut height_img = RgbImage::from_pixel(512, 512, Rgb([34, 168, 76]));

    for z in 0..512 {
        for x in 0..512 {
            let depth = water_depth_grid[z][x];
            if depth > 0 {
                let blue_rgb = depth_to_water_color(depth);
                height_img.put_pixel(x as u32, z as u32, Rgb(blue_rgb));
            } else {
                let current_y = world_y_grid[z][x];
                let diff_from_lowest = (current_y - min_land_y).max(0);

                let land_rgb = relative_height_to_color(diff_from_lowest, max_diff);
                height_img.put_pixel(x as u32, z as u32, Rgb(land_rgb));
            }
        }
    }

    println!("Spawning parallel image scaling tasks...");

    let color_task = tokio::task::spawn_blocking(move || {
        let scaled = resize(&color_img, 2048, 2048, FilterType::Nearest);
        scaled.save("region_color_0_0.png").unwrap();
        println!("[Thread 1] Saved 'region_color_0_0.png'");
    });

    let height_task = tokio::task::spawn_blocking(move || {
        let scaled = resize(&height_img, 2048, 2048, FilterType::Nearest);
        scaled.save("region_height_0_0.png").unwrap();
        println!("[Thread 2] Saved 'region_height_0_0.png'");
    });

    let _ = tokio::join!(color_task, height_task);

    println!("All tasks finished!");
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

