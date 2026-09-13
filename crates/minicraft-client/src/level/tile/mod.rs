//! Stable tile identities, registry metadata, and tile-local helpers.
//!
//! Layout mirrors Java `level/tile/*`: one module per tile class. This file is
//! the registry (`Tiles`) plus the shared `Tile` identity enum and the public
//! free-function API used by the rest of the client.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

pub mod boss_door;
pub mod boss_floor;
pub mod boss_wall;
pub mod cactus;
pub mod cloud;
pub mod connect_tile;
pub mod decor;
pub mod dirt;
pub mod door;
pub mod exploded;
pub mod fence;
pub mod floor;
pub mod flower;
pub mod grass;
pub mod hard_rock;
pub mod hole;
pub mod infinite_fall;
pub mod lava;
pub mod lava_brick;
pub mod material;
pub mod ore;
pub mod path;
pub mod rock;
pub mod sand;
pub mod sapling;
pub mod sign;
pub mod stairs;
pub mod tile_item;
pub mod torch;
pub mod tree;
pub mod wall;
pub mod water;
pub mod wool;

pub mod entity;
pub mod farming;

pub use tile_item::{is_tile_item, tile_placement};

/// Java `Tile.tickCount`: a global tick counter incremented once after each
/// unpaused `level.tick` (Java `Updater.tick` no-menu branch). Water/lava and
/// other tile-local animation readers may sample this value.
///
/// Java 2.2.4's `SpriteAnimation` for water/lava advances on `Updater.gameTime`
/// rather than this counter; the field is still part of the core-loop contract
/// and is incremented at the same call site.
static TILE_TICK: AtomicU32 = AtomicU32::new(0);

/// Current global tile tick (Java `Tile.tickCount`).
pub fn tile_tick() -> u32 {
    TILE_TICK.load(Ordering::Relaxed)
}

/// Increment after `level.tick`, matching Java `Tile.tickCount++`.
pub fn increment_tile_tick() {
    TILE_TICK.fetch_add(1, Ordering::Relaxed);
}

/// Fluid (water/lava) animation phase derived from the global tile tick.
/// Equivalent to Java `Tile.tickCount % period` readers.
pub fn fluid_animation_phase(period: u32) -> u32 {
    if period == 0 {
        0
    } else {
        tile_tick() % period
    }
}

/// Java `Tile.Material` — wall/floor/door/fence construction material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Wood,
    Stone,
    Obsidian,
}

impl Material {
    pub const ALL: [Self; 3] = [Self::Wood, Self::Stone, Self::Obsidian];

    /// Java `Material.getRequiredTool`.
    pub fn required_tool(self) -> crate::item::ToolKind {
        match self {
            Self::Wood => crate::item::ToolKind::Axe,
            Self::Stone | Self::Obsidian => crate::item::ToolKind::Pickaxe,
        }
    }

