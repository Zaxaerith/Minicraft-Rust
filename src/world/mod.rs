mod generation;
mod random;
pub mod spawn;
mod structure;
mod tile_behavior;

use crate::{
    assets::Assets,
    gfx::{HEIGHT, Screen, WIDTH},
    input::Input,
};

const TILE_SIZE: i32 = 16;
const DAY_LENGTH: u32 = 64_800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Normal,
    Forest,
    Desert,
    Plain,
    Hell,
}

impl Theme {
    pub fn from_index(index: usize) -> Self {
        [
            Self::Normal,
            Self::Forest,
            Self::Desert,
            Self::Plain,
            Self::Hell,
        ]
        .get(index)
        .copied()
        .unwrap_or(Self::Normal)
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainType {
    Island,
    Box,
    Mountain,
    Irregular,
}

impl TerrainType {
    pub fn from_index(index: usize) -> Self {
        [Self::Island, Self::Box, Self::Mountain, Self::Irregular]
            .get(index)
            .copied()
            .unwrap_or(Self::Island)
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSpec {
    pub size: usize,
    pub theme: Theme,
    pub terrain: TerrainType,
}

impl Default for WorldSpec {
    fn default() -> Self {
        Self {
            size: 128,
            theme: Theme::Normal,
            terrain: TerrainType::Island,
        }
    }
}

impl WorldSpec {
    pub fn new(size: usize, theme: usize, terrain: usize) -> Self {
        Self {
            size: match size {
                128 | 256 | 512 => size,
                _ => 128,
            },
            theme: Theme::from_index(theme),
            terrain: TerrainType::from_index(terrain),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    fn solid(self, data: u16) -> bool {
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

    fn light_radius(self) -> i32 {
        match self {
            Self::Lava => 6,
            Self::Torch => 5,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Down,
    Up,
    Left,
    Right,
}

struct Player {
    x: i32,
    y: i32,
    direction: Direction,
    walk_distance: u32,
    health: u8,
    stamina: u8,
    wood: u16,
}

struct Level {
    depth: i8,
    tiles: Vec<Tile>,
    data: Vec<u16>,
    max_mob_count: usize,
    pending_spawns: Vec<spawn::NaturalMob>,
}

pub enum WorldAction {
    None,
    ReturnToTitle,
}

pub struct World {
    width: usize,
    height: usize,
    levels: Vec<Level>,
    current_level: usize,
    player: Player,
    seed: i64,
    tick: u64,
    day_tick: u32,
    days: u32,
    paused: bool,
    pause_selection: usize,
    inventory_open: bool,
    notification: Option<(String, u16)>,
    random: random::JavaRandom,
}

impl World {
    pub fn new_with_options(seed: i64, spec: WorldSpec, difficulty: usize) -> Self {
        let size = spec.size;
        let mut levels: Vec<Level> = [1, 0, -1, -2, -3, -4]
            .into_iter()
            .map(|depth| {
                let generated = generation::level(size, size, depth, seed, spec);
                Level {
                    depth,
                    tiles: generated.tiles,
                    data: generated.data,
                    max_mob_count: spawn::max_mob_count(depth, difficulty),
                    pending_spawns: Vec::new(),
                }
            })
            .collect();
        for level in &mut levels {
            structure::decorate(level, size, size, seed);
            for (tile, data) in level.tiles.iter().zip(&mut level.data) {
                if *tile != Tile::Flower {
                    *data = 0;
                }
            }
        }
        let mut down_stairs: Vec<Vec<usize>> = levels
            .iter()
            .map(|level| {
                level
                    .tiles
                    .iter()
                    .enumerate()
                    .filter_map(|(index, tile)| (*tile == Tile::StairsDown).then_some(index))
                    .collect()
            })
            .collect();
        for upper in 0..levels.len() - 1 {
            let stairs = down_stairs[upper].clone();
            for &index in &stairs {
                if levels[upper + 1].tiles[index] == Tile::StairsDown
                    && let Some(replacement) =
                        find_stair_site(&levels[upper + 1].tiles, size, size, index)
                {
                    levels[upper + 1].tiles[replacement] = Tile::StairsDown;
                    if let Some(saved) = down_stairs[upper + 1]
                        .iter_mut()
                        .find(|saved| **saved == index)
                    {
                        *saved = replacement;
                    }
                }
            }
            structure::link_from_parent(&mut levels[upper + 1], size, size, &stairs);
            for saved in &mut down_stairs[upper + 1] {
                if levels[upper + 1].tiles[*saved] != Tile::StairsDown
                    && let Some(replacement) =
                        find_stair_site(&levels[upper + 1].tiles, size, size, *saved)
                {
                    levels[upper + 1].tiles[replacement] = Tile::StairsDown;
                    levels[upper + 1].data[replacement] = 0;
                    *saved = replacement;
                }
            }
        }
        let current_level = 1;
        let (spawn_x, spawn_y) = find_spawn(&levels[current_level].tiles, size, size);
        Self {
            width: size,
            height: size,
            levels,
            current_level,
            player: Player {
                x: spawn_x * TILE_SIZE + 8,
                y: spawn_y * TILE_SIZE + 8,
                direction: Direction::Down,
                walk_distance: 0,
                health: 10,
                stamina: 10,
                wood: 0,
            },
            seed,
            tick: 0,
            day_tick: 0,
            days: 1,
            paused: false,
            pause_selection: 0,
            inventory_open: false,
            notification: Some(("A NEW WORLD AWAKENS".to_owned(), 150)),
            random: random::JavaRandom::new(seed ^ 0x05EE_D224),
        }
    }

    pub fn new_at_depth_with_options(
        seed: i64,
        depth: i8,
        spec: WorldSpec,
        difficulty: usize,
    ) -> Result<Self, String> {
        let mut world = Self::new_with_options(seed, spec, difficulty);
        let index = world
            .levels
            .iter()
            .position(|level| level.depth == depth)
            .ok_or_else(|| format!("depth must be between -4 and 1, got {depth}"))?;
        world.current_level = index;
        let (x, y) = find_spawn(&world.levels[index].tiles, world.width, world.height);
        world.player.x = x * TILE_SIZE + 8;
        world.player.y = y * TILE_SIZE + 8;
        world.notification = Some((format!("DEPTH {depth} PREVIEW"), 150));
        Ok(world)
    }

    pub fn tick(&mut self, input: &Input) -> WorldAction {
        if input.exit {
            if self.inventory_open {
                self.inventory_open = false;
            } else {
                self.paused = !self.paused;
            }
            return WorldAction::None;
        }
        if self.paused {
            if input.up_pressed {
                self.pause_selection = self.pause_selection.saturating_sub(1);
            }
            if input.down_pressed {
                self.pause_selection = (self.pause_selection + 1).min(1);
            }
            if input.select {
                if self.pause_selection == 0 {
                    self.paused = false;
                } else {
                    return WorldAction::ReturnToTitle;
                }
            }
            return WorldAction::None;
        }
        if input.menu {
            self.inventory_open = !self.inventory_open;
        }
        if self.inventory_open {
            return WorldAction::None;
        }

        self.tick = self.tick.wrapping_add(1);
        self.day_tick += 1;
        if self.day_tick >= DAY_LENGTH {
            self.day_tick = 0;
            self.days += 1;
        }
        tile_behavior::tick_random_tiles(
            &mut self.levels[self.current_level],
            self.width,
            self.height,
            &mut self.random,
        );
        try_queue_natural_spawn(
            &mut self.levels[self.current_level],
            self.width,
            self.height,
            self.player.x,
            self.player.y,
            self.day_tick,
            self.days,
            &mut self.random,
        );
        if let Some((_, remaining)) = &mut self.notification {
            *remaining = remaining.saturating_sub(1);
            if *remaining == 0 {
                self.notification = None;
            }
        }

        let mut horizontal = 0;
        let mut vertical = 0;
        if input.left {
            horizontal -= 1;
            self.player.direction = Direction::Left;
        }
        if input.right {
            horizontal += 1;
            self.player.direction = Direction::Right;
        }
        if input.up {
            vertical -= 1;
            self.player.direction = Direction::Up;
        }
        if input.down {
            vertical += 1;
            self.player.direction = Direction::Down;
        }

        if horizontal != 0 || vertical != 0 {
            let on_water = self.tile_at_pixel(self.player.x, self.player.y) == Tile::Water;
            if !on_water || self.tick.is_multiple_of(2) {
                self.move_player(horizontal, vertical);
                self.player.walk_distance = self.player.walk_distance.wrapping_add(1);
            }
        }
        if input.attack {
            self.attack();
        }
        if input.select {
            self.use_stairs();
        }
        if self.tick.is_multiple_of(90) && self.player.stamina < 10 {
            self.player.stamina += 1;
        }
        if self.tile_at_pixel(self.player.x, self.player.y) == Tile::Lava
            && self.tick.is_multiple_of(30)
        {
            self.player.health = self.player.health.saturating_sub(1);
            self.notification = Some(("THE LAVA BURNS".to_owned(), 45));
        }
        WorldAction::None
    }

    fn move_player(&mut self, horizontal: i32, vertical: i32) {
        let next_x = self.player.x + horizontal;
        if self.can_stand(next_x, self.player.y) {
            self.player.x = next_x;
        }
        let next_y = self.player.y + vertical;
        if self.can_stand(self.player.x, next_y) {
            self.player.y = next_y;
        }
    }

    fn can_stand(&self, x: i32, y: i32) -> bool {
        [(-4, -3), (4, -3), (-4, 4), (4, 4)]
            .into_iter()
            .all(|(offset_x, offset_y)| {
                let (tile, data) = self.tile_and_data_at_pixel(x + offset_x, y + offset_y);
                !tile.solid(data)
            })
    }

    fn tile_at_pixel(&self, x: i32, y: i32) -> Tile {
        self.tile_and_data_at_pixel(x, y).0
    }

    fn tile_and_data_at_pixel(&self, x: i32, y: i32) -> (Tile, u16) {
        let tile_x = (x / TILE_SIZE).clamp(0, self.width as i32 - 1) as usize;
        let tile_y = (y / TILE_SIZE).clamp(0, self.height as i32 - 1) as usize;
        let index = tile_x + tile_y * self.width;
        (
            self.levels[self.current_level].tiles[index],
            self.levels[self.current_level].data[index],
        )
    }

    fn attack(&mut self) {
        let (offset_x, offset_y) = match self.player.direction {
            Direction::Down => (0, 12),
            Direction::Up => (0, -12),
            Direction::Left => (-12, 0),
            Direction::Right => (12, 0),
        };
        let tile_x = (self.player.x + offset_x) / TILE_SIZE;
        let tile_y = (self.player.y + offset_y) / TILE_SIZE;
        if tile_x < 0 || tile_y < 0 || tile_x >= self.width as i32 || tile_y >= self.height as i32 {
            return;
        }
        let index = tile_x as usize + tile_y as usize * self.width;
        let damage = (self.random.next_int(3) + 1) as u16;
        match self.levels[self.current_level].tiles[index] {
            Tile::Tree => {
                let total = self.levels[self.current_level].data[index] + damage;
                if total >= 20 {
                    self.levels[self.current_level].tiles[index] = Tile::Grass;
                    self.levels[self.current_level].data[index] = 0;
                    let wood = (self.random.next_int(3) + 1) as u16;
                    self.player.wood += wood;
                    self.notification = Some((format!("WOOD +{wood}"), 60));
                } else {
                    self.levels[self.current_level].data[index] = total;
                    self.notification = Some((format!("TREE {total}/20"), 30));
                }
            }
            Tile::WoodDoor | Tile::StoneDoor | Tile::ObsidianDoor => {
                self.levels[self.current_level].data[index] ^= 1;
                self.notification = Some(("DOOR TOGGLED".to_owned(), 30));
            }
            Tile::Cactus => {
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    10,
                    Tile::Sand,
                );
            }
            Tile::Rock => {
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    50,
                    Tile::Dirt,
                );
            }
            Tile::HardRock => {
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    200,
                    Tile::Dirt,
                );
            }
            Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall => {
                let replacement = match self.levels[self.current_level].tiles[index] {
                    Tile::WoodWall => Tile::WoodFloor,
                    Tile::StoneWall => Tile::StoneFloor,
                    _ => Tile::ObsidianFloor,
                };
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    100,
                    replacement,
                );
            }
            Tile::IronOre | Tile::GoldOre | Tile::GemOre | Tile::LapisOre | Tile::CloudOre => {
                let replacement = if self.levels[self.current_level].tiles[index] == Tile::CloudOre
                {
                    Tile::Cloud
                } else {
                    Tile::Dirt
                };
                let health = (self.random.next_int(10) * 4 + 20) as u16;
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    health,
                    replacement,
                );
                let _drop_count_roll = self.random.next_int(2);
            }
            _ => {}
        }
    }

    fn use_stairs(&mut self) {
        let tile = self.tile_at_pixel(self.player.x, self.player.y);
        let target = match tile {
            Tile::StairsDown if self.current_level + 1 < self.levels.len() => {
                Some(self.current_level + 1)
            }
            Tile::StairsUp if self.current_level > 0 => Some(self.current_level - 1),
            _ => None,
        };
        if let Some(target) = target {
            self.current_level = target;
            self.notification = Some((format!("ENTERED DEPTH {}", self.levels[target].depth), 90));
        }
    }

    pub fn render(&self, screen: &mut Screen, assets: &Assets) {
        let map_width = self.width as i32 * TILE_SIZE;
        let map_height = self.height as i32 * TILE_SIZE;
        let camera_x = (self.player.x - WIDTH as i32 / 2).clamp(0, map_width - WIDTH as i32);
        let camera_y =
            (self.player.y - (HEIGHT as i32 - 8) / 2).clamp(0, map_height - HEIGHT as i32);
        let first_x = camera_x / TILE_SIZE;
        let first_y = camera_y / TILE_SIZE;
        let last_x = ((camera_x + WIDTH as i32) / TILE_SIZE + 1).min(self.width as i32);
        let last_y = ((camera_y + HEIGHT as i32) / TILE_SIZE + 1).min(self.height as i32);
        let mut lights = vec![(self.player.x - camera_x, self.player.y - camera_y, 5 * 8)];

        for tile_y in first_y..last_y {
            for tile_x in first_x..last_x {
                let tile = self.levels[self.current_level].tiles
                    [tile_x as usize + tile_y as usize * self.width];
                let data = self.levels[self.current_level].data
                    [tile_x as usize + tile_y as usize * self.width];
                let image = assets.tile(tile, data);
                let frame_count = (image.height / 16).max(1);
                let frame = (self.tick as usize / 12) % frame_count;
                for underlay in underlay_layers(tile, data, self.levels[self.current_level].depth)
                    .into_iter()
                    .flatten()
                {
                    let underlay_image = assets.tile(underlay, 0);
                    let underlay_frames = (underlay_image.height / 16).max(1);
                    assets.render_tile(
                        screen,
                        underlay,
                        0,
                        tile_x * TILE_SIZE - camera_x,
                        tile_y * TILE_SIZE - camera_y,
                        (self.tick as usize / 12) % underlay_frames,
                        connection_mask(
                            &self.levels[self.current_level].tiles,
                            self.width,
                            self.height,
                            tile_x as usize,
                            tile_y as usize,
                            underlay,
                        ),
                    );
                }
                let connected = connection_mask(
                    &self.levels[self.current_level].tiles,
                    self.width,
                    self.height,
                    tile_x as usize,
                    tile_y as usize,
                    tile,
                );
                assets.render_tile(
                    screen,
                    tile,
                    data,
                    tile_x * TILE_SIZE - camera_x,
                    tile_y * TILE_SIZE - camera_y,
                    frame,
                    connected,
                );
                let light_radius = tile.light_radius();
                if light_radius > 0 {
                    lights.push((
                        tile_x * TILE_SIZE + 8 - camera_x,
                        tile_y * TILE_SIZE + 8 - camera_y,
                        light_radius * 8,
                    ));
                }
            }
        }

        let (source_x, flip) = match self.player.direction {
            Direction::Down => (0, false),
            Direction::Up => (16, false),
            Direction::Left => (32 + (self.player.walk_distance as usize / 8 % 2) * 16, true),
            Direction::Right => (
                32 + (self.player.walk_distance as usize / 8 % 2) * 16,
                false,
            ),
        };
        screen.blit_region(
            &assets.skin,
            self.player.x - 8 - camera_x,
            self.player.y - 11 - camera_y,
            source_x,
            assets.skin_row,
            16,
            16,
            flip,
        );

        let depth = self.levels[self.current_level].depth;
        let darkness = if depth == 0 {
            surface_darkness(self.day_tick)
        } else {
            176
        };
        screen.darken_with_lights(&lights, darkness);

        render_hud(
            screen,
            assets,
            &self.player,
            self.seed,
            depth,
            self.day_tick,
            self.days,
        );
        if let Some((message, _)) = &self.notification {
            let width = message.chars().count() as i32 * 8 + 4;
            screen.rect((WIDTH as i32 - width) / 2, 21, width, 11, 0x101018);
            screen.centered_text(&assets.font, message, 23);
        }
        if self.inventory_open {
            render_inventory(screen, assets, &self.player);
        }
        if self.paused {
            render_pause(screen, assets, self.pause_selection);
        }
    }
}

fn damage_tile(level: &mut Level, index: usize, damage: u16, health: u16, replacement: Tile) {
    let total = level.data[index].saturating_add(damage);
    if total >= health {
        level.tiles[index] = replacement;
        level.data[index] = 0;
    } else {
        level.data[index] = total;
    }
}

#[allow(clippy::too_many_arguments)]
fn try_queue_natural_spawn(
    level: &mut Level,
    width: usize,
    height: usize,
    player_x: i32,
    player_y: i32,
    day_tick: u32,
    days: u32,
    random: &mut random::JavaRandom,
) {
    if level.pending_spawns.len() >= level.max_mob_count {
        return;
    }
    let skip = spawn::spawn_skip_chance(level.pending_spawns.len(), level.max_mob_count);
    if skip > 0 && random.next_int(skip.min(i32::MAX as usize) as i32) != 0 {
        return;
    }
    for _ in 0..30 {
        let roll = random.next_int(100);
        let x = random.next_int(width as i32) as usize;
        let y = random.next_int(height as i32) as usize;
        let pixel_x = x as i32 * TILE_SIZE + 8;
        let pixel_y = y as i32 * TILE_SIZE + 8;
        let dx = pixel_x - player_x;
        let dy = pixel_y - player_y;
        if dx * dx + dy * dy < 160 * 160 {
            continue;
        }
        let tile = level.tiles[x + y * width];
        let lit = torch_lit(&level.tiles, width, height, x, y);
        if spawn::hostile_allowed(level.depth, day_tick, days, tile, lit) {
            level
                .pending_spawns
                .push(spawn::choose_hostile(level.depth, roll));
            return;
        }
        if spawn::passive_allowed(level.depth, tile) {
            level
                .pending_spawns
                .push(spawn::choose_passive(day_tick >= 48_600, roll));
            return;
        }
    }
}

fn torch_lit(tiles: &[Tile], width: usize, height: usize, x: usize, y: usize) -> bool {
    let left = x.saturating_sub(3);
    let right = (x + 3).min(width - 1);
    let top = y.saturating_sub(3);
    let bottom = (y + 3).min(height - 1);
    (top..=bottom).any(|yy| (left..=right).any(|xx| tiles[xx + yy * width] == Tile::Torch))
}

fn render_hud(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    seed: i64,
    depth: i8,
    day_tick: u32,
    days: u32,
) {
    screen.rect(0, 0, WIDTH as i32, 18, 0x101018);
    screen.text(&assets.font, "HP", 4, 5);
    for index in 0..10 {
        screen.rect(
            23 + index * 5,
            5,
            4,
            7,
            if index < player.health as i32 {
                0xDD3333
            } else {
                0x431818
            },
        );
    }
    screen.text(&assets.font, "ST", 82, 5);
    for index in 0..10 {
        screen.rect(
            101 + index * 5,
            5,
            4,
            7,
            if index < player.stamina as i32 {
                0xE6D84A
            } else {
                0x4A4518
            },
        );
    }
    screen.text(&assets.font, &format!("WOOD {}", player.wood), 158, 5);
    screen.text(
        &assets.font,
        &format!("D{days} {}", time_name(day_tick)),
        220,
        5,
    );
    screen.text(&assets.font, &format!("SEED {seed}"), 4, HEIGHT as i32 - 9);
    screen.text(
        &assets.font,
        &format!("DEPTH {depth}"),
        WIDTH as i32 - 62,
        HEIGHT as i32 - 9,
    );
}

fn surface_darkness(tick: u32) -> u8 {
    let quarter = DAY_LENGTH / 4;
    match tick {
        0..=16_199 => (104_u32.saturating_sub(tick * 104 / quarter)) as u8,
        16_200..=32_399 => 0,
        32_400..=48_599 => ((tick - quarter * 2) * 104 / quarter) as u8,
        _ => 104,
    }
}

fn time_name(tick: u32) -> &'static str {
    match tick / (DAY_LENGTH / 4) {
        0 => "MORN",
        1 => "DAY",
        2 => "EVE",
        _ => "NIGHT",
    }
}

fn render_inventory(screen: &mut Screen, assets: &Assets, player: &Player) {
    screen.rect(58, 42, 172, 108, 0x15151D);
    screen.frame(58, 42, 172, 108, 0xC8C8C8);
    screen.centered_text(&assets.font, "INVENTORY", 50);
    screen.text(
        &assets.font,
        &format!("WOOD              {}", player.wood),
        72,
        76,
    );
    screen.text(&assets.font, "MORE ITEMS ARRIVE IN PHASE 5", 40, 116);
    screen.centered_text(&assets.font, "X OR ESC TO CLOSE", 135);
}

fn render_pause(screen: &mut Screen, assets: &Assets, selection: usize) {
    screen.rect(78, 52, 132, 88, 0x15151D);
    screen.frame(78, 52, 132, 88, 0xC8C8C8);
    screen.centered_text(&assets.font, "PAUSED", 62);
    for (index, label) in ["RETURN TO GAME", "QUIT TO TITLE"].iter().enumerate() {
        let marker = if index == selection { ">" } else { " " };
        screen.text(
            &assets.font,
            &format!("{marker}{label}"),
            86,
            88 + index as i32 * 18,
        );
    }
}

fn find_spawn(tiles: &[Tile], width: usize, height: usize) -> (i32, i32) {
    let center_x = width as i32 / 2;
    let center_y = height as i32 / 2;
    for radius in 0..width as i32 / 2 {
        for y in center_y - radius..=center_y + radius {
            for x in center_x - radius..=center_x + radius {
                if x >= 0
                    && y >= 0
                    && x < width as i32
                    && y < height as i32
                    && matches!(
                        tiles[x as usize + y as usize * width],
                        Tile::Grass
                            | Tile::Sand
                            | Tile::Flower
                            | Tile::Dirt
                            | Tile::Cloud
                            | Tile::ObsidianFloor
                            | Tile::StoneFloor
                            | Tile::WoodFloor
                    )
                {
                    return (x, y);
                }
            }
        }
    }
    (center_x, center_y)
}

fn connection_mask(
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

fn connects(tile: Tile, neighbor: Tile) -> bool {
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

fn underlay_layers(tile: Tile, data: u16, depth: i8) -> [Option<Tile>; 2] {
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

fn find_stair_site(tiles: &[Tile], width: usize, height: usize, origin: usize) -> Option<usize> {
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
mod registry_tests {
    use super::{
        DAY_LENGTH, Level, Tile, World, random::JavaRandom, surface_darkness, time_name,
        try_queue_natural_spawn,
    };

    #[test]
    fn all_2_2_4_tile_ids_round_trip() {
        assert_eq!(Tile::ALL.len(), 59);
        for (id, tile) in Tile::ALL.iter().copied().enumerate() {
            assert_eq!(tile.id(), id as u8);
            assert_eq!(Tile::from_id(id as u8), Some(tile));
        }
        assert_eq!(Tile::from_id(59), None);
        assert_eq!(Tile::Sign.id(), 58);
        assert_eq!(Tile::from_legacy_id(1), Some((Tile::Rock, 0)));
        assert_eq!(
            Tile::from_legacy_id(44),
            Some((Tile::Torch, Tile::Grass.id() as u16))
        );
        assert_eq!(Tile::from_legacy_id(123), Some((Tile::ObsidianDoor, 0)));
        assert_eq!(Tile::from_legacy_id(999), None);
        for legacy_id in [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24,
            30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
            52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 100, 101, 102, 103, 104, 105,
            107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 119, 120, 121, 122, 123, 127,
        ] {
            assert!(
                Tile::from_legacy_id(legacy_id).is_some(),
                "missing legacy tile ID {legacy_id}"
            );
        }
        assert!(Tile::WoodDoor.solid(0));
        assert!(!Tile::WoodDoor.solid(1));
        assert_eq!(Tile::Lava.light_radius(), 6);
        assert_eq!(Tile::Torch.light_radius(), 5);
    }

    #[test]
    fn six_levels_are_linked_by_matching_stairs() {
        let world = World::new_with_options(0x100, super::WorldSpec::default(), 1);
        assert_eq!(
            world
                .levels
                .iter()
                .map(|level| level.depth)
                .collect::<Vec<_>>(),
            vec![1, 0, -1, -2, -3, -4]
        );
        for upper in 0..world.levels.len() - 1 {
            let mut links = 0;
            for (index, tile) in world.levels[upper].tiles.iter().enumerate() {
                if *tile == Tile::StairsDown {
                    assert_eq!(world.levels[upper + 1].tiles[index], Tile::StairsUp);
                    links += 1;
                }
            }
            assert!(
                links > 0,
                "depth {} has no lower link",
                world.levels[upper].depth
            );
        }
        assert!(world.levels[0].tiles.contains(&Tile::WoodFloor));
        assert!(world.levels[1].tiles.contains(&Tile::WoodWall));
        assert!(
            world.levels[2..5]
                .iter()
                .any(|level| level.tiles.contains(&Tile::StoneFloor))
        );
        assert!(world.levels[5].tiles.contains(&Tile::BossFloor));
        assert!(world.levels[5].tiles.contains(&Tile::OrnateObsidian));
    }

    #[test]
    fn day_cycle_uses_the_2_2_4_quarters() {
        assert_eq!(DAY_LENGTH, 64_800);
        assert_eq!(time_name(0), "MORN");
        assert_eq!(time_name(16_200), "DAY");
        assert_eq!(time_name(32_400), "EVE");
        assert_eq!(time_name(48_600), "NIGHT");
        assert!(surface_darkness(0) > surface_darkness(8_100));
        assert_eq!(surface_darkness(16_200), 0);
        assert!(surface_darkness(48_600) > 0);
    }

    #[test]
    fn natural_spawn_pipeline_queues_eligible_requests() {
        let mut level = Level {
            depth: -1,
            tiles: vec![Tile::Dirt; 128 * 128],
            data: vec![0; 128 * 128],
            max_mob_count: 1,
            pending_spawns: Vec::new(),
        };
        let mut random = JavaRandom::new(7);
        try_queue_natural_spawn(&mut level, 128, 128, 8, 8, 0, 1, &mut random);
        assert_eq!(level.pending_spawns.len(), 1);
        try_queue_natural_spawn(&mut level, 128, 128, 8, 8, 0, 1, &mut random);
        assert_eq!(level.pending_spawns.len(), 1);
    }
}
