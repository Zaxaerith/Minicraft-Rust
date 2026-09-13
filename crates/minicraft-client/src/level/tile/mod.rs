//! Stable tile identities, registry metadata, and tile-local helpers.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::item::ItemId;

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

    pub fn solid(self, data: u16) -> bool {
        if matches!(
            self,
            Self::WoodDoor | Self::StoneDoor | Self::ObsidianDoor | Self::BossDoor
        ) {
            return data == 0;
        }
        matches!(
            self,
            Self::Rock
                | Self::IronOre
                | Self::GoldOre
                | Self::GemOre
                | Self::LapisOre
                | Self::CloudOre
                | Self::Tree
                | Self::Cactus
                | Self::HardRock
                | Self::InfiniteFall
                | Self::WoodWall
                | Self::StoneWall
                | Self::ObsidianWall
                | Self::BossWall
                | Self::WoodFence
                | Self::StoneFence
                | Self::ObsidianFence
        )
    }

    pub fn light_radius(self) -> i32 {
        match self {
            Self::Lava => 6,
            Self::Torch => 5,
            _ => 0,
        }
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

pub fn connects(tile: Tile, neighbor: Tile) -> bool {
    match tile {
        Tile::Grass => matches!(
            neighbor,
            Tile::Grass | Tile::Flower | Tile::Tree | Tile::TreeSapling | Tile::Path
        ),
        Tile::Sand => matches!(
            neighbor,
            Tile::Sand
                | Tile::Cactus
                | Tile::CactusSapling
                | Tile::Hole
                | Tile::Lava
                | Tile::Exploded
        ),
        Tile::Water | Tile::Lava | Tile::Hole => {
            matches!(
                neighbor,
                Tile::Water | Tile::Lava | Tile::Hole | Tile::Exploded
            )
        }
        Tile::Rock => neighbor == Tile::Rock,
        Tile::HardRock => neighbor == Tile::HardRock,
        Tile::Cloud => neighbor != Tile::InfiniteFall,
        Tile::Exploded => neighbor == Tile::Exploded,
        Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall | Tile::BossWall => matches!(
            neighbor,
            Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall | Tile::BossWall
        ),
        _ => false,
    }
}

pub fn underlay_layers(tile: Tile, data: u16, depth: i8) -> [Option<Tile>; 2] {
    match tile {
        Tile::Grass => [Some(Tile::Dirt), None],
        Tile::Flower | Tile::Tree | Tile::TreeSapling | Tile::Path => {
            [Some(Tile::Dirt), Some(Tile::Grass)]
        }
        Tile::Sand => [Some(Tile::Dirt), None],
        Tile::Cactus | Tile::CactusSapling => [Some(Tile::Dirt), Some(Tile::Sand)],
        Tile::Water | Tile::Lava | Tile::Hole | Tile::Rock | Tile::HardRock => {
            [Some(Tile::Dirt), None]
        }
        Tile::StairsUp | Tile::StairsDown => [
            Some(if depth == 1 { Tile::Cloud } else { Tile::Dirt }),
            None,
        ],
        Tile::Torch | Tile::Sign | Tile::WoodFence | Tile::StoneFence | Tile::ObsidianFence => {
            let base = u8::try_from(data).ok().and_then(Tile::from_id);
            if matches!(base, Some(Tile::Grass | Tile::Sand)) {
                [Some(Tile::Dirt), base]
            } else {
                [base, None]
            }
        }
        _ => [None, None],
    }
}

pub fn tile_placement(item: ItemId, target: Tile) -> Option<(Tile, u16)> {
    let fluid_or_hole = matches!(target, Tile::Hole | Tile::Water | Tile::Lava);
    let floor_target = matches!(
        target,
        Tile::Dirt
            | Tile::Grass
            | Tile::Sand
            | Tile::Path
            | Tile::WoodFloor
            | Tile::StoneFloor
            | Tile::ObsidianFloor
            | Tile::RawStone
            | Tile::RawObsidian
            | Tile::OrnateStone
            | Tile::OrnateObsidian
            | Tile::WhiteWool
            | Tile::RedWool
            | Tile::BlueWool
            | Tile::GreenWool
            | Tile::YellowWool
            | Tile::BlackWool
    );
    Some(match item {
        ItemId::Flower if target == Tile::Grass => (Tile::Flower, 0),
        ItemId::Acorn if target == Tile::Grass => (Tile::TreeSapling, 0),
        ItemId::Dirt if fluid_or_hole => (Tile::Dirt, 0),
        ItemId::NaturalRock
            if fluid_or_hole
                || matches!(target, Tile::Dirt | Tile::Grass | Tile::Sand | Tile::Path) =>
        {
            (Tile::Rock, 0)
        }
        ItemId::Plank if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud) => {
            (Tile::WoodFloor, 0)
        }
        ItemId::PlankWall if target == Tile::WoodFloor => (Tile::WoodWall, 0),
        ItemId::WoodDoor if target == Tile::WoodFloor => (Tile::WoodDoor, 0),
        ItemId::WoodFence if floor_target => (Tile::WoodFence, target.id() as u16),
        ItemId::Stone if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) => {
            (Tile::RawStone, 0)
        }
        ItemId::StoneBrick
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::StoneFloor, 0)
        }
        ItemId::OrnateStone
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::OrnateStone, 0)
        }
        ItemId::StoneWall if target == Tile::StoneFloor => (Tile::StoneWall, 0),
        ItemId::StoneDoor if target == Tile::StoneFloor => (Tile::StoneDoor, 0),
        ItemId::StoneFence if floor_target => (Tile::StoneFence, target.id() as u16),
        ItemId::RawObsidian
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::RawObsidian, 0)
        }
        ItemId::ObsidianBrick
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::ObsidianFloor, 0)
        }
        ItemId::OrnateObsidian
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::OrnateObsidian, 0)
        }
        ItemId::ObsidianWall if target == Tile::ObsidianFloor => (Tile::ObsidianWall, 0),
        ItemId::ObsidianDoor if target == Tile::ObsidianFloor => (Tile::ObsidianDoor, 0),
        ItemId::ObsidianFence if floor_target => (Tile::ObsidianFence, target.id() as u16),
        ItemId::Wool if matches!(target, Tile::Hole | Tile::Water) => (Tile::WhiteWool, 0),
        ItemId::RedWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::RedWool, 0),
        ItemId::BlueWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::BlueWool, 0),
        ItemId::GreenWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::GreenWool, 0),
        ItemId::YellowWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::YellowWool, 0),
        ItemId::BlackWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::BlackWool, 0),
        ItemId::Sand if fluid_or_hole => (Tile::Sand, 0),
        ItemId::Cactus if target == Tile::Sand => (Tile::CactusSapling, 0),
        ItemId::Cloud if target == Tile::InfiniteFall => (Tile::Cloud, 0),
        ItemId::WheatSeeds if target == Tile::Farmland => (Tile::Wheat, 0),
        ItemId::Potato if target == Tile::Farmland => (Tile::Potato, 0),
        ItemId::Carrot if target == Tile::Farmland => (Tile::Carrot, 0),
        ItemId::TomatoSeeds if target == Tile::Farmland => (Tile::Tomato, 0),
        ItemId::HeavenlyBerries if target == Tile::Farmland => (Tile::HeavenlyBerries, 0),
        ItemId::HellishBerries if target == Tile::Farmland => (Tile::HellishBerries, 0),
        ItemId::GrassSeeds if target == Tile::Dirt => (Tile::Grass, 0),
        ItemId::Torch if floor_target => (Tile::Torch, target.id() as u16),
        ItemId::Sign if floor_target => (Tile::Sign, target.id() as u16),
        ItemId::FarmlandItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Farmland, 0)
        }
        ItemId::HoleItem if matches!(target, Tile::Dirt | Tile::Grass) => (Tile::Hole, 0),
        ItemId::LavaItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Lava, 0)
        }
        ItemId::PathItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Path, 0)
        }
        ItemId::WaterItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Water, 0)
        }
        _ => return None,
    })
}

/// Java TileItem instances: shows the placement frame and places via tile_placement.
pub fn is_tile_item(item: ItemId) -> bool {
    [
        Tile::Hole,
        Tile::Water,
        Tile::Lava,
        Tile::Grass,
        Tile::Sand,
        Tile::Dirt,
        Tile::Path,
        Tile::WoodFloor,
        Tile::StoneFloor,
        Tile::Farmland,
        Tile::Cloud,
        Tile::ObsidianFloor,
        Tile::WhiteWool,
        Tile::WoodWall,
        Tile::StoneWall,
    ]
    .iter()
    .any(|target| tile_placement(item, *target).is_some())
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
                if matches!(
                    tiles[index],
                    Tile::Rock
                        | Tile::Dirt
                        | Tile::Grass
                        | Tile::Cloud
                        | Tile::ObsidianFloor
                        | Tile::RawObsidian
                ) {
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
}
