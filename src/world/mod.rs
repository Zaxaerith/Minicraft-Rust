mod generation;
mod random;
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

    fn solid(self) -> bool {
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
                | Self::LavaBrick
                | Self::HardRock
                | Self::InfiniteFall
                | Self::RawStone
                | Self::RawObsidian
                | Self::OrnateStone
                | Self::OrnateObsidian
                | Self::WoodWall
                | Self::StoneWall
                | Self::ObsidianWall
                | Self::BossWall
                | Self::WoodDoor
                | Self::StoneDoor
                | Self::ObsidianDoor
                | Self::BossDoor
                | Self::WoodFence
                | Self::StoneFence
                | Self::ObsidianFence
        )
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
    pub fn new(seed: i64) -> Self {
        Self::new_with_spec(seed, WorldSpec::default())
    }

    pub fn new_with_spec(seed: i64, spec: WorldSpec) -> Self {
        let size = spec.size;
        let mut levels: Vec<Level> = [1, 0, -1, -2, -3, -4]
            .into_iter()
            .map(|depth| Level {
                depth,
                tiles: generation::level(size, size, depth, seed, spec),
                data: vec![0; size * size],
            })
            .collect();
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
            for index in stairs {
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
                levels[upper + 1].tiles[index] = Tile::StairsUp;
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
            random: random::JavaRandom::new(seed ^ 0x5EED_224),
        }
    }

    pub fn new_at_depth(seed: i64, depth: i8) -> Result<Self, String> {
        let mut world = Self::new(seed);
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
            .all(|(offset_x, offset_y)| !self.tile_at_pixel(x + offset_x, y + offset_y).solid())
    }

    fn tile_at_pixel(&self, x: i32, y: i32) -> Tile {
        let tile_x = (x / TILE_SIZE).clamp(0, self.width as i32 - 1) as usize;
        let tile_y = (y / TILE_SIZE).clamp(0, self.height as i32 - 1) as usize;
        self.levels[self.current_level].tiles[tile_x + tile_y * self.width]
    }

    fn attack(&mut self) {
        if self.player.stamina == 0 {
            self.notification = Some(("TOO EXHAUSTED".to_owned(), 60));
            return;
        }
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
        if self.levels[self.current_level].tiles[index] == Tile::Tree {
            self.levels[self.current_level].tiles[index] = Tile::Grass;
            self.player.wood += 1;
            self.player.stamina -= 1;
            self.notification = Some(("WOOD +1".to_owned(), 60));
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

        for tile_y in first_y..last_y {
            for tile_x in first_x..last_x {
                let tile = self.levels[self.current_level].tiles
                    [tile_x as usize + tile_y as usize * self.width];
                let data = self.levels[self.current_level].data
                    [tile_x as usize + tile_y as usize * self.width];
                let image = assets.tile(tile, data);
                let frame_count = (image.height / 16).max(1);
                let frame = (self.tick as usize / 12) % frame_count;
                screen.blit_region(
                    image,
                    tile_x * TILE_SIZE - camera_x,
                    tile_y * TILE_SIZE - camera_y,
                    0,
                    frame * 16,
                    16.min(image.width),
                    16.min(image.height),
                    false,
                );
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
        screen.darken_outside(
            self.player.x - camera_x,
            self.player.y - camera_y,
            if depth == 0 { 72 } else { 58 },
            darkness,
        );

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
    use super::{DAY_LENGTH, Tile, World, surface_darkness, time_name};

    #[test]
    fn all_2_2_4_tile_ids_round_trip() {
        assert_eq!(Tile::ALL.len(), 59);
        for (id, tile) in Tile::ALL.iter().copied().enumerate() {
            assert_eq!(tile.id(), id as u8);
            assert_eq!(Tile::from_id(id as u8), Some(tile));
        }
        assert_eq!(Tile::from_id(59), None);
        assert_eq!(Tile::Sign.id(), 58);
    }

    #[test]
    fn six_levels_are_linked_by_matching_stairs() {
        let world = World::new(0x100);
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
}
