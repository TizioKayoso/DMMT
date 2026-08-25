use std::collections::HashSet;
use std::sync::Mutex;

static MISSING_BLOCKS: Mutex<Option<HashSet<String>>> = Mutex::new(None);
//should i have used a JSON file? yes. Did i? No.
pub fn block_to_rgb(name: &str) -> [u8; 3] {
    match name {
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" => [0, 0, 0],

        "minecraft:grass_block" => [89, 125, 39],
        "minecraft:dirt" | "minecraft:coarse_dirt" | "minecraft:rooted_dirt" => [134, 96, 67],
        "minecraft:podzol" => [92, 64, 42],
        "minecraft:mycelium" => [111, 98, 111],
        "minecraft:mud" => [60, 58, 50],
        "minecraft:packed_mud" => [138, 121, 95],
        "minecraft:mud_bricks" => [89, 68, 62],

        "minecraft:stone" => [125, 125, 125],
        "minecraft:cobblestone" => [115, 115, 115],
        "minecraft:mossy_cobblestone" => [89, 105, 79],
        "minecraft:granite" | "minecraft:polished_granite" => [149, 103, 85],
        "minecraft:diorite" | "minecraft:polished_diorite" => [188, 188, 188],
        "minecraft:andesite" | "minecraft:polished_andesite" => [132, 135, 133],
        "minecraft:tuff" | "minecraft:polished_tuff" | "minecraft:tuff_bricks" => [108, 108, 100],
        "minecraft:calcite" => [224, 224, 220],
        "minecraft:dripstone_block" | "minecraft:pointed_dripstone" => [134, 109, 92],
        "minecraft:deepslate" => [80, 80, 80],
        "minecraft:cobbled_deepslate" => [74, 74, 74],
        "minecraft:polished_deepslate" => [80, 80, 80],
        "minecraft:deepslate_bricks" => [70, 70, 70],
        "minecraft:deepslate_tiles" => [60, 60, 60],
        "minecraft:chiseled_deepslate" => [65, 65, 65],

        "minecraft:sand"
        | "minecraft:sandstone"
        | "minecraft:cut_sandstone"
        | "minecraft:chiseled_sandstone"
        | "minecraft:smooth_sandstone" => [218, 210, 158],
        "minecraft:red_sand"
        | "minecraft:red_sandstone"
        | "minecraft:cut_red_sandstone"
        | "minecraft:chiseled_red_sandstone"
        | "minecraft:smooth_red_sandstone" => [190, 102, 33],
        "minecraft:gravel" => [130, 127, 126],

        "minecraft:coal_ore" | "minecraft:deepslate_coal_ore" => [45, 45, 45],
        "minecraft:iron_ore"
        | "minecraft:deepslate_iron_ore"
        | "minecraft:copper_ore"
        | "minecraft:deepslate_copper_ore"
        | "minecraft:gold_ore"
        | "minecraft:deepslate_gold_ore"
        | "minecraft:redstone_ore"
        | "minecraft:deepslate_redstone_ore"
        | "minecraft:lapis_ore"
        | "minecraft:deepslate_lapis_ore"
        | "minecraft:diamond_ore"
        | "minecraft:deepslate_diamond_ore"
        | "minecraft:emerald_ore"
        | "minecraft:deepslate_emerald_ore" => [167, 167, 167],
        "minecraft:nether_gold_ore" | "minecraft:nether_quartz_ore" => [181, 98, 32],
        "minecraft:ancient_debris" => [91, 58, 48],

        "minecraft:raw_iron_block" => [165, 136, 121],
        "minecraft:raw_copper_block" => [192, 106, 75],
        "minecraft:raw_gold_block" => [221, 169, 46],
        "minecraft:iron_block" => [167, 167, 167],
        "minecraft:gold_block" => [255, 216, 61],
        "minecraft:copper_block"
        | "minecraft:cut_copper"
        | "minecraft:waxed_copper_block"
        | "minecraft:waxed_cut_copper" => [192, 106, 75],
        "minecraft:exposed_copper"
        | "minecraft:exposed_cut_copper"
        | "minecraft:waxed_exposed_copper"
        | "minecraft:waxed_exposed_cut_copper" => [154, 123, 104],
        "minecraft:weathered_copper"
        | "minecraft:weathered_cut_copper"
        | "minecraft:waxed_weathered_copper"
        | "minecraft:waxed_weathered_cut_copper" => [95, 155, 137],
        "minecraft:oxidized_copper"
        | "minecraft:oxidized_cut_copper"
        | "minecraft:waxed_oxidized_copper"
        | "minecraft:waxed_oxidized_cut_copper" => [72, 147, 139],

        "minecraft:diamond_block" => [92, 219, 213],
        "minecraft:emerald_block" => [64, 217, 113],
        "minecraft:lapis_block" => [38, 97, 156],
        "minecraft:redstone_block" => [180, 0, 0],
        "minecraft:netherite_block" => [44, 42, 46],
        "minecraft:amethyst_block" | "minecraft:budding_amethyst" => [145, 92, 178],

        "minecraft:oak_log" | "minecraft:oak_wood" => [102, 76, 40],
        "minecraft:stripped_oak_log" | "minecraft:stripped_oak_wood" => [180, 145, 90],
        "minecraft:oak_planks"
        | "minecraft:oak_fence"
        | "minecraft:oak_fence_gate"
        | "minecraft:oak_door"
        | "minecraft:oak_trapdoor"
        | "minecraft:oak_pressure_plate"
        | "minecraft:oak_button" => [162, 130, 79],
        "minecraft:oak_leaves" => [72, 181, 24],
        "minecraft:oak_sapling" => [72, 140, 32],

        "minecraft:spruce_log" | "minecraft:spruce_wood" => [76, 57, 36],
        "minecraft:stripped_spruce_log" | "minecraft:stripped_spruce_wood" => [116, 88, 52],
        "minecraft:spruce_planks"
        | "minecraft:spruce_fence"
        | "minecraft:spruce_fence_gate"
        | "minecraft:spruce_door"
        | "minecraft:spruce_trapdoor"
        | "minecraft:spruce_pressure_plate"
        | "minecraft:spruce_button" => [114, 84, 48],
        "minecraft:spruce_leaves" => [61, 96, 61],
        "minecraft:spruce_sapling" => [58, 102, 58],

        "minecraft:birch_log" | "minecraft:birch_wood" => [216, 216, 216],
        "minecraft:stripped_birch_log" | "minecraft:stripped_birch_wood" => [196, 182, 124],
        "minecraft:birch_planks"
        | "minecraft:birch_fence"
        | "minecraft:birch_fence_gate"
        | "minecraft:birch_door"
        | "minecraft:birch_trapdoor"
        | "minecraft:birch_pressure_plate"
        | "minecraft:birch_button" => [196, 179, 123],
        "minecraft:birch_leaves" => [128, 167, 85],
        "minecraft:birch_sapling" => [95, 140, 55],

        "minecraft:jungle_log" | "minecraft:jungle_wood" => [92, 65, 42],
        "minecraft:stripped_jungle_log" | "minecraft:stripped_jungle_wood" => [168, 124, 79],
        "minecraft:jungle_planks"
        | "minecraft:jungle_fence"
        | "minecraft:jungle_fence_gate"
        | "minecraft:jungle_door"
        | "minecraft:jungle_trapdoor"
        | "minecraft:jungle_pressure_plate"
        | "minecraft:jungle_button" => [160, 115, 80],
        "minecraft:jungle_leaves" => [72, 181, 24],
        "minecraft:jungle_sapling" => [72, 140, 32],

        "minecraft:acacia_log" | "minecraft:acacia_wood" => [104, 88, 67],
        "minecraft:stripped_acacia_log" | "minecraft:stripped_acacia_wood" => [175, 103, 65],
        "minecraft:acacia_planks"
        | "minecraft:acacia_fence"
        | "minecraft:acacia_fence_gate"
        | "minecraft:acacia_door"
        | "minecraft:acacia_trapdoor"
        | "minecraft:acacia_pressure_plate"
        | "minecraft:acacia_button" => [168, 90, 50],
        "minecraft:acacia_leaves" => [72, 181, 24],
        "minecraft:acacia_sapling" => [72, 140, 32],

        "minecraft:dark_oak_log" | "minecraft:dark_oak_wood" => [59, 45, 27],
        "minecraft:stripped_dark_oak_log" | "minecraft:stripped_dark_oak_wood" => [72, 55, 33],
        "minecraft:dark_oak_planks"
        | "minecraft:dark_oak_fence"
        | "minecraft:dark_oak_fence_gate"
        | "minecraft:dark_oak_door"
        | "minecraft:dark_oak_trapdoor"
        | "minecraft:dark_oak_pressure_plate"
        | "minecraft:dark_oak_button" => [65, 43, 20],
        "minecraft:dark_oak_leaves" => [72, 181, 24],
        "minecraft:dark_oak_sapling" => [72, 140, 32],

        "minecraft:mangrove_log" | "minecraft:mangrove_wood" => [91, 35, 33],
        "minecraft:stripped_mangrove_log"
        | "minecraft:stripped_mangrove_wood"
        | "minecraft:mangrove_planks"
        | "minecraft:mangrove_fence"
        | "minecraft:mangrove_fence_gate"
        | "minecraft:mangrove_door"
        | "minecraft:mangrove_trapdoor"
        | "minecraft:mangrove_pressure_plate"
        | "minecraft:mangrove_button" => [117, 54, 48],
        "minecraft:mangrove_leaves" | "minecraft:mangrove_propagule" => [70, 110, 40],

        "minecraft:cherry_log" | "minecraft:cherry_wood" => [178, 102, 102],
        "minecraft:stripped_cherry_log" | "minecraft:stripped_cherry_wood" => [188, 122, 122],
        "minecraft:cherry_planks"
        | "minecraft:cherry_fence"
        | "minecraft:cherry_fence_gate"
        | "minecraft:cherry_door"
        | "minecraft:cherry_trapdoor"
        | "minecraft:cherry_pressure_plate"
        | "minecraft:cherry_button" => [224, 146, 157],
        "minecraft:cherry_leaves" => [226, 151, 179],

        "minecraft:bamboo_block" => [177, 166, 65],
        "minecraft:stripped_bamboo_block" => [195, 184, 86],
        "minecraft:bamboo_planks" | "minecraft:bamboo_mosaic" => [194, 180, 90],
        "minecraft:bamboo" | "minecraft:bamboo_sapling" => [100, 145, 45],

        "minecraft:crimson_stem" | "minecraft:crimson_hyphae" => [99, 45, 54],
        "minecraft:stripped_crimson_stem" | "minecraft:stripped_crimson_hyphae" => [139, 64, 72],
        "minecraft:crimson_planks" => [113, 54, 67],
        "minecraft:crimson_nylium" | "minecraft:crimson_fungus" | "minecraft:crimson_roots" => {
            [130, 30, 45]
        }

        "minecraft:warped_stem" | "minecraft:warped_hyphae" => [42, 104, 99],
        "minecraft:stripped_warped_stem" | "minecraft:stripped_warped_hyphae" => [55, 130, 123],
        "minecraft:warped_planks" => [42, 105, 99],
        "minecraft:warped_nylium"
        | "minecraft:warped_fungus"
        | "minecraft:warped_roots"
        | "minecraft:nether_sprouts" => [20, 100, 96],
        "minecraft:nether_wart_block" => [117, 25, 33],
        "minecraft:warped_wart_block" => [20, 120, 110],

        "minecraft:netherrack" => [112, 54, 50],
        "minecraft:nether_bricks"
        | "minecraft:cracked_nether_bricks"
        | "minecraft:chiseled_nether_bricks" => [45, 22, 25],
        "minecraft:red_nether_bricks" => [68, 20, 20],
        "minecraft:blackstone"
        | "minecraft:polished_blackstone"
        | "minecraft:polished_blackstone_bricks"
        | "minecraft:cracked_polished_blackstone_bricks"
        | "minecraft:chiseled_polished_blackstone" => [53, 49, 54],
        "minecraft:basalt" | "minecraft:polished_basalt" | "minecraft:smooth_basalt" => {
            [80, 75, 75]
        }
        "minecraft:soul_sand" => [81, 69, 62],
        "minecraft:soul_soil" => [78, 67, 63],
        "minecraft:magma_block" => [150, 50, 30],
        "minecraft:glowstone" => [255, 226, 145],
        "minecraft:shroomlight" => [255, 170, 100],

        "minecraft:end_stone" | "minecraft:end_stone_bricks" => [218, 215, 161],
        "minecraft:purpur_block"
        | "minecraft:purpur_pillar"
        | "minecraft:purpur_slab"
        | "minecraft:purpur_stairs" => [178, 76, 216],
        "minecraft:obsidian" => [20, 18, 29],
        "minecraft:crying_obsidian" => [53, 22, 72],

        "minecraft:bricks"
        | "minecraft:brick_slab"
        | "minecraft:brick_stairs"
        | "minecraft:brick_wall" => [151, 84, 73],

        "minecraft:stone_bricks"
        | "minecraft:stone_brick_slab"
        | "minecraft:stone_brick_stairs"
        | "minecraft:stone_brick_wall" => [125, 125, 125],

        "minecraft:mossy_stone_bricks"
        | "minecraft:mossy_stone_brick_slab"
        | "minecraft:mossy_stone_brick_stairs"
        | "minecraft:mossy_stone_brick_wall" => [89, 105, 79],

        "minecraft:cracked_stone_bricks" => [110, 110, 110],
        "minecraft:chiseled_stone_bricks" => [125, 125, 125],

        "minecraft:quartz_block"
        | "minecraft:quartz_pillar"
        | "minecraft:chiseled_quartz_block"
        | "minecraft:smooth_quartz"
        | "minecraft:quartz_bricks" => [235, 230, 220],

        "minecraft:glass" | "minecraft:glass_pane" => [255, 255, 255],
        "minecraft:tinted_glass" => [50, 50, 50],

        "minecraft:white_wool" | "minecraft:white_carpet" | "minecraft:white_concrete" => {
            [233, 236, 236]
        }
        "minecraft:light_gray_wool" | "minecraft:light_gray_carpet" => [142, 142, 134],
        "minecraft:gray_wool" | "minecraft:gray_carpet" => [62, 68, 68],
        "minecraft:black_wool" | "minecraft:black_carpet" => [20, 21, 25],
        "minecraft:brown_wool" | "minecraft:brown_carpet" => [114, 71, 40],
        "minecraft:red_wool" | "minecraft:red_carpet" => [160, 39, 34],
        "minecraft:orange_wool" | "minecraft:orange_carpet" => [240, 118, 19],
        "minecraft:yellow_wool" | "minecraft:yellow_carpet" => [248, 197, 39],
        "minecraft:lime_wool" | "minecraft:lime_carpet" => [112, 185, 25],
        "minecraft:green_wool" | "minecraft:green_carpet" => [85, 110, 27],
        "minecraft:cyan_wool" | "minecraft:cyan_carpet" => [21, 137, 145],
        "minecraft:light_blue_wool" | "minecraft:light_blue_carpet" => [58, 175, 217],
        "minecraft:blue_wool" | "minecraft:blue_carpet" => [53, 57, 157],
        "minecraft:purple_wool" | "minecraft:purple_carpet" => [121, 42, 172],
        "minecraft:magenta_wool" | "minecraft:magenta_carpet" => [190, 68, 179],
        "minecraft:pink_wool" | "minecraft:pink_carpet" => [237, 141, 172],

        "minecraft:light_gray_concrete" => [125, 125, 115],
        "minecraft:gray_concrete" => [54, 58, 61],
        "minecraft:black_concrete" => [8, 10, 15],
        "minecraft:brown_concrete" => [96, 59, 36],
        "minecraft:red_concrete" => [142, 32, 32],
        "minecraft:orange_concrete" => [224, 97, 0],
        "minecraft:yellow_concrete" => [241, 175, 21],
        "minecraft:lime_concrete" => [94, 169, 24],
        "minecraft:green_concrete" => [73, 91, 36],
        "minecraft:cyan_concrete" => [21, 119, 136],
        "minecraft:light_blue_concrete" => [36, 137, 199],
        "minecraft:blue_concrete" => [44, 46, 143],
        "minecraft:purple_concrete" => [100, 32, 156],
        "minecraft:magenta_concrete" => [169, 44, 159],
        "minecraft:pink_concrete" => [213, 101, 143],

        "minecraft:terracotta" => [152, 94, 68],
        "minecraft:white_terracotta" => [210, 178, 161],
        "minecraft:light_gray_terracotta" => [135, 107, 98],
        "minecraft:gray_terracotta" => [87, 72, 64],
        "minecraft:black_terracotta" => [37, 23, 16],
        "minecraft:brown_terracotta" => [77, 51, 36],
        "minecraft:red_terracotta" => [143, 61, 47],
        "minecraft:orange_terracotta" => [161, 83, 37],
        "minecraft:yellow_terracotta" => [186, 133, 35],
        "minecraft:lime_terracotta" => [103, 117, 53],
        "minecraft:green_terracotta" => [76, 83, 42],
        "minecraft:cyan_terracotta" => [86, 91, 91],
        "minecraft:light_blue_terracotta" => [113, 108, 137],
        "minecraft:blue_terracotta" => [74, 60, 91],
        "minecraft:purple_terracotta" => [118, 70, 86],
        "minecraft:magenta_terracotta" => [149, 88, 108],
        "minecraft:pink_terracotta" => [161, 78, 78],

        "minecraft:snow" | "minecraft:snow_block" => [240, 240, 240],
        "minecraft:ice" => [160, 160, 255],
        "minecraft:packed_ice" => [138, 180, 255],
        "minecraft:blue_ice" => [116, 168, 255],
        "minecraft:frosted_ice" => [160, 160, 255],

        "minecraft:water" => [64, 64, 255],
        "minecraft:kelp" | "minecraft:kelp_plant" => [35, 120, 75],
        "minecraft:seagrass" | "minecraft:tall_seagrass" => [40, 140, 70],

        "minecraft:clay" => [164, 168, 184],

        "minecraft:prismarine" | "minecraft:prismarine_bricks" => [99, 168, 158],
        "minecraft:dark_prismarine" => [53, 104, 97],
        "minecraft:sea_lantern" => [180, 240, 230],

        "minecraft:amethyst_cluster"
        | "minecraft:large_amethyst_bud"
        | "minecraft:medium_amethyst_bud"
        | "minecraft:small_amethyst_bud" => [145, 92, 178],

        "minecraft:ochre_froglight" => [255, 210, 100],
        "minecraft:verdant_froglight" => [170, 230, 170],
        "minecraft:pearlescent_froglight" => [230, 190, 220],

        "minecraft:honey_block" => [235, 180, 55],
        "minecraft:honeycomb_block" => [220, 130, 30],
        "minecraft:slime_block" => [112, 190, 105],

        "minecraft:sponge" => [195, 195, 75],
        "minecraft:wet_sponge" => [170, 190, 75],
        "minecraft:hay_block" => [195, 175, 50],
        "minecraft:bone_block" => [220, 215, 190],
        "minecraft:tnt" => [200, 40, 30],

        "minecraft:dandelion" => [240, 210, 30],
        "minecraft:poppy" => [190, 30, 35],
        "minecraft:blue_orchid" => [50, 100, 220],
        "minecraft:allium" => [150, 70, 190],
        "minecraft:azure_bluet" => [220, 220, 220],
        "minecraft:red_tulip" => [220, 40, 40],
        "minecraft:orange_tulip" => [240, 130, 30],
        "minecraft:white_tulip" => [235, 235, 235],
        "minecraft:pink_tulip" => [240, 120, 170],
        "minecraft:oxeye_daisy" => [235, 235, 210],
        "minecraft:cornflower" => [50, 90, 210],
        "minecraft:lily_of_the_valley" => [235, 235, 235],
        "minecraft:wither_rose" => [35, 25, 25],
        "minecraft:torchflower" | "minecraft:torchflower_crop" => [245, 130, 35],

        "minecraft:short_grass"
        | "minecraft:tall_grass"
        | "minecraft:moss_block"
        | "minecraft:moss_carpet" => [89, 125, 39],

        "minecraft:fern" | "minecraft:large_fern" => [60, 130, 45],

        "minecraft:azalea" | "minecraft:flowering_azalea" | "minecraft:azalea_leaves" => {
            [91, 140, 50]
        }

        "minecraft:flowering_azalea_leaves" => [111, 161, 73],

        "minecraft:red_mushroom" | "minecraft:red_mushroom_block" => [190, 40, 40],
        "minecraft:brown_mushroom" | "minecraft:brown_mushroom_block" => [130, 85, 55],
        "minecraft:mushroom_stem" => [210, 200, 180],

        "minecraft:vine" => [50, 120, 45],
        "minecraft:glow_lichen" => [100, 150, 100],
        "minecraft:weeping_vines" => [145, 35, 45],
        "minecraft:twisting_vines" => [35, 120, 100],

        "minecraft:cactus" => [55, 145, 50],
        "minecraft:pumpkin" | "minecraft:carved_pumpkin" | "minecraft:jack_o_lantern" => {
            [230, 130, 25]
        }
        "minecraft:melon" => [120, 170, 50],
        "minecraft:sugar_cane" => [100, 180, 70],
        "minecraft:cocoa" => [130, 75, 35],
        "minecraft:sweet_berry_bush" => [55, 135, 45],
        "minecraft:pitcher_plant" | "minecraft:pitcher_crop" => [80, 140, 75],
        "minecraft:lily_pad" => [45, 125, 35],
        "minecraft:hanging_roots" => [115, 95, 55],

        "minecraft:tube_coral_block"
        | "minecraft:tube_coral"
        | "minecraft:dead_tube_coral_block"
        | "minecraft:dead_tube_coral" => [40, 130, 180],

        "minecraft:brain_coral_block"
        | "minecraft:brain_coral"
        | "minecraft:dead_brain_coral_block"
        | "minecraft:dead_brain_coral" => [220, 90, 150],

        "minecraft:bubble_coral_block"
        | "minecraft:bubble_coral"
        | "minecraft:dead_bubble_coral_block"
        | "minecraft:dead_bubble_coral" => [150, 90, 210],

        "minecraft:fire_coral_block"
        | "minecraft:fire_coral"
        | "minecraft:dead_fire_coral_block"
        | "minecraft:dead_fire_coral" => [220, 70, 60],

        "minecraft:horn_coral_block"
        | "minecraft:horn_coral"
        | "minecraft:dead_horn_coral_block"
        | "minecraft:dead_horn_coral" => [220, 170, 50],

        "minecraft:torch" | "minecraft:wall_torch" => [255, 190, 80],
        "minecraft:soul_torch" | "minecraft:soul_wall_torch" => [80, 190, 220],
        "minecraft:redstone_torch" | "minecraft:redstone_wall_torch" => [180, 30, 20],
        "minecraft:lantern" | "minecraft:soul_lantern" => [220, 180, 90],
        "minecraft:campfire" | "minecraft:soul_campfire" => [220, 100, 40],
        "minecraft:fire" | "minecraft:soul_fire" => [255, 100, 30],

        "minecraft:iron_bars" | "minecraft:iron_door" | "minecraft:iron_trapdoor" => {
            [167, 167, 167]
        }

        "minecraft:ladder" => [150, 115, 70],

        "minecraft:enchanting_table" => [50, 35, 35],
        "minecraft:ender_chest" => [20, 50, 55],
        "minecraft:chest" | "minecraft:trapped_chest" => [150, 90, 35],
        "minecraft:barrel" => [130, 85, 45],

        "minecraft:redstone_wire" => [180, 0, 0],
        "minecraft:redstone_lamp" => [160, 110, 60],
        "minecraft:observer" => [90, 90, 90],
        "minecraft:dispenser" | "minecraft:dropper" => [115, 115, 115],
        "minecraft:piston" | "minecraft:sticky_piston" => [135, 120, 95],
        "minecraft:note_block" => [100, 65, 45],
        "minecraft:jukebox" => [80, 50, 30],
        "minecraft:hopper" => [80, 80, 80],
        "minecraft:lever" => [100, 100, 100],
        "minecraft:target" => [210, 70, 55],

        "minecraft:rail"
        | "minecraft:powered_rail"
        | "minecraft:detector_rail"
        | "minecraft:activator_rail" => [120, 105, 75],

        "minecraft:furnace" | "minecraft:blast_furnace" | "minecraft:smoker" => [80, 80, 80],

        "minecraft:crafting_table" => [130, 90, 50],
        "minecraft:cartography_table" => [100, 75, 45],
        "minecraft:fletching_table" => [160, 115, 70],
        "minecraft:loom" => [145, 100, 65],
        "minecraft:smithing_table" => [75, 65, 55],
        "minecraft:stonecutter" => [115, 115, 115],
        "minecraft:grindstone" => [125, 125, 125],
        "minecraft:composter" => [105, 70, 40],
        "minecraft:lectern" => [150, 115, 70],
        "minecraft:brewing_stand" => [100, 100, 100],
        "minecraft:cauldron" => [70, 70, 70],

        "minecraft:bedrock" => [80, 80, 80],
        "minecraft:reinforced_deepslate" => [50, 50, 50],
        "minecraft:structure_block" => [95, 85, 110],
        "minecraft:jigsaw" => [125, 100, 140],
        "minecraft:command_block"
        | "minecraft:chain_command_block"
        | "minecraft:repeating_command_block" => [130, 80, 180],

        "minecraft:dragon_egg" => [25, 20, 30],
        "minecraft:end_portal_frame" => [80, 125, 105],
        "minecraft:end_portal" => [10, 5, 25],
        "minecraft:nether_portal" => [90, 20, 150],

        "minecraft:skeleton_skull" | "minecraft:skeleton_wall_skull" => [190, 190, 175],
        "minecraft:wither_skeleton_skull" | "minecraft:wither_skeleton_wall_skull" => [45, 45, 45],
        "minecraft:zombie_head" | "minecraft:zombie_wall_head" => [75, 120, 65],
        "minecraft:creeper_head" | "minecraft:creeper_wall_head" => [75, 150, 60],
        "minecraft:player_head" | "minecraft:player_wall_head" => [190, 150, 120],
        "minecraft:piglin_head" | "minecraft:piglin_wall_head" => [180, 120, 110],
        "minecraft:dragon_head" | "minecraft:dragon_wall_head" => [35, 30, 45],

        "minecraft:sniffer_egg" => [210, 150, 100],
        "minecraft:turtle_egg" => [220, 205, 145],
        "minecraft:frogspawn" => [70, 100, 70],

        "minecraft:sculk" | "minecraft:sculk_vein" => [10, 55, 65],
        "minecraft:sculk_catalyst" => [15, 80, 90],
        "minecraft:sculk_sensor" | "minecraft:calibrated_sculk_sensor" => [20, 90, 100],
        "minecraft:sculk_shrieker" => [15, 70, 80],

        "minecraft:lightning_rod" => [170, 100, 70],
        "minecraft:candle" => [220, 190, 145],
        "minecraft:white_candle" => [235, 235, 225],
        "minecraft:light_gray_candle" => [160, 160, 155],
        "minecraft:gray_candle" => [75, 78, 78],
        "minecraft:black_candle" => [25, 25, 28],
        "minecraft:brown_candle" => [120, 75, 45],
        "minecraft:red_candle" => [170, 45, 40],
        "minecraft:orange_candle" => [235, 120, 25],
        "minecraft:yellow_candle" => [245, 195, 45],
        "minecraft:lime_candle" => [115, 180, 30],
        "minecraft:green_candle" => [75, 110, 35],
        "minecraft:cyan_candle" => [30, 135, 145],
        "minecraft:light_blue_candle" => [60, 170, 215],
        "minecraft:blue_candle" => [55, 60, 155],
        "minecraft:purple_candle" => [120, 45, 170],
        "minecraft:magenta_candle" => [190, 70, 180],
        "minecraft:pink_candle" => [235, 140, 170],

        "minecraft:decorated_pot" => [165, 105, 75],
        "minecraft:trial_spawner" | "minecraft:ominous_trial_spawner" | "minecraft:vault" => {
            [80, 95, 100]
        }
        "minecraft:grass" => [89, 125, 39],
        "minecraft:dirt_path" | "minecraft:grass_path" => [150, 118, 65],
        "minecraft:farmland" => [95, 58, 30],
        "minecraft:smooth_stone" => [160, 160, 160],
        "minecraft:dead_bush" => [110, 80, 40],
        // Flowers & Tall Vegetation
        "minecraft:sunflower" => [245, 200, 35],
        "minecraft:rose_bush" => [170, 35, 35],
        "minecraft:lilac" => [165, 115, 170],
        "minecraft:peony" => [220, 130, 170],
        "minecraft:tall_dry_grass" => [160, 150, 75],
        "minecraft:firefly_bush" | "minecraft:leaf_litter" => [90, 120, 50],
        "minecraft:small_dripleaf" | "minecraft:big_dripleaf" | "minecraft:big_dripleaf_stem" => [80, 140, 50],

        // Slabs, Stairs & Walls
        "minecraft:spruce_slab" => [114, 84, 48], // Matches spruce_planks
        "minecraft:cobblestone_wall" => [115, 115, 115],
        "minecraft:polished_andesite_slab" | "minecraft:andesite_slab" => [132, 135, 133],
        "minecraft:smooth_stone" | "minecraft:smooth_stone_slab" => [160, 160, 160],
        "minecraft:deepslate_tile_stairs" | "minecraft:deepslate_tile_slab" => [60, 60, 60],
        "minecraft:blackstone_stairs"
        | "minecraft:polished_blackstone_stairs"
        | "minecraft:blackstone_slab"
        | "minecraft:polished_blackstone_slab" => [53, 49, 54],

        // Utilities & Water
        "minecraft:anvil" | "minecraft:chipped_anvil" | "minecraft:damaged_anvil" => [70, 70, 70],
        "minecraft:bubble_column" => [64, 64, 255],
        unmapped => {
            if let Ok(mut set_guard) = MISSING_BLOCKS.lock() {
                let set = set_guard.get_or_insert_with(HashSet::new);
                if set.insert(unmapped.to_string()) {
                    println!("Missing block color: {}", unmapped);
                }
            }
            [255, 0, 255]
        }
    }
}
