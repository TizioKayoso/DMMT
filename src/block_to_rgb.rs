use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    Air,
    Water,
    BubbleColumn,
    Color([u8; 3]),
}

static BLOCK_MAP: OnceLock<HashMap<String, [u8; 3]>> = OnceLock::new();
static MISSING_BLOCKS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn get_block_map() -> &'static HashMap<String, [u8; 3]> {
    BLOCK_MAP.get_or_init(|| {
        let data = std::fs::read_to_string("blocks.json").unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&data).unwrap_or_default()
    })
}

pub fn parse_block_name(name: &str) -> Block {
    match name {
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" => Block::Air,
        "minecraft:water" => Block::Water,
        "minecraft:bubble_column" => Block::BubbleColumn,
        _ => {
            let map = get_block_map();
            if let Some(&color) = map.get(name) {
                Block::Color(color)
            } else {
                if let Ok(mut set_guard) = MISSING_BLOCKS.lock() {
                    let set = set_guard.get_or_insert_with(HashSet::new);
                    if set.insert(name.to_string()) {
                        println!("Missing block color: {}", name);
                    }
                }
                Block::Color([255, 0, 255])
            }
        }
    }
}

#[inline(always)]
pub fn block_to_rgb(block: Block) -> [u8; 3] {
    match block {
        Block::Air => [0, 0, 0],
        Block::Water | Block::BubbleColumn => [64, 64, 255],
        Block::Color(rgb) => rgb,
    }
}