    /// Java `Material.ordinal`.
    pub fn ordinal(self) -> u8 {
        match self {
            Self::Wood => 0,
            Self::Stone => 1,
            Self::Obsidian => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[allow(dead_code)] // Remaining 2.2.4 level types will construct every registered tile.
pub enum Tile {
    Grass = 0,
    Dirt = 1,
    Flower = 2,
    Hole = 3,
    StairsUp = 4,
    StairsDown = 5,
    Water = 6,
    Rock = 7,
    Tree = 8,
    TreeSapling = 9,
    Sand = 10,
    Cactus = 11,
    CactusSapling = 12,
    IronOre = 13,
    GoldOre = 14,
    GemOre = 15,
    LapisOre = 16,
    Lava = 17,
    LavaBrick = 18,
    Exploded = 19,
    Farmland = 20,
    Wheat = 21,
    HardRock = 22,
    InfiniteFall = 23,
    Cloud = 24,
    CloudOre = 25,
    WoodDoor = 26,
    StoneDoor = 27,
    ObsidianDoor = 28,
    WoodFloor = 29,
    StoneFloor = 30,
    ObsidianFloor = 31,
    WoodWall = 32,
    StoneWall = 33,
    ObsidianWall = 34,
    WhiteWool = 35,
    Path = 36,
    RedWool = 37,
    BlueWool = 38,
    GreenWool = 39,
    YellowWool = 40,
    BlackWool = 41,
    Potato = 42,
    RawStone = 43,
    RawObsidian = 44,
    OrnateStone = 45,
    OrnateObsidian = 46,
    BossWall = 47,
    BossFloor = 48,
    BossDoor = 49,
    Tomato = 50,
    Carrot = 51,
    HeavenlyBerries = 52,
    HellishBerries = 53,
    WoodFence = 54,
    StoneFence = 55,
    ObsidianFence = 56,
    Torch = 57,
    Sign = 58,
}

impl Tile {
    pub const ALL: [Self; 59] = [
        Self::Grass,
        Self::Dirt,
        Self::Flower,
        Self::Hole,
        Self::StairsUp,
        Self::StairsDown,
        Self::Water,
        Self::Rock,
        Self::Tree,
        Self::TreeSapling,
        Self::Sand,
        Self::Cactus,
        Self::CactusSapling,
        Self::IronOre,
        Self::GoldOre,
        Self::GemOre,
        Self::LapisOre,
        Self::Lava,
        Self::LavaBrick,
        Self::Exploded,
        Self::Farmland,
        Self::Wheat,
        Self::HardRock,
        Self::InfiniteFall,
        Self::Cloud,
        Self::CloudOre,
        Self::WoodDoor,
        Self::StoneDoor,
        Self::ObsidianDoor,
        Self::WoodFloor,
        Self::StoneFloor,
        Self::ObsidianFloor,
        Self::WoodWall,
        Self::StoneWall,
        Self::ObsidianWall,
        Self::WhiteWool,
        Self::Path,
        Self::RedWool,
        Self::BlueWool,
        Self::GreenWool,
        Self::YellowWool,
        Self::BlackWool,
        Self::Potato,
        Self::RawStone,
        Self::RawObsidian,
        Self::OrnateStone,
        Self::OrnateObsidian,
        Self::BossWall,
        Self::BossFloor,
        Self::BossDoor,
        Self::Tomato,
        Self::Carrot,
        Self::HeavenlyBerries,
        Self::HellishBerries,
        Self::WoodFence,
        Self::StoneFence,
        Self::ObsidianFence,
        Self::Torch,
        Self::Sign,
    ];

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.get(id as usize).copied()
    }

    #[allow(dead_code)] // Consumed by the Java-save importer scheduled for phase 7.
    pub fn from_legacy_id(id: u16) -> Option<(Self, u16)> {
        let plain = |tile| Some((tile, 0));
        match id {
            0 | 100 => plain(Self::Grass),
            1 => plain(Self::Rock),
            2 | 104 => plain(Self::Water),
            3 | 107 => plain(Self::Flower),
            4 | 102 => plain(Self::Tree),
            5 | 105 => plain(Self::Dirt),
            6 | 101 => plain(Self::Sand),
            7 | 103 => plain(Self::Cactus),
            8 | 119 => plain(Self::Hole),
            9 | 64 => plain(Self::TreeSapling),
            10 | 65 => plain(Self::CactusSapling),
            11 => plain(Self::Farmland),
            12 => plain(Self::Wheat),
            13 => plain(Self::Lava),
            14 | 109 => plain(Self::StairsDown),
            15 | 108 => plain(Self::StairsUp),
            16 => plain(Self::InfiniteFall),
            17 => plain(Self::Cloud),
            18 => plain(Self::HardRock),
            19 => plain(Self::IronOre),
            20 => plain(Self::GoldOre),
            21 => plain(Self::GemOre),
            22 => plain(Self::CloudOre),
            24 => plain(Self::LapisOre),
            30 => plain(Self::Exploded),
            31 | 110 => plain(Self::WoodFloor),
            32 | 111 => plain(Self::StoneFloor),
            33 => plain(Self::WoodWall),
            34 => plain(Self::StoneWall),
            35 | 36 | 112 | 113 => plain(Self::WoodDoor),
            37 | 38 | 114 | 115 => plain(Self::StoneDoor),
            39 => plain(Self::LavaBrick),
            41 | 57 => plain(Self::WhiteWool),
            42 | 58 => plain(Self::RedWool),
            43 | 59 => plain(Self::BlueWool),
            45 | 60 => plain(Self::GreenWool),
            56 | 62 => plain(Self::BlackWool),
            61 | 127 => plain(Self::YellowWool),
            63 | 120 => plain(Self::ObsidianFloor),
            121 => plain(Self::ObsidianWall),
            122 | 123 | 116 | 117 => plain(Self::ObsidianDoor),
            40 => Some((Self::Torch, Self::Sand.id() as u16)),
            44 => Some((Self::Torch, Self::Grass.id() as u16)),
            46 => Some((Self::Torch, Self::Dirt.id() as u16)),
            47 => Some((Self::Torch, Self::WoodFloor.id() as u16)),
            48 => Some((Self::Torch, Self::StoneFloor.id() as u16)),
            49 => Some((Self::Torch, Self::ObsidianFloor.id() as u16)),
            50 => Some((Self::Torch, Self::WhiteWool.id() as u16)),
            51 => Some((Self::Torch, Self::RedWool.id() as u16)),
            52 => Some((Self::Torch, Self::BlueWool.id() as u16)),
            53 => Some((Self::Torch, Self::GreenWool.id() as u16)),
            54 => Some((Self::Torch, Self::YellowWool.id() as u16)),
            55 => Some((Self::Torch, Self::BlackWool.id() as u16)),
            _ => None,
        }
    }

    pub fn asset_name(self) -> &'static str {
        const ASSETS: [&str; 59] = [
            "grass",
            "dirt",
            "flower_shape0",
            "hole",
            "stairs_up",
            "stairs_down",
            "water",
            "rock",
            "oak",
            "sapling",
            "sand",
            "cactus",
            "sapling",
            "iron_ore",
            "gold_ore",
            "gem_ore",
            "lapis_ore",
            "lava",
            "missing_tile",
            "exploded",
            "farmland",
            "wheat_stage5",
            "hardrock",
            "cloud_background",
            "cloud",
            "cloud_ore",
            "wood_door",
            "stone_door",
            "obsidian_door",
            "wood_floor",
            "stone_floor",
            "obsidian_floor",
            "wood_wall",
            "stone_wall",
            "obsidian_wall",
            "white_wool",
            "path",
            "red_wool",
            "blue_wool",
            "green_wool",
            "yellow_wool",
            "black_wool",
            "potato_stage5",
            "stone",
            "obsidian",
            "ornate_stone",
            "ornate_obsidian",
            "obsidian_wall",
            "obsidian_floor",
            "obsidian_door",
            "tomato_stage3",
            "carrot_stage3",
            "heavenly_berries_stage3",
            "hellish_berries_stage3",
            "wood_fence",
            "stone_fence",
            "obsidian_fence",
            "torch",
            "sign",
        ];
        ASSETS[self.id() as usize]
    }

    /// Inverse of Java `Tile.mayPass` for walkable hosts (no boats).
    /// `data` is the tile's level data (doors store open/closed there).
    pub fn solid(self, data: u16) -> bool {
        match self {
            Self::WoodDoor | Self::StoneDoor | Self::ObsidianDoor | Self::BossDoor => {
                door::is_closed(data)
            }
            Self::Rock => true,
            Self::IronOre | Self::GoldOre | Self::GemOre | Self::LapisOre | Self::CloudOre => true,
            Self::Tree | Self::Cactus | Self::HardRock => true,
            Self::InfiniteFall => true,
            Self::WoodWall | Self::StoneWall | Self::ObsidianWall | Self::BossWall => true,
            Self::WoodFence | Self::StoneFence | Self::ObsidianFence => true,
            _ => false,
        }
    }

    pub fn light_radius(self) -> i32 {
        match self {
            Self::Lava => lava::LIGHT_RADIUS,
            Self::Torch => torch::LIGHT_RADIUS,
            _ => 0,
        }
    }

    /// Java `Tile.maySpawn`.
    pub fn may_spawn(self) -> bool {
        match self {
            Self::Grass => grass::MAY_SPAWN,
            Self::Dirt => dirt::MAY_SPAWN,
            Self::Flower => flower::MAY_SPAWN,
            Self::Sand => sand::MAY_SPAWN,
            Self::TreeSapling | Self::CactusSapling => sapling::MAY_SPAWN,
            Self::Path => path::MAY_SPAWN,
            Self::WoodFloor | Self::StoneFloor | Self::ObsidianFloor => floor::MAY_SPAWN,
            Self::RawStone | Self::RawObsidian => material::MAY_SPAWN,
            Self::OrnateStone | Self::OrnateObsidian => decor::MAY_SPAWN,
            _ => false,
        }
    }

    /// Java `Tiles.explosionBlacklist`.
    pub fn explosion_blacklisted(self) -> bool {
        matches!(
            self,
            Self::StairsUp
                | Self::StairsDown
                | Self::HardRock
                | Self::ObsidianDoor
                | Self::ObsidianFloor
                | Self::ObsidianWall
                | Self::RawObsidian
                | Self::OrnateObsidian
                | Self::BossWall
                | Self::BossFloor
                | Self::BossDoor
        )
    }
}

/// Join mask for auto-tiling (N, S, W, E, NW, SW, NE, SE).
pub fn connection_mask(
    tiles: &[Tile],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    tile: Tile,
) -> [bool; 8] {
    [
        (0, -1),
        (0, 1),
        (-1, 0),
        (1, 0),
        (-1, -1),
        (-1, 1),
        (1, -1),
        (1, 1),
    ]
    .map(|(dx, dy)| {
        let xx = x as i32 + dx;
        let yy = y as i32 + dy;
        if xx < 0 || yy < 0 || xx >= width as i32 || yy >= height as i32 {
            return false;
        }
        connects(tile, tiles[xx as usize + yy as usize * width])
    })
}

/// Whether `tile`'s auto-tiling appearance joins through `neighbor`.
/// Per-class logic lives in the matching module (Java sprite connection checkers).
pub fn connects(tile: Tile, neighbor: Tile) -> bool {
    match tile {
        Tile::Dirt => dirt::connects(neighbor),
        Tile::Grass => grass::connects(neighbor),
        Tile::Flower => flower::connects(neighbor),
        Tile::Tree => tree::connects(neighbor),
        Tile::TreeSapling => sapling::tree_connects(neighbor),
        Tile::CactusSapling => sapling::cactus_connects(neighbor),
        Tile::Path => path::connects(neighbor),
        Tile::Sand => sand::connects(neighbor),
        Tile::Cactus => cactus::connects(neighbor),
        Tile::Water => water::connects(neighbor),
        Tile::Lava => lava::connects(neighbor),
        Tile::Hole => hole::connects(neighbor),
        Tile::Rock => rock::connects(neighbor),
        Tile::HardRock => hard_rock::connects(neighbor),
        Tile::Cloud => cloud::connects(neighbor),
        Tile::Exploded => exploded::connects(neighbor),
        Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall | Tile::BossWall => {
            wall::connects(neighbor)
        }
        _ => false,
    }
}

/// Overlay tiles (torch/fence/sign) forward appearance to their base tile in `data`.
pub fn underlay_layers(tile: Tile, data: u16, depth: i8) -> [Option<Tile>; 2] {
    match tile {
        Tile::Dirt => dirt::underlay(),
        Tile::Grass => grass::underlay(),
        Tile::Flower => flower::underlay(),
        Tile::Tree => tree::underlay(),
        Tile::TreeSapling => sapling::tree_underlay(),
        Tile::CactusSapling => sapling::cactus_underlay(),
        Tile::Path => path::underlay(),
        Tile::Sand => sand::underlay(),
        Tile::Cactus => cactus::underlay(),
        Tile::Water => water::underlay(),
        Tile::Lava => lava::underlay(),
        Tile::Hole => hole::underlay(),
        Tile::Rock => rock::underlay(),
        Tile::HardRock => hard_rock::underlay(),
        Tile::IronOre => ore::underlay(ore::OreType::Iron),
        Tile::GoldOre => ore::underlay(ore::OreType::Gold),
        Tile::GemOre => ore::underlay(ore::OreType::Gem),
        Tile::LapisOre => ore::underlay(ore::OreType::Lapis),
        Tile::CloudOre => ore::underlay(ore::OreType::Cloud),
        Tile::StairsUp | Tile::StairsDown => stairs::underlay(depth),
        Tile::Torch => torch::underlay(data),
        Tile::Sign => sign::underlay(data),
        Tile::WoodFence | Tile::StoneFence | Tile::ObsidianFence => fence::underlay(data),
        _ => [None, None],
    }
}

pub fn find_stair_site(
    tiles: &[Tile],
    width: usize,
    height: usize,
    origin: usize,
) -> Option<usize> {
    let origin_x = origin % width;
    let origin_y = origin / width;
    for radius in 1..width.max(height) {
        let left = origin_x.saturating_sub(radius);
        let right = (origin_x + radius).min(width - 1);
        let top = origin_y.saturating_sub(radius);
        let bottom = (origin_y + radius).min(height - 1);
        for y in top..=bottom {
            for x in left..=right {
                let index = x + y * width;
                if stairs::is_site(tiles[index]) {
                    return Some(index);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_tick_increments_like_java_static() {
        let before = tile_tick();
        increment_tile_tick();
        // Other threads may also increment; the contract is monotonic +1 from us.
        assert!(tile_tick() >= before + 1);
        increment_tile_tick();
        assert!(tile_tick() >= before + 2);
    }

    #[test]
    fn fluid_animation_phase_zero_period_is_a_no_animation_guard() {
        assert_eq!(fluid_animation_phase(0), 0);
    }

    #[test]
    fn water_and_lava_are_the_fluid_tiles_named_by_java_tickcount_comment() {
        assert_eq!(Tile::Water.asset_name(), "water");
        assert_eq!(Tile::Lava.asset_name(), "lava");
        // Both fluid tiles connect to each other (Java connectsToFluid).
        assert!(connects(Tile::Water, Tile::Lava));
        assert!(connects(Tile::Lava, Tile::Water));
    }

    #[test]
    fn registry_ids_match_java_tiles_init() {
        assert_eq!(Tile::ALL.len(), 59);
        assert_eq!(Tile::Grass.id(), 0);
        assert_eq!(Tile::Lava.id(), 17);
        assert_eq!(Tile::Sign.id(), 58);
        assert!(Tile::StairsUp.explosion_blacklisted());
        assert!(!Tile::Grass.explosion_blacklisted());
        assert!(Tile::Grass.may_spawn());
        assert!(!Tile::Water.may_spawn());
    }
}
