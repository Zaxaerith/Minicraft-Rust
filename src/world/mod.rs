mod entity;
mod generation;
mod java_save;
mod random;
pub mod spawn;
mod structure;
mod tile_behavior;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    assets::Assets,
    audio::SoundEffect,
    content::{Book, ProgressEvent, ProgressState},
    gfx::{HEIGHT, Screen, WIDTH},
    input::Input,
    item::{
        ANVIL_RECIPES, ANVIL_TOOL_RECIPES, ArmorKind, ENCHANTER_RECIPES, FURNACE_RECIPES,
        HAND_RECIPES, Inventory, ItemId, ItemStack, LOOM_RECIPES, OVEN_RECIPES, PotionKind,
        ToolItem, ToolKind, ToolTier, WORKBENCH_STATION_RECIPES, WORKBENCH_TOOL_RECIPES,
    },
};

pub use entity::FurnitureKind;
use entity::{EntityArena, EntityKind};
pub use java_save::{import_java_save, probe_java_save};

const TILE_SIZE: i32 = 16;
const DAY_LENGTH: u32 = 64_800;
const MAX_STAT: u8 = 10;
const MAX_HEALTH: u8 = 20;
const MAX_HUNGER_TICKS: i16 = 400;
const HUNGER_STAMINA_STEPS: [i16; 3] = [10, 7, 5];
const HUNGER_TICK_INTERVALS: [u64; 3] = [120, 30, 10];
const HUNGER_MOVE_STEPS: [u8; 3] = [8, 3, 1];
const STARVATION_HEALTH_FLOORS: [u8; 3] = [5, 3, 0];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Hardcore,
    Score,
}

impl GameMode {
    pub const ALL: [Self; 4] = [Self::Survival, Self::Creative, Self::Hardcore, Self::Score];

    pub const fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Creative,
            2 => Self::Hardcore,
            3 => Self::Score,
            _ => Self::Survival,
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlayOptions {
    pub difficulty: usize,
    pub mode: GameMode,
    pub score_minutes: usize,
    pub tutorials: bool,
    pub quests: bool,
    pub show_quests: bool,
    pub custom_skin: bool,
}

impl PlayOptions {
    #[cfg(test)]
    pub const fn survival(difficulty: usize) -> Self {
        Self {
            difficulty,
            mode: GameMode::Survival,
            score_minutes: 20,
            tutorials: false,
            quests: false,
            show_quests: true,
            custom_skin: false,
        }
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Direction {
    Down,
    Up,
    Left,
    Right,
}

fn default_direction() -> Direction {
    Direction::Down
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ActiveItem {
    Stack(ItemId),
    Tool(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreativeEntry {
    Stack(ItemId),
    Tool(ToolItem),
}

#[derive(Serialize, Deserialize)]
struct Player {
    x: i32,
    y: i32,
    direction: Direction,
    walk_distance: u32,
    attack_time: u8,
    #[serde(default = "default_direction")]
    attack_direction: Direction,
    #[serde(default)]
    attack_item: Option<ActiveItem>,
    health: u8,
    max_health: u8,
    stamina: u8,
    hunger: u8,
    armor: u8,
    armor_kind: Option<ArmorKind>,
    armor_damage_buffer: u8,
    hurt_time: u8,
    stamina_recharge: u8,
    stamina_recharge_delay: u8,
    hunger_stamina_count: i16,
    hunger_ticks: i16,
    step_count: u8,
    hunger_charge_delay: u16,
    hunger_starve_delay: u8,
    potion_effects: [u16; PotionKind::ALL.len()],
    regen_tick: u8,
    fishing_level: Option<u8>,
    fishing_ticks: u16,
    watering_content: u16,
    clothing: ItemId,
    inventory: Inventory,
    active_item: Option<ActiveItem>,
}

#[derive(Serialize, Deserialize)]
struct Level {
    depth: i8,
    tiles: Vec<Tile>,
    data: Vec<u16>,
    max_mob_count: usize,
    pending_spawns: Vec<spawn::SpawnIntent>,
    entities: EntityArena,
}

#[derive(Serialize, Deserialize)]
struct SignEditor {
    level: usize,
    tile: usize,
    text: String,
}

pub enum WorldAction {
    None,
    SaveGame,
    ReturnToTitle,
    QuitWithoutSaving,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum PausePage {
    #[default]
    Main,
    Options,
    Achievements,
    Quests,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PauseAction {
    Return,
    Options,
    Achievements,
    Quests,
    Save,
    MainMenu,
}

fn pause_actions(quests_enabled: bool) -> Vec<PauseAction> {
    let mut actions = vec![
        PauseAction::Return,
        PauseAction::Options,
        PauseAction::Achievements,
    ];
    if quests_enabled {
        actions.push(PauseAction::Quests);
    }
    actions.extend([PauseAction::Save, PauseAction::MainMenu]);
    actions
}

#[derive(Serialize, Deserialize)]
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
    difficulty: usize,
    mode: GameMode,
    score: u32,
    score_ticks: u32,
    score_multiplier: u8,
    multiplier_ticks: u16,
    game_over: bool,
    story_complete: bool,
    sleeping: u16,
    tutorials_enabled: bool,
    quests_enabled: bool,
    show_quests: bool,
    progress: ProgressState,
    signs: Vec<HashMap<usize, String>>,
    sign_editor: Option<SignEditor>,
    book_open: Option<(Book, usize)>,
    paused: bool,
    pause_selection: usize,
    #[serde(default)]
    pause_page: PausePage,
    #[serde(default)]
    pause_confirm: bool,
    inventory_open: bool,
    inventory_selection: usize,
    inventory_item_selection: usize,
    inventory_pane: usize,
    crafting_station: Option<FurnitureKind>,
    #[serde(default)]
    personal_crafting: bool,
    notification: Option<(String, u16)>,
    air_wizard_defeated: bool,
    obsidian_knight_defeated: bool,
    random: random::JavaRandom,
    #[serde(skip)]
    sound_events: Vec<SoundEffect>,
}

#[derive(Serialize)]
struct WorldSaveRef<'a> {
    format: u32,
    game_version: &'static str,
    world: &'a World,
}

#[derive(Deserialize)]
struct WorldSaveOwned {
    format: u32,
    game_version: String,
    world: World,
}

impl World {
    pub fn to_save_string(&self) -> Result<String, String> {
        serde_json::to_string(&WorldSaveRef {
            format: 2,
            game_version: "2.2.4-rust",
            world: self,
        })
        .map_err(|error| format!("cannot serialize world: {error}"))
    }

    pub fn from_save_string(text: &str) -> Result<Self, String> {
        let save: WorldSaveOwned = serde_json::from_str(text)
            .map_err(|error| format!("cannot parse world state: {error}"))?;
        if save.format != 2 {
            return Err(format!("unsupported Rust world format {}", save.format));
        }
        if save.game_version != "2.2.4-rust" {
            return Err(format!(
                "unsupported Rust game version {}",
                save.game_version
            ));
        }
        let mut world = save.world;
        world.validate_save()?;
        let overflow = world.player.inventory.set_capacity(27);
        for stack in overflow {
            world.levels[world.current_level].entities.spawn_item(
                stack,
                world.player.x,
                world.player.y,
            );
        }
        if matches!(world.player.active_item, Some(ActiveItem::Stack(item)) if world.player.inventory.count(item) == 0)
        {
            world.player.active_item = None;
        }
        Ok(world)
    }

    pub fn autosave_due(&self) -> bool {
        self.tick > 0 && self.tick.is_multiple_of(1_800)
    }

    pub fn save_tick(&self) -> u64 {
        self.tick
    }

    pub fn take_sound_events(&mut self) -> Vec<SoundEffect> {
        std::mem::take(&mut self.sound_events)
    }

    /// Menu feedback is sampled on the render/input frame so it does not wait for
    /// the next fixed 60 Hz world tick. Gameplay sounds remain tick-driven.
    pub fn immediate_menu_sound(&self, input: &Input) -> Option<SoundEffect> {
        if self.paused {
            if input.select {
                return Some(SoundEffect::Confirm);
            }
            if input.up_pressed
                || input.down_pressed
                || (self.pause_page == PausePage::Options
                    && (input.left_pressed || input.right_pressed))
            {
                return Some(SoundEffect::Select);
            }
        } else if self.inventory_open
            && (input.up_pressed || input.down_pressed || input.left_pressed || input.right_pressed)
        {
            return Some(SoundEffect::Select);
        }
        None
    }

    fn sound(&mut self, effect: SoundEffect) {
        self.sound_events.push(effect);
    }

    fn validate_save(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 || self.width > 512 || self.height > 512 {
            return Err(format!(
                "invalid saved world dimensions {}x{}",
                self.width, self.height
            ));
        }
        if self.levels.is_empty() || self.current_level >= self.levels.len() {
            return Err("saved world has no valid current level".to_owned());
        }
        let expected = self
            .width
            .checked_mul(self.height)
            .ok_or_else(|| "saved world dimensions overflow".to_owned())?;
        if self
            .levels
            .iter()
            .any(|level| level.tiles.len() != expected || level.data.len() != expected)
        {
            return Err("saved level storage does not match world dimensions".to_owned());
        }
        if self.signs.len() != self.levels.len() {
            return Err("saved sign storage does not match level count".to_owned());
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn new_with_options(seed: i64, spec: WorldSpec, difficulty: usize) -> Self {
        Self::new_with_play_options(seed, spec, PlayOptions::survival(difficulty))
    }

    pub fn new_with_play_options(seed: i64, spec: WorldSpec, options: PlayOptions) -> Self {
        let difficulty = options.difficulty.min(2);
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
                    entities: EntityArena::default(),
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
        for level in &mut levels {
            structure::finalize_entities(level, size, size);
        }
        let current_level = 1;
        let (spawn_x, spawn_y) = find_spawn(&levels[current_level].tiles, size, size);
        // Java Player uses a 27-slot inventory in every mode. Creative mode exposes
        // the unlimited item catalogue as a separate pane rather than preloading it.
        let inventory = Inventory::new(27);
        let sign_levels = levels.len();
        let mut progress = ProgressState::load().expect("bundled progression data must be valid");
        if options.custom_skin {
            progress.unlock_achievement("minicraft.achievement.skin");
        }
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
                attack_time: 0,
                attack_direction: Direction::Down,
                attack_item: None,
                health: 10,
                max_health: 10,
                stamina: 10,
                hunger: 10,
                armor: 0,
                armor_kind: None,
                armor_damage_buffer: 0,
                hurt_time: 0,
                stamina_recharge: 0,
                stamina_recharge_delay: 0,
                hunger_stamina_count: HUNGER_STAMINA_STEPS[difficulty.min(2)],
                hunger_ticks: MAX_HUNGER_TICKS,
                step_count: 0,
                hunger_charge_delay: 0,
                hunger_starve_delay: 0,
                potion_effects: [0; PotionKind::ALL.len()],
                regen_tick: 0,
                fishing_level: None,
                fishing_ticks: 0,
                watering_content: 0,
                clothing: ItemId::RegularClothes,
                inventory,
                active_item: None,
            },
            seed,
            tick: 0,
            day_tick: 0,
            days: 1,
            difficulty,
            mode: options.mode,
            score: 0,
            score_ticks: (match options.score_minutes {
                10 | 20 | 40 | 60 | 120 => options.score_minutes,
                _ => 20,
            } * 60
                * 60) as u32,
            score_multiplier: 1,
            multiplier_ticks: 300,
            game_over: false,
            story_complete: false,
            sleeping: 0,
            tutorials_enabled: options.tutorials,
            quests_enabled: options.quests,
            show_quests: options.show_quests,
            progress,
            signs: (0..sign_levels).map(|_| HashMap::new()).collect(),
            sign_editor: None,
            book_open: None,
            paused: false,
            pause_selection: 0,
            pause_page: PausePage::Main,
            pause_confirm: false,
            inventory_open: false,
            inventory_selection: 0,
            inventory_item_selection: 0,
            inventory_pane: 0,
            crafting_station: None,
            personal_crafting: false,
            notification: None,
            air_wizard_defeated: false,
            obsidian_knight_defeated: false,
            random: random::JavaRandom::new(seed ^ 0x05EE_D224),
            sound_events: Vec::new(),
        }
    }

    pub fn new_at_depth_with_options(
        seed: i64,
        depth: i8,
        spec: WorldSpec,
        options: PlayOptions,
    ) -> Result<Self, String> {
        let mut world = Self::new_with_play_options(seed, spec, options);
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

    /// Populates a compact deterministic scene for the headless renderer.
    pub fn populate_entity_preview(&mut self) {
        let center_x = self.player.x;
        let center_y = self.player.y;
        let level = &mut self.levels[self.current_level];
        for (index, mob) in spawn::NaturalMob::ALL.into_iter().enumerate() {
            let column = index as i32 % 3 - 1;
            let row = index as i32 / 3 - 1;
            level
                .entities
                .spawn_mob(mob, center_x + column * 32, center_y + row * 28);
        }
        level.entities.spawn_item(
            ItemStack::new(ItemId::Wood, 2),
            center_x + 20,
            center_y + 14,
        );
        level
            .entities
            .spawn_furniture(FurnitureKind::Workbench, center_x + 64, center_y + 32);
    }

    /// Opens a deterministic workbench/inventory scene for visual regression.
    pub fn populate_workbench_preview(&mut self) {
        self.player.inventory.add(ItemId::Wood, 20);
        self.player.inventory.add(ItemId::Stone, 10);
        self.player.inventory.add(ItemId::Coal, 4);
        if let Some(index) = WORKBENCH_TOOL_RECIPES[0].craft(&mut self.player.inventory) {
            self.player.active_item = Some(ActiveItem::Tool(index));
        }
        self.inventory_open = true;
        self.crafting_station = Some(FurnitureKind::Workbench);
        self.inventory_pane = 1;
        self.inventory_selection = WORKBENCH_STATION_RECIPES.len() + 4;
    }

    /// Opens a deterministic food inventory and oven scene for visual regression.
    pub fn populate_food_preview(&mut self) {
        for item in [
            ItemId::Apple,
            ItemId::RawFish,
            ItemId::Bread,
            ItemId::CookedFish,
            ItemId::GoldenApple,
        ] {
            self.player.inventory.add(item, 2);
        }
        self.player.health = 8;
        self.player.stamina = 8;
        self.player.hunger = 6;
        self.player.active_item = Some(ActiveItem::Stack(ItemId::CookedFish));
        self.inventory_open = true;
        self.crafting_station = Some(FurnitureKind::Oven);
        self.inventory_pane = 0;
        self.inventory_item_selection = 3;
        self.inventory_selection = 2;
        self.notification = Some(("FOOD SURVIVAL PREVIEW".to_owned(), 150));
    }

    /// Opens the Java-style player inventory for visual regression.
    pub fn populate_inventory_preview(&mut self) {
        self.player.inventory.add(ItemId::Wood, 20);
        self.player.inventory.add(ItemId::Stone, 10);
        self.player.inventory.add(ItemId::Coal, 4);
        self.player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood));
        self.inventory_open = true;
        self.crafting_station = None;
        self.inventory_pane = 0;
        self.inventory_item_selection = 1;
    }

    /// Opens the Java personal crafting display (Z / Shift+E) for visual regression.
    pub fn populate_personal_crafting_preview(&mut self) {
        self.player.inventory.add(ItemId::Wood, 20);
        self.player.inventory.add(ItemId::Coal, 4);
        self.inventory_open = true;
        self.personal_crafting = true;
        self.crafting_station = None;
        self.inventory_pane = 1;
        self.inventory_selection = 1;
    }

    /// Opens the Java-style sliding creative catalogue for visual regression.
    pub fn populate_creative_inventory_preview(&mut self) {
        self.mode = GameMode::Creative;
        self.player.inventory = Inventory::new(27);
        self.player.inventory.add(ItemId::Wood, 20);
        self.player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood));
        self.inventory_open = true;
        self.crafting_station = None;
        self.inventory_pane = 1;
        self.inventory_item_selection = 0;
        self.inventory_selection = 3;
    }

    /// Renders all currently ported crafting furniture from local entity art.
    pub fn populate_stations_preview(&mut self) {
        let center_x = self.player.x;
        let center_y = self.player.y;
        for (index, kind) in FurnitureKind::ALL.into_iter().enumerate() {
            let column = index as i32 % 3 - 1;
            let row = index as i32 / 3;
            self.levels[self.current_level].entities.spawn_furniture(
                kind,
                center_x + column * 32,
                center_y + 24 + row * 28,
            );
        }
    }

    pub fn populate_score_preview(&mut self) {
        self.mode = GameMode::Score;
        self.score = 12_450;
        self.score_multiplier = 7;
        self.score_ticks = 7 * 60 + 42;
        self.notification = Some(("SCORE MODE PREVIEW".to_owned(), 150));
    }

    pub fn populate_book_preview(&mut self) {
        self.book_open = Some((Book::Antidious, 0));
    }

    pub fn populate_sign_preview(&mut self) {
        self.sign_editor = Some(SignEditor {
            level: self.current_level,
            tile: 0,
            text: "WELCOME TO MINICRAFT".to_owned(),
        });
    }

    pub fn populate_progress_preview(&mut self) {
        self.tutorials_enabled = true;
        self.quests_enabled = true;
        self.show_quests = true;
        self.player.inventory.add(ItemId::Wood, 1);
        self.player.potion_effects[PotionKind::Speed.id()] = 4_200;
        self.player.potion_effects[PotionKind::Light.id()] = 2_400;
        self.update_progress(ProgressEvent::InventoryChanged);
        self.notification = Some(("DATA-DRIVEN PROGRESS ACTIVE".to_owned(), 150));
    }

    pub fn populate_boss_preview(&mut self) {
        let species = if self.levels[self.current_level].depth == -4 {
            spawn::NaturalMob::ObsidianKnight
        } else {
            spawn::NaturalMob::AirWizard
        };
        if !self.levels[self.current_level].entities.has_mob(species) {
            self.levels[self.current_level].entities.spawn_mob(
                species,
                self.player.x + 40,
                self.player.y,
            );
        }
        self.notification = None;
    }

    pub fn populate_pause_preview(&mut self) {
        self.paused = true;
        self.pause_page = PausePage::Main;
        self.pause_selection = 1;
        self.pause_confirm = false;
        self.notification = None;
    }

    pub fn tick(&mut self, input: &Input) -> WorldAction {
        if self.tick_sign_editor(input) || self.tick_book(input) {
            return WorldAction::None;
        }
        if self.game_over {
            return if input.select || input.exit {
                WorldAction::ReturnToTitle
            } else {
                WorldAction::None
            };
        }
        if self.sleeping > 0 {
            self.sleeping -= 1;
            if self.sleeping == 0 {
                self.day_tick = 0;
                self.player.stamina = MAX_STAT;
                self.player.health = self
                    .player
                    .health
                    .saturating_add(2)
                    .min(self.player.max_health);
            }
            return WorldAction::None;
        }
        if input.exit {
            if self.inventory_open {
                self.inventory_open = false;
                self.personal_crafting = false;
                self.crafting_station = None;
            } else if self.paused {
                if self.pause_confirm {
                    self.pause_confirm = false;
                    self.pause_selection = pause_actions(self.quests_enabled).len() - 1;
                } else if self.pause_page != PausePage::Main {
                    self.pause_page = PausePage::Main;
                    self.pause_selection = 0;
                } else {
                    self.paused = false;
                }
            } else {
                self.paused = true;
                self.pause_page = PausePage::Main;
                self.pause_selection = 0;
                self.pause_confirm = false;
            }
            return WorldAction::None;
        }
        if self.paused {
            if self.pause_confirm {
                if input.up_pressed || input.down_pressed {
                    self.pause_selection = usize::from(self.pause_selection == 0);
                }
                if input.select {
                    if self.pause_selection == 0 {
                        self.pause_confirm = false;
                        self.pause_selection = pause_actions(self.quests_enabled).len() - 1;
                    } else {
                        self.paused = false;
                        self.pause_confirm = false;
                        return WorldAction::QuitWithoutSaving;
                    }
                }
                return WorldAction::None;
            }
            let count = match self.pause_page {
                PausePage::Main => pause_actions(self.quests_enabled).len(),
                PausePage::Options => 5,
                PausePage::Achievements | PausePage::Quests => 1,
            };
            if input.up_pressed {
                self.pause_selection = self.pause_selection.checked_sub(1).unwrap_or(count - 1);
            }
            if input.down_pressed {
                self.pause_selection = (self.pause_selection + 1) % count;
            }
            if self.pause_page == PausePage::Options {
                let direction = i32::from(input.right_pressed) - i32::from(input.left_pressed);
                match self.pause_selection {
                    0 if direction != 0 => {
                        self.difficulty = wrap_index(self.difficulty, direction, 3);
                    }
                    1 if direction != 0 => self.show_quests = !self.show_quests,
                    2 if direction != 0 => self.tutorials_enabled = !self.tutorials_enabled,
                    3 if direction != 0 => self.quests_enabled = !self.quests_enabled,
                    _ => {}
                }
            }
            if input.select {
                match self.pause_page {
                    PausePage::Main => {
                        match pause_actions(self.quests_enabled)[self.pause_selection] {
                            PauseAction::Return => self.paused = false,
                            PauseAction::Options => {
                                self.pause_page = PausePage::Options;
                                self.pause_selection = 0;
                            }
                            PauseAction::Achievements => {
                                self.pause_page = PausePage::Achievements;
                                self.pause_selection = 0;
                            }
                            PauseAction::Quests => {
                                self.pause_page = PausePage::Quests;
                                self.pause_selection = 0;
                            }
                            PauseAction::Save => {
                                self.paused = false;
                                return WorldAction::SaveGame;
                            }
                            PauseAction::MainMenu => {
                                self.pause_confirm = true;
                                self.pause_selection = 0;
                            }
                        }
                    }
                    PausePage::Options => match self.pause_selection {
                        1 => self.show_quests = !self.show_quests,
                        2 => self.tutorials_enabled = !self.tutorials_enabled,
                        3 => self.quests_enabled = !self.quests_enabled,
                        4 => {
                            self.pause_page = PausePage::Main;
                            self.pause_selection = 1;
                        }
                        _ => {}
                    },
                    PausePage::Achievements | PausePage::Quests => {
                        self.pause_page = PausePage::Main;
                        self.pause_selection = 0;
                    }
                }
            }
            return WorldAction::None;
        }
        if input.craft {
            if self.inventory_open && self.personal_crafting {
                self.inventory_open = false;
                self.personal_crafting = false;
            } else if self.inventory_open && self.crafting_station.is_none() {
                // Make the Java personal-crafting display reachable directly from
                // the already-open player inventory instead of closing the menu.
                self.personal_crafting = true;
                self.inventory_selection = 0;
            } else if !self.inventory_open {
                self.inventory_open = true;
                self.personal_crafting = true;
                self.player.active_item = None;
                self.crafting_station = None;
                self.inventory_selection = 0;
            }
        } else if input.menu {
            self.inventory_open = !self.inventory_open;
            self.personal_crafting = false;
            if self.inventory_open {
                self.player.active_item = None;
                self.crafting_station = None;
                self.inventory_pane = usize::from(
                    self.mode == GameMode::Creative && self.player.inventory.used_slots() == 0,
                );
            }
        }
        if self.inventory_open {
            if self.personal_crafting || self.crafting_station.is_some() {
                self.inventory_pane = 1;
                let count = self.crafting_recipe_count();
                if input.up_pressed {
                    self.inventory_selection = self
                        .inventory_selection
                        .checked_sub(1)
                        .unwrap_or(count.saturating_sub(1));
                }
                if input.down_pressed {
                    self.inventory_selection = (self.inventory_selection + 1) % count.max(1);
                }
                if (input.select || input.attack) && self.crafting_recipe_count() > 0 {
                    self.craft_selected_recipe();
                }
                return WorldAction::None;
            }

            if self.mode == GameMode::Creative {
                if input.left_pressed {
                    self.inventory_pane = 0;
                }
                if input.right_pressed {
                    self.inventory_pane = 1;
                }
            } else {
                self.inventory_pane = 0;
            }

            if self.inventory_pane == 1 {
                let entries = creative_entries();
                if input.up_pressed {
                    self.inventory_selection = self
                        .inventory_selection
                        .checked_sub(1)
                        .unwrap_or(entries.len().saturating_sub(1));
                }
                if input.down_pressed {
                    self.inventory_selection =
                        (self.inventory_selection + 1) % entries.len().max(1);
                }
                if input.select
                    && let Some(entry) = entries.get(self.inventory_selection).copied()
                {
                    let inserted = match entry {
                        CreativeEntry::Stack(item) => self.player.inventory.add(item, 1) == 0,
                        CreativeEntry::Tool(tool) => self.player.inventory.add_tool(tool).is_some(),
                    };
                    let _ = inserted;
                }
                return WorldAction::None;
            }

            if input.up_pressed {
                let count = self.player.inventory.used_slots();
                self.inventory_item_selection = self
                    .inventory_item_selection
                    .checked_sub(1)
                    .unwrap_or(count.saturating_sub(1));
            }
            if input.down_pressed {
                let count = self.player.inventory.used_slots();
                self.inventory_item_selection = (self.inventory_item_selection + 1) % count.max(1);
            }
            if input.attack || input.select {
                let stack_count = self.player.inventory.slots().len();
                self.player.active_item = if self.inventory_item_selection < stack_count {
                    self.player
                        .inventory
                        .slots()
                        .get(self.inventory_item_selection)
                        .map(|stack| ActiveItem::Stack(stack.item))
                } else {
                    let tool_index = self.inventory_item_selection - stack_count;
                    self.player
                        .inventory
                        .tools()
                        .get(tool_index)
                        .map(|_| ActiveItem::Tool(tool_index))
                };
                if self.player.active_item.is_some() {
                    self.inventory_open = false;
                    self.personal_crafting = false;
                }
            }
            return WorldAction::None;
        }

        self.tick = self.tick.wrapping_add(1);
        if self.mode == GameMode::Score {
            if self.score_ticks == 0 {
                self.game_over = true;
                return WorldAction::None;
            }
            self.score_ticks -= 1;
            if self.score_ticks == 0 {
                self.game_over = true;
                return WorldAction::None;
            }
            if self.score_multiplier > 1 {
                self.multiplier_ticks = self.multiplier_ticks.saturating_sub(1);
                if self.multiplier_ticks == 0 {
                    self.score_multiplier = 1;
                    self.multiplier_ticks = 300;
                }
            }
        }
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
        let time_slowed = self.effect_active(PotionKind::Time);
        let outcome = {
            let level = &mut self.levels[self.current_level];
            for intent in std::mem::take(&mut level.pending_spawns) {
                level.entities.spawn_mob(intent.kind, intent.x, intent.y);
            }
            level.entities.tick(
                &mut level.tiles,
                &level.data,
                self.width,
                self.height,
                self.player.x,
                self.player.y,
                time_slowed,
                self.mode == GameMode::Creative,
                &mut self.random,
            )
        };
        self.player.hurt_time = self.player.hurt_time.saturating_sub(1);
        self.player.attack_time = self.player.attack_time.saturating_sub(1);
        if self.player.attack_time == 0 {
            self.player.attack_item = None;
        }
        if outcome.player_damage > 0 {
            self.hurt_player(outcome.player_damage, false);
        }
        for (x, y, radius, player_tnt) in outcome.explosions {
            self.apply_explosion(x, y, radius, player_tnt);
        }
        for mob in outcome.defeated_mobs {
            self.handle_defeated_mob(mob);
        }
        if let Some((_, remaining)) = &mut self.notification {
            *remaining = remaining.saturating_sub(1);
            if *remaining == 0 {
                self.notification = None;
            }
        }

        // Java updates potion timers, fishing and survival recharge before it
        // handles this tick's controls. An attack therefore resets recharge to
        // zero for the full following tick instead of immediately gaining one.
        self.tick_potion_effects();
        self.tick_fishing();
        if self.mode != GameMode::Creative {
            self.tick_survival_stats();
        } else {
            self.player.health = self.player.max_health;
            self.player.stamina = MAX_STAT;
            self.player.hunger = MAX_STAT;
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
            let swimming = self.effect_active(PotionKind::Swim);
            let speed = if self.effect_active(PotionKind::Speed) {
                2
            } else {
                1
            };
            if (!on_water || swimming || self.tick.is_multiple_of(2))
                && self.move_player(horizontal * speed, vertical * speed)
            {
                self.player.walk_distance = self.player.walk_distance.wrapping_add(1);
                self.player.step_count = self.player.step_count.saturating_add(1);
            }
        }
        let collected = self.levels[self.current_level].entities.collect_near(
            self.player.x,
            self.player.y,
            &mut self.player.inventory,
        );
        for stack in &collected {
            self.add_score(1, 0);
            if stack.item == ItemId::Gem {
                self.unlock_achievement("minicraft.achievement.find_gem");
            }
        }
        if !collected.is_empty() {
            self.sound(SoundEffect::Pickup);
        }
        if input.attack
            && (self.player.stamina > 0
                || self.effect_active(PotionKind::Energy)
                || self.mode == GameMode::Creative)
        {
            // Java pays one stamina before every attack(), even when the held item
            // then charges an additional item/tile-specific stamina cost.
            if self.mode != GameMode::Creative && !self.effect_active(PotionKind::Energy) {
                self.player.stamina -= 1;
            }
            self.player.stamina_recharge = 0;
            if !self.use_active_self_item() {
                self.attack();
            }
        }
        if input.select {
            self.use_target();
        }
        if input.pickup {
            self.player.attack_time = 5;
            self.player.attack_direction = self.player.direction;
            self.player.attack_item = None;
            self.pickup_target();
        }
        if self.tile_at_pixel(self.player.x, self.player.y) == Tile::Lava
            && self.tick.is_multiple_of(30)
            && !self.effect_active(PotionKind::Lava)
        {
            self.hurt_player(1, false);
        }
        if self.player.health == 0 {
            self.sound(SoundEffect::Death);
            if self.mode == GameMode::Hardcore {
                self.game_over = true;
            } else {
                self.respawn();
            }
        }
        self.update_progress(ProgressEvent::InventoryChanged);
        WorldAction::None
    }

    fn crafting_recipe_count(&self) -> usize {
        match self.crafting_station {
            None => HAND_RECIPES.len(),
            Some(FurnitureKind::Workbench) => {
                WORKBENCH_STATION_RECIPES.len() + WORKBENCH_TOOL_RECIPES.len()
            }
            Some(FurnitureKind::Oven) => OVEN_RECIPES.len(),
            Some(FurnitureKind::Furnace) => FURNACE_RECIPES.len(),
            Some(FurnitureKind::Anvil) => ANVIL_RECIPES.len() + ANVIL_TOOL_RECIPES.len(),
            Some(FurnitureKind::Enchanter) => ENCHANTER_RECIPES.len(),
            Some(FurnitureKind::Loom) => LOOM_RECIPES.len(),
            _ => 0,
        }
    }

    fn craft_selected_recipe(&mut self) {
        let crafted = match self.crafting_station {
            None => {
                let recipe = HAND_RECIPES[self.inventory_selection];
                self.craft_stack(recipe)
            }
            Some(FurnitureKind::Workbench) => {
                if let Some(recipe) = WORKBENCH_STATION_RECIPES.get(self.inventory_selection) {
                    self.craft_stack(*recipe)
                } else {
                    let index = self.inventory_selection - WORKBENCH_STATION_RECIPES.len();
                    self.craft_tool(WORKBENCH_TOOL_RECIPES[index])
                }
            }
            Some(FurnitureKind::Oven) => self.craft_stack(OVEN_RECIPES[self.inventory_selection]),
            Some(FurnitureKind::Furnace) => {
                self.craft_stack(FURNACE_RECIPES[self.inventory_selection])
            }
            Some(FurnitureKind::Anvil) => {
                if let Some(recipe) = ANVIL_RECIPES.get(self.inventory_selection) {
                    self.craft_stack(*recipe)
                } else {
                    let index = self.inventory_selection - ANVIL_RECIPES.len();
                    self.craft_tool(ANVIL_TOOL_RECIPES[index])
                }
            }
            Some(FurnitureKind::Enchanter) => {
                self.craft_stack(ENCHANTER_RECIPES[self.inventory_selection])
            }
            Some(FurnitureKind::Loom) => self.craft_stack(LOOM_RECIPES[self.inventory_selection]),
            _ => return,
        };
        if crafted {
            self.sound(SoundEffect::Craft);
        }
    }

    fn craft_stack(&mut self, recipe: crate::item::Recipe) -> bool {
        let crafted = if self.mode == GameMode::Creative {
            self.player
                .inventory
                .add(recipe.output.item, recipe.output.count)
                == 0
        } else {
            recipe.craft(&mut self.player.inventory)
        };
        if crafted {
            self.crafting_achievement(recipe.output.item);
        }
        crafted
    }

    fn craft_tool(&mut self, recipe: crate::item::ToolRecipe) -> bool {
        let crafted = if self.mode == GameMode::Creative {
            self.player.inventory.add_tool(recipe.output)
        } else {
            recipe.craft(&mut self.player.inventory)
        };
        if let Some(index) = crafted {
            self.player.active_item = Some(ActiveItem::Tool(index));
            if recipe.output.tier != ToolTier::Wood {
                self.unlock_achievement("minicraft.achievement.upgrade");
            }
            if recipe.output.kind == ToolKind::Bow {
                self.unlock_achievement("minicraft.achievement.bow");
            }
            true
        } else {
            false
        }
    }

    fn use_active_self_item(&mut self) -> bool {
        let Some(ActiveItem::Stack(item)) = self.player.active_item else {
            return false;
        };
        if item.food_value().is_some() {
            self.eat_active_food();
            return true;
        }
        if let Some(armor) = item.armor_kind() {
            if self.player.armor_kind.is_none() && self.pay_stamina(9) {
                self.player.armor = armor.durability();
                self.player.armor_kind = Some(armor);
                self.player.armor_damage_buffer = 0;
                self.consume_active_stack(item);
            }
            return true;
        }
        if let Some(potion) = item.potion_kind() {
            if self.apply_potion(potion) {
                self.consume_active_stack(item);
                if self.mode != GameMode::Creative
                    && self.player.inventory.add(ItemId::GlassBottle, 1) != 0
                {
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(ItemId::GlassBottle, 1),
                        self.player.x,
                        self.player.y,
                    );
                }
            }
            return true;
        }
        if item == ItemId::AirTotem {
            if self.levels[self.current_level].depth != 1 {
                self.notification = Some(("USE THIS IN THE SKY".to_owned(), 60));
            } else if self.levels[self.current_level]
                .entities
                .has_mob(spawn::NaturalMob::AirWizard)
            {
                self.notification = Some(("THE AIR WIZARD IS ALREADY ACTIVE".to_owned(), 60));
            } else if self.pay_stamina(2) {
                self.levels[self.current_level].entities.spawn_mob(
                    spawn::NaturalMob::AirWizard,
                    self.player.x + 8,
                    self.player.y + 8,
                );
                self.consume_active_stack(item);
            }
            return true;
        }
        if item == ItemId::ObsidianPoppet {
            let center_x = self.width as i32 / 2;
            let center_y = self.height as i32 / 2;
            let near_center = (self.player.x.div_euclid(TILE_SIZE) - center_x).abs() <= 3
                && (self.player.y.div_euclid(TILE_SIZE) - center_y).abs() <= 3;
            if self.levels[self.current_level].depth != -4 || !near_center {
                self.notification = Some(("USE THIS IN THE DUNGEON BOSS ROOM".to_owned(), 75));
            } else if self.levels[self.current_level]
                .entities
                .has_mob(spawn::NaturalMob::ObsidianKnight)
            {
                self.notification = Some(("THE OBSIDIAN KNIGHT IS ALREADY ACTIVE".to_owned(), 60));
            } else if self.levels[self.current_level]
                .entities
                .has_furniture(FurnitureKind::KnightStatue)
            {
                self.notification = Some(("THE KNIGHT STATUE IS ALREADY HERE".to_owned(), 60));
            } else if self.pay_stamina(2) {
                self.levels[self.current_level].entities.spawn_furniture(
                    FurnitureKind::KnightStatue,
                    center_x * TILE_SIZE + 8,
                    center_y * TILE_SIZE + 8,
                );
                self.consume_active_stack(item);
            }
            return true;
        }
        if item == ItemId::ObsidianHeart {
            if self.player.max_health < MAX_HEALTH {
                self.player.max_health = (self.player.max_health + 5).min(MAX_HEALTH);
                self.player.health = (self.player.health + 5).min(self.player.max_health);
                self.consume_active_stack(item);
            } else {
                self.notification = Some(("HEALTH INCREASE IS AT MAX!".to_owned(), 90));
            }
            return true;
        }
        if matches!(
            item,
            ItemId::RedClothes
                | ItemId::BlueClothes
                | ItemId::GreenClothes
                | ItemId::YellowClothes
                | ItemId::BlackClothes
                | ItemId::OrangeClothes
                | ItemId::PurpleClothes
                | ItemId::CyanClothes
                | ItemId::RegularClothes
        ) {
            if self.player.clothing != item {
                let previous = self.player.clothing;
                self.consume_active_stack(item);
                if self.mode != GameMode::Creative {
                    self.player.inventory.add(previous, 1);
                }
                self.player.clothing = item;
                self.unlock_achievement("minicraft.achievement.clothes");
            }
            return true;
        }
        if matches!(item, ItemId::Book | ItemId::AntidiousBook) {
            self.book_open = Some((
                if item == ItemId::AntidiousBook {
                    Book::Antidious
                } else {
                    Book::GameGuide
                },
                0,
            ));
            return true;
        }
        false
    }

    fn tick_book(&mut self, input: &Input) -> bool {
        let Some((book, page)) = self.book_open.as_mut() else {
            return false;
        };
        let pages = book.pages();
        if input.left_pressed {
            *page = page.saturating_sub(1);
        }
        if input.right_pressed {
            *page = (*page + 1).min(pages.len().saturating_sub(1));
        }
        if input.exit || input.select {
            self.book_open = None;
        }
        true
    }

    fn tick_sign_editor(&mut self, input: &Input) -> bool {
        let Some(editor) = self.sign_editor.as_mut() else {
            return false;
        };
        if input.backspace {
            editor.text.pop();
        }
        for character in &input.text {
            if !character.is_control() && editor.text.chars().count() < 28 {
                editor.text.push(*character);
            }
        }
        if input.exit || input.select {
            let editor = self.sign_editor.take().expect("sign editor is open");
            if editor.text.trim().is_empty() {
                self.signs[editor.level].remove(&editor.tile);
            } else {
                self.signs[editor.level].insert(editor.tile, editor.text.trim().to_owned());
            }
        }
        true
    }

    fn add_score(&mut self, points: u32, multiplier_gain: u8) {
        if self.mode != GameMode::Score {
            return;
        }
        self.score = self
            .score
            .saturating_add(points.saturating_mul(u32::from(self.score_multiplier)));
        if multiplier_gain > 0 {
            self.score_multiplier = self
                .score_multiplier
                .saturating_add(multiplier_gain)
                .min(50);
            self.multiplier_ticks = 300;
        }
    }

    fn handle_defeated_mob(&mut self, species: spawn::NaturalMob) {
        let (score, multiplier) = match species {
            spawn::NaturalMob::AirWizard => (100_000, 0),
            spawn::NaturalMob::ObsidianKnight => (300_000, 0),
            spawn::NaturalMob::Cow | spawn::NaturalMob::Pig | spawn::NaturalMob::Sheep => (15, 0),
            _ => (50, 1),
        };
        self.add_score(score, multiplier);
        match species {
            spawn::NaturalMob::AirWizard => {
                self.sound(SoundEffect::BossDeath);
                self.air_wizard_defeated = true;
                self.unlock_achievement("minicraft.achievement.airwizard");
                self.notification = Some((
                    "THE AIR WIZARD FALLS; THE OBSIDIAN DEPTHS OPEN".to_owned(),
                    180,
                ));
            }
            spawn::NaturalMob::ObsidianKnight => {
                self.sound(SoundEffect::BossDeath);
                self.obsidian_knight_defeated = true;
                self.story_complete = true;
                self.unlock_achievement("minicraft.achievement.obsidianknight");
                self.notification = Some((
                    "THE OBSIDIAN KNIGHT FALLS; MINICRAFT IS RESTORED".to_owned(),
                    240,
                ));
            }
            _ => {}
        }
    }

    fn unlock_achievement(&mut self, id: &str) {
        if self.progress.unlock_achievement(id) {
            let label = id.rsplit('.').next().unwrap_or(id).replace('_', " ");
            self.notification = Some((format!("ACHIEVEMENT: {}", label.to_ascii_uppercase()), 90));
        }
    }

    fn crafting_achievement(&mut self, item: ItemId) {
        let achievement = match item {
            ItemId::Workbench => Some("minicraft.achievement.benchmarking"),
            ItemId::Plank => Some("minicraft.achievement.planks"),
            ItemId::WoodDoor | ItemId::StoneDoor | ItemId::ObsidianDoor => {
                Some("minicraft.achievement.doors")
            }
            _ => None,
        };
        if let Some(id) = achievement {
            self.unlock_achievement(id);
        }
    }

    fn inventory_progress(&self) -> HashMap<String, u16> {
        let mut inventory = HashMap::new();
        for stack in self.player.inventory.slots() {
            *inventory
                .entry(stack.item.display_name().to_ascii_lowercase())
                .or_insert(0) += stack.count;
        }
        for tool in self.player.inventory.tools() {
            let tier = if tool.tier == ToolTier::Rock {
                "stone"
            } else {
                tool.tier.display_name()
            };
            let name = format!(
                "{} {}",
                tier.to_ascii_lowercase(),
                tool.kind.display_name().to_ascii_lowercase()
            );
            *inventory.entry(name).or_insert(0) += 1;
        }
        inventory
    }

    fn update_progress(&mut self, event: ProgressEvent) {
        if !self.tutorials_enabled && !self.quests_enabled {
            return;
        }
        let inventory = self.inventory_progress();
        let update = self.progress.update_filtered(
            &event,
            &inventory,
            self.tutorials_enabled,
            self.quests_enabled,
        );
        for reward in update.rewards {
            self.grant_reward(&reward);
        }
    }

    fn grant_reward(&mut self, reward: &str) {
        let (name, count) = reward
            .rsplit_once('_')
            .and_then(|(name, count)| count.parse::<u16>().ok().map(|count| (name, count)))
            .unwrap_or((reward, 1));
        let normalized = name.replace('_', " ").to_ascii_lowercase();
        let Some(item) = ItemId::ALL.iter().copied().find(|item| {
            item.display_name().to_ascii_lowercase() == normalized
                || item.asset_name().replace('_', " ") == normalized
        }) else {
            return;
        };
        let remainder = self.player.inventory.add(item, count);
        if remainder > 0 {
            self.levels[self.current_level].entities.spawn_item(
                ItemStack::new(item, remainder),
                self.player.x,
                self.player.y,
            );
        }
    }

    fn pickup_target(&mut self) {
        let (offset_x, offset_y) = match self.player.direction {
            Direction::Down => (0, 12),
            Direction::Up => (0, -12),
            Direction::Left => (-12, 0),
            Direction::Right => (12, 0),
        };
        let creative = self.mode == GameMode::Creative;
        let picked = self.levels[self.current_level]
            .entities
            .pickup_furniture_near(
                self.player.x + offset_x,
                self.player.y + offset_y,
                14,
                creative,
            );
        let Some(kind) = picked else {
            return;
        };
        let Some(item) = item_for_furniture(kind) else {
            return;
        };
        if creative || self.player.inventory.add(item, 1) == 0 {
            self.player.active_item = Some(ActiveItem::Stack(item));
        } else {
            self.levels[self.current_level].entities.spawn_item(
                ItemStack::new(item, 1),
                self.player.x,
                self.player.y,
            );
        }
        self.sound(SoundEffect::Pickup);
    }

    fn consume_active_stack(&mut self, item: ItemId) {
        if self.mode == GameMode::Creative {
            return;
        }
        debug_assert!(self.player.inventory.remove(item, 1));
        if self.player.inventory.count(item) == 0 {
            self.player.active_item = None;
        }
    }

    fn pay_stamina(&mut self, cost: u8) -> bool {
        if self.mode == GameMode::Creative || self.effect_active(PotionKind::Energy) {
            return true;
        }
        if self.player.stamina == 0 {
            return false;
        }
        self.player.stamina = self.player.stamina.saturating_sub(cost);
        true
    }

    fn apply_potion(&mut self, potion: PotionKind) -> bool {
        match potion {
            PotionKind::Awkward => return false,
            PotionKind::Health => {
                self.player.health = (self.player.health + 5).min(self.player.max_health);
            }
            PotionKind::Escape => {
                if self.current_level == 1 {
                    self.notification = Some(("YOU CANNOT ESCAPE FROM HERE".to_owned(), 60));
                    return false;
                }
                self.current_level = if self.current_level == 0 {
                    1
                } else {
                    self.current_level - 1
                };
                let (x, y) = find_spawn(
                    &self.levels[self.current_level].tiles,
                    self.width,
                    self.height,
                );
                self.player.x = x * TILE_SIZE + 8;
                self.player.y = y * TILE_SIZE + 8;
            }
            _ => {
                self.player.potion_effects[potion.id()] = potion.duration();
            }
        }
        true
    }

    fn effect_active(&self, potion: PotionKind) -> bool {
        self.player.potion_effects[potion.id()] > 0
    }

    fn tick_potion_effects(&mut self) {
        for duration in &mut self.player.potion_effects {
            *duration = duration.saturating_sub(1);
        }
        if self.effect_active(PotionKind::Regen) {
            self.player.regen_tick = self.player.regen_tick.saturating_add(1);
            if self.player.regen_tick > 60 {
                self.player.regen_tick = 0;
                self.player.health = self
                    .player
                    .health
                    .saturating_add(1)
                    .min(self.player.max_health);
            }
        } else {
            self.player.regen_tick = 0;
        }
    }

    fn tick_fishing(&mut self) {
        let Some(level) = self.player.fishing_level else {
            return;
        };
        self.player.fishing_ticks = self.player.fishing_ticks.saturating_add(1);
        if self.player.fishing_ticks < 120 {
            return;
        }
        self.player.fishing_ticks = 0;
        self.player.fishing_level = None;
        const CHANCES: [[i32; 4]; 4] = [
            [44, 14, 9, 4],
            [24, 14, 9, 4],
            [59, 49, 9, 4],
            [79, 69, 59, 4],
        ];
        let roll = self.random.next_int(100);
        let chances = CHANCES[level as usize];
        let caught = if roll > chances[0] {
            Some(ItemId::RawFish)
        } else if roll > chances[1] {
            Some(
                [
                    ItemId::Cloth,
                    ItemId::Leather,
                    ItemId::Bone,
                    ItemId::Slime,
                    ItemId::Wood,
                ][self.random.next_int(5) as usize],
            )
        } else if roll >= chances[3] && roll <= chances[2] {
            Some(
                [
                    ItemId::Gunpowder,
                    ItemId::Gem,
                    ItemId::Book,
                    ItemId::GoldenApple,
                ][self.random.next_int(4) as usize],
            )
        } else {
            None
        };
        if let Some(item) = caught {
            if self.player.inventory.add(item, 1) != 0 {
                self.levels[self.current_level].entities.spawn_item(
                    ItemStack::new(item, 1),
                    self.player.x,
                    self.player.y,
                );
            }
            self.unlock_achievement("minicraft.achievement.fish");
        } else {
            let tiers = [
                ToolTier::Wood,
                ToolTier::Rock,
                ToolTier::Iron,
                ToolTier::Gold,
            ];
            let kinds = [ToolKind::Pickaxe, ToolKind::Axe, ToolKind::Shovel];
            let tier = tiers[self.random.next_int(tiers.len() as i32) as usize];
            let kind = kinds[self.random.next_int(kinds.len() as i32) as usize];
            let _ = self
                .player
                .inventory
                .add_tool(crate::item::ToolItem::new(kind, tier));
        }
    }

    fn hurt_player(&mut self, mut damage: u8, direct: bool) -> bool {
        if self.mode == GameMode::Creative {
            return false;
        }
        if self.player.hurt_time > 0 && !direct {
            return false;
        }
        if self.effect_active(PotionKind::Shield) {
            damage = damage.div_ceil(2);
        }
        let mut health_damage = damage;
        if !direct && let Some(kind) = self.player.armor_kind {
            self.player.armor_damage_buffer =
                self.player.armor_damage_buffer.saturating_add(damage);
            health_damage = 0;
            while self.player.armor_damage_buffer > kind.level() {
                self.player.armor_damage_buffer -= kind.level() + 1;
                health_damage = health_damage.saturating_add(1);
            }
            let overflow = damage.saturating_sub(self.player.armor);
            self.player.armor = self.player.armor.saturating_sub(damage);
            if self.player.armor == 0 {
                health_damage = health_damage.saturating_add(overflow);
                self.player.armor_kind = None;
                self.player.armor_damage_buffer = 0;
            }
        }
        self.player.health = self.player.health.saturating_sub(health_damage);
        self.player.hurt_time = 30;
        self.sound(SoundEffect::PlayerHurt);
        true
    }

    fn eat_active_food(&mut self) -> bool {
        let Some(ActiveItem::Stack(item)) = self.player.active_item else {
            return false;
        };
        let Some(feed) = item.food_value() else {
            return false;
        };
        if self.player.hunger >= MAX_STAT
            || (self.player.stamina == 0 && !self.effect_active(PotionKind::Energy))
        {
            return false;
        }
        self.pay_stamina(2);
        self.player.hunger = self.player.hunger.saturating_add(feed).min(MAX_STAT);
        self.consume_active_stack(item);
        true
    }

    fn tick_survival_stats(&mut self) {
        if self.player.stamina == 0
            && self.player.stamina_recharge_delay == 0
            && self.player.stamina_recharge == 0
        {
            self.player.stamina_recharge_delay = 40;
        }
        if self.player.stamina_recharge_delay > 0 && self.player.stamina < MAX_STAT {
            self.player.stamina_recharge_delay -= 1;
        }
        if self.player.stamina_recharge_delay == 0 {
            self.player.stamina_recharge = self.player.stamina_recharge.saturating_add(1);
            while self.player.stamina_recharge > 10 {
                self.player.stamina_recharge -= 10;
                self.player.stamina = self.player.stamina.saturating_add(1).min(MAX_STAT);
            }
        }

        let difficulty = self.difficulty.min(2);
        if self.player.stamina < MAX_STAT {
            self.player.hunger_ticks -= difficulty as i16;
            if self.player.stamina == 0 {
                self.player.hunger_ticks -= difficulty as i16;
            }
        }
        if self.tick.is_multiple_of(HUNGER_TICK_INTERVALS[difficulty]) {
            self.player.hunger_ticks -= 1;
        }
        if self.player.step_count >= HUNGER_MOVE_STEPS[difficulty] {
            self.player.hunger_ticks -= 1;
            self.player.step_count = 0;
        }
        if self.player.hunger_charge_delay > 0 {
            self.player.hunger_ticks -= (2 + difficulty) as i16;
            if self.player.hunger == 0 {
                self.player.hunger_ticks -= difficulty as i16;
            }
        }
        while self.player.hunger_ticks <= 0 {
            self.player.hunger_ticks += MAX_HUNGER_TICKS;
            self.player.hunger_stamina_count -= 1;
        }
        while self.player.hunger_stamina_count <= 0 {
            self.player.hunger = self.player.hunger.saturating_sub(1);
            self.player.hunger_stamina_count += HUNGER_STAMINA_STEPS[difficulty];
        }

        if self.player.health < self.player.max_health && self.player.hunger > MAX_STAT / 2 {
            self.player.hunger_charge_delay += 1;
            let distance = u16::from(MAX_STAT - self.player.hunger + 2);
            if self.player.hunger_charge_delay > 20 * distance * distance {
                self.player.health = self
                    .player
                    .health
                    .saturating_add(1)
                    .min(self.player.max_health);
                self.player.hunger_charge_delay = 0;
            }
        } else {
            self.player.hunger_charge_delay = 0;
        }

        if self.player.hunger_starve_delay == 0 {
            self.player.hunger_starve_delay = 120;
        }
        if self.player.hunger == 0 && self.player.health > STARVATION_HEALTH_FLOORS[difficulty] {
            self.player.hunger_starve_delay -= 1;
            if self.player.hunger_starve_delay == 0 {
                self.player.health = self.player.health.saturating_sub(1);
            }
        }
    }

    fn move_player(&mut self, horizontal: i32, vertical: i32) -> bool {
        let old_x = self.player.x;
        let old_y = self.player.y;
        let next_x = self.player.x + horizontal;
        if self.can_stand(next_x, self.player.y) {
            self.player.x = next_x;
        }
        let next_y = self.player.y + vertical;
        if self.can_stand(self.player.x, next_y) {
            self.player.y = next_y;
        }
        self.player.x != old_x || self.player.y != old_y
    }

    fn can_stand(&self, x: i32, y: i32) -> bool {
        [(-4, -3), (4, -3), (-4, 4), (4, 4)]
            .into_iter()
            .all(|(offset_x, offset_y)| {
                let (tile, data) = self.tile_and_data_at_pixel(x + offset_x, y + offset_y);
                (self.mode == GameMode::Creative && tile == Tile::InfiniteFall || !tile.solid(data))
                    && !self.levels[self.current_level]
                        .entities
                        .furniture_blocks(x + offset_x, y + offset_y)
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
        self.player.walk_distance = self.player.walk_distance.wrapping_add(8);
        self.player.attack_time = if self.player.active_item.is_some() {
            10
        } else {
            5
        };
        self.player.attack_direction = self.player.direction;
        self.player.attack_item = self.player.active_item;
        let (offset_x, offset_y) = match self.player.direction {
            Direction::Down => (0, 12),
            Direction::Up => (0, -12),
            Direction::Left => (-12, 0),
            Direction::Right => (12, 0),
        };
        let target_x = self.player.x + offset_x;
        let target_y = self.player.y + offset_y;
        if let Some(ActiveItem::Tool(index)) = self.player.active_item
            && let Some(tool) = self.player.inventory.tools().get(index).copied()
        {
            if tool.kind == ToolKind::Bow {
                if (self.mode == GameMode::Creative
                    || self.player.stamina > 0
                    || self.effect_active(PotionKind::Energy))
                    && (self.mode == GameMode::Creative
                        || self.player.inventory.count(ItemId::Arrow) > 0)
                    && !tool.is_depleted()
                {
                    if self.mode != GameMode::Creative {
                        self.player.inventory.remove(ItemId::Arrow, 1);
                        self.player.inventory.tools_mut()[index].pay_durability();
                    }
                    let (far_x, far_y) = match self.player.direction {
                        Direction::Down => (self.player.x, self.player.y + 160),
                        Direction::Up => (self.player.x, self.player.y - 160),
                        Direction::Left => (self.player.x - 160, self.player.y),
                        Direction::Right => (self.player.x + 160, self.player.y),
                    };
                    self.levels[self.current_level].entities.spawn_arrow(
                        self.player.x,
                        self.player.y,
                        far_x,
                        far_y,
                        tool.tier.level() + 3,
                        false,
                    );
                }
                return;
            }
            if tool.kind == ToolKind::Shears {
                if self.levels[self.current_level].entities.shear_nearest(
                    target_x,
                    target_y,
                    &mut self.random,
                ) && self.mode != GameMode::Creative
                {
                    self.player.inventory.tools_mut()[index].pay_durability();
                }
                return;
            }
        }
        if let Some(ActiveItem::Stack(item)) = self.player.active_item
            && let Some(kind) = furniture_for_item(item)
            && self.place_furniture(kind, item, offset_x, offset_y)
        {
            return;
        }

        if self.levels[self.current_level]
            .entities
            .ignite_tnt_near(target_x, target_y)
        {
            self.sound(SoundEffect::Fuse);
            return;
        }

        let mut damage = (self.random.next_int(2) + 1) as u8;
        if self.levels[self.current_level]
            .entities
            .mob_near(self.player.x + offset_x, self.player.y + offset_y)
        {
            damage = damage.saturating_add(self.pay_tool_melee_bonus());
        }
        if let Some(hit) = self.levels[self.current_level].entities.damage_nearest(
            self.player.x + offset_x,
            self.player.y + offset_y,
            u16::from(damage),
            &mut self.random,
        ) {
            self.player.attack_time = 5;
            self.levels[self.current_level]
                .entities
                .spawn_text_particle(hit.damage.to_string(), hit.x, hit.y);
            if !hit.defeated
                || !matches!(
                    hit.species,
                    spawn::NaturalMob::AirWizard | spawn::NaturalMob::ObsidianKnight
                )
            {
                self.sound(SoundEffect::MonsterHurt);
            }
            if hit.defeated {
                self.handle_defeated_mob(hit.species);
            }
            return;
        }
        let tile_x = (self.player.x + offset_x) / TILE_SIZE;
        let tile_y = (self.player.y + offset_y) / TILE_SIZE;
        if tile_x < 0 || tile_y < 0 || tile_x >= self.width as i32 || tile_y >= self.height as i32 {
            return;
        }
        let index = tile_x as usize + tile_y as usize * self.width;
        if let Some(ActiveItem::Stack(item)) = self.player.active_item {
            let target = self.levels[self.current_level].tiles[index];
            if item == ItemId::WateringCan {
                if target == Tile::Water {
                    self.player.watering_content = 1_800;
                } else if self.player.watering_content > 0 {
                    self.player.watering_content -= 1;
                    if matches!(
                        target,
                        Tile::Wheat
                            | Tile::Potato
                            | Tile::Tomato
                            | Tile::Carrot
                            | Tile::HeavenlyBerries
                            | Tile::HellishBerries
                    ) {
                        let data = self.levels[self.current_level].data[index];
                        let fertilization = (data >> 7).min(150);
                        self.levels[self.current_level].data[index] =
                            (data & 0x7f) | ((fertilization + 1).min(150) << 7);
                    } else if target == Tile::Farmland {
                        self.levels[self.current_level].data[index] = 7;
                    }
                }
                return;
            }
            if let Some(fishing_level) = item.fishing_level() {
                if target == Tile::Water
                    && self.tile_at_pixel(self.player.x, self.player.y) != Tile::Water
                {
                    self.player.fishing_level = Some(fishing_level);
                    self.player.fishing_ticks = 0;
                }
                return;
            }
            if self.use_bucket(item, index) {
                return;
            }
            if matches!(item, ItemId::Fertilizer | ItemId::ArcaneFertilizer)
                && matches!(
                    target,
                    Tile::Wheat
                        | Tile::Potato
                        | Tile::Tomato
                        | Tile::Carrot
                        | Tile::HeavenlyBerries
                        | Tile::HellishBerries
                )
            {
                let data = self.levels[self.current_level].data[index];
                let old = data >> 7;
                let amount = if item == ItemId::ArcaneFertilizer {
                    300
                } else if old < 100 {
                    40
                } else if old < 200 {
                    30
                } else if old < 300 {
                    25
                } else if old < 400 {
                    20
                } else {
                    10
                };
                self.levels[self.current_level].data[index] =
                    (data & 0x7f) | ((old + amount).min(511) << 7);
                self.consume_active_stack(item);
                return;
            }
            if let Some((placed, data)) = tile_placement(item, target) {
                self.levels[self.current_level].tiles[index] = placed;
                self.levels[self.current_level].data[index] = data;
                self.consume_active_stack(item);
                self.update_progress(ProgressEvent::PlacedTile {
                    tile: tile_progress_name(placed),
                });
                if matches!(
                    placed,
                    Tile::Wheat
                        | Tile::Potato
                        | Tile::Tomato
                        | Tile::Carrot
                        | Tile::HeavenlyBerries
                        | Tile::HellishBerries
                ) {
                    self.unlock_achievement("minicraft.achievement.plant_seed");
                }
                return;
            }
        }

        if matches!(self.player.active_item, Some(ActiveItem::Stack(_))) {
            return;
        }

        if self.mode == GameMode::Creative {
            self.player.attack_time = 5;
            if self.creative_break_tile(index) {
                return;
            }
        }

        if let Some(tool) = self.active_tool() {
            let target = self.levels[self.current_level].tiles[index];
            let replacement = match (tool.kind, target) {
                (ToolKind::Shovel, Tile::Dirt) => Some((Tile::Hole, Some(ItemId::Dirt))),
                (ToolKind::Shovel, Tile::Grass) => Some((Tile::Dirt, None)),
                (ToolKind::Shovel, Tile::Sand) => Some((Tile::Hole, Some(ItemId::Sand))),
                (ToolKind::Shovel, Tile::Cloud) => Some((Tile::InfiniteFall, Some(ItemId::Cloud))),
                (ToolKind::Hoe, Tile::Dirt | Tile::Grass) => Some((Tile::Farmland, None)),
                (ToolKind::Pickaxe, Tile::Grass) => Some((Tile::Path, None)),
                _ => None,
            };
            if let Some((tile, drop)) = replacement {
                let cost = if target == Tile::Cloud { 5 } else { 4 };
                if self.pay_tool_terrain_damage(tool.kind, cost).is_none() {
                    return;
                }
                self.levels[self.current_level].tiles[index] = tile;
                self.levels[self.current_level].data[index] = 0;
                self.levels[self.current_level].entities.spawn_particle(
                    entity::ParticleKind::Smash,
                    tile_x * TILE_SIZE,
                    tile_y * TILE_SIZE,
                );
                self.sound(SoundEffect::MonsterHurt);
                if tool.kind == ToolKind::Hoe {
                    self.update_progress(ProgressEvent::ItemUsedOnTile {
                        item: format!(
                            "{} hoe",
                            if tool.tier == ToolTier::Rock {
                                "stone"
                            } else {
                                tool.tier.display_name()
                            }
                            .to_ascii_lowercase()
                        ),
                        tile: tile_progress_name(target),
                    });
                }
                if let Some(item) = drop {
                    let count = if item == ItemId::Cloud {
                        (self.random.next_int(3) + 1) as u16
                    } else {
                        1
                    };
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(item, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                } else if target == Tile::Grass {
                    let seed = if tool.kind == ToolKind::Hoe {
                        (self.random.next_int(5) != 0).then_some(ItemId::WheatSeeds)
                    } else if tool.kind == ToolKind::Shovel {
                        (self.random.next_int(5) == 0).then_some(ItemId::GrassSeeds)
                    } else {
                        None
                    };
                    if let Some(seed) = seed {
                        self.levels[self.current_level].entities.spawn_item(
                            ItemStack::new(seed, 1),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                }
                return;
            }
        }
        let bare_damage = (self.random.next_int(3) + 1) as u16;
        let attacked_tile = self.levels[self.current_level].tiles[index];
        if matches!(
            attacked_tile,
            Tile::Tree
                | Tile::Cactus
                | Tile::Rock
                | Tile::HardRock
                | Tile::WoodWall
                | Tile::StoneWall
                | Tile::ObsidianWall
                | Tile::BossWall
                | Tile::Sign
                | Tile::IronOre
                | Tile::GoldOre
                | Tile::GemOre
                | Tile::LapisOre
                | Tile::CloudOre
                | Tile::Wheat
                | Tile::Potato
                | Tile::Tomato
                | Tile::Carrot
                | Tile::HeavenlyBerries
                | Tile::HellishBerries
        ) {
            self.levels[self.current_level].entities.spawn_particle(
                entity::ParticleKind::Smash,
                tile_x * TILE_SIZE,
                tile_y * TILE_SIZE,
            );
            self.sound(SoundEffect::MonsterHurt);
        }
        match self.levels[self.current_level].tiles[index] {
            Tile::Tree => {
                let tool_damage = self.pay_tool_terrain_damage(ToolKind::Axe, 4);
                let damage = tool_damage.unwrap_or(bare_damage);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let total = self.levels[self.current_level].data[index] + damage;
                if total >= 20 {
                    self.levels[self.current_level].tiles[index] = Tile::Grass;
                    self.levels[self.current_level].data[index] = 0;
                    let wood = (self.random.next_int(3) + 1) as u16;
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(ItemId::Wood, wood),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                    let acorns = self.random.next_int(3) as u16;
                    if acorns > 0 {
                        self.levels[self.current_level].entities.spawn_item(
                            ItemStack::new(ItemId::Acorn, acorns),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                    self.unlock_achievement("minicraft.achievement.woodcutter");
                } else {
                    self.levels[self.current_level].data[index] = total;
                }
            }
            Tile::WoodDoor | Tile::StoneDoor | Tile::ObsidianDoor => {
                self.levels[self.current_level].data[index] ^= 1;
            }
            Tile::BossDoor => {
                if self.mode == GameMode::Creative || self.obsidian_knight_defeated {
                    self.levels[self.current_level].data[index] ^= 1;
                } else {
                    self.notification = Some(("DEFEAT THE OBSIDIAN KNIGHT FIRST".to_owned(), 75));
                }
            }
            Tile::Cactus => {
                self.player.attack_time = 5;
                self.pay_basic_attack_tool_durability();
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        bare_damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                if damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    bare_damage,
                    10,
                    Tile::Sand,
                ) {
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(ItemId::Cactus, (self.random.next_int(3) + 2) as u16),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
            }
            Tile::Wheat
            | Tile::Potato
            | Tile::Tomato
            | Tile::Carrot
            | Tile::HeavenlyBerries
            | Tile::HellishBerries => {
                self.player.attack_time = 5;
                self.pay_basic_attack_tool_durability();
                let crop = self.levels[self.current_level].tiles[index];
                let data = self.levels[self.current_level].data[index];
                let mature = (data >> 3) & 7 == 7;
                self.levels[self.current_level].tiles[index] = Tile::Farmland;
                self.levels[self.current_level].data[index] = data & 7;
                let (produce, seed) = match crop {
                    Tile::Wheat => (ItemId::Wheat, Some(ItemId::WheatSeeds)),
                    Tile::Potato => (ItemId::Potato, None),
                    Tile::Tomato => (ItemId::Tomato, Some(ItemId::TomatoSeeds)),
                    Tile::Carrot => (ItemId::Carrot, None),
                    Tile::HeavenlyBerries => (ItemId::HeavenlyBerries, None),
                    Tile::HellishBerries => (ItemId::HellishBerries, None),
                    _ => unreachable!(),
                };
                if let Some(seed) = seed {
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(seed, 1),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
                if mature || seed.is_none() {
                    let count = if mature {
                        (self.random.next_int(3) + 2) as u16
                    } else {
                        1
                    };
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(produce, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
                if mature {
                    let crop_score = (self.random.next_int(5) + 1) as u32;
                    self.add_score(crop_score, 0);
                }
            }
            Tile::Rock => {
                let tool_damage = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 5);
                let used_pickaxe = tool_damage.is_some();
                let damage = tool_damage.unwrap_or(bare_damage);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let level = &mut self.levels[self.current_level];
                if damage_tile(level, index, damage, 50, Tile::Dirt) {
                    let count = if used_pickaxe {
                        (self.random.next_int(3) + 2) as u16
                    } else {
                        1
                    };
                    level.entities.spawn_item(
                        ItemStack::new(ItemId::Stone, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                    let coal_max = if self.difficulty == 2 { 1 } else { 2 };
                    let coal = if used_pickaxe {
                        self.random.next_int(coal_max + 1) as u16
                    } else {
                        0
                    };
                    if coal > 0 {
                        level.entities.spawn_item(
                            ItemStack::new(ItemId::Coal, coal),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                }
            }
            Tile::HardRock => {
                let gem_pickaxe = self
                    .active_tool()
                    .is_some_and(|tool| tool.kind == ToolKind::Pickaxe && tool.tier.level() == 4);
                if !gem_pickaxe {
                    if self.active_tool().is_some() {
                        self.notification = Some(("GEM PICKAXE REQUIRED".to_owned(), 45));
                    }
                    self.levels[self.current_level]
                        .entities
                        .spawn_text_particle(
                            "0".to_owned(),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                    return;
                }
                let Some(damage) = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 2) else {
                    return;
                };
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let level = &mut self.levels[self.current_level];
                if damage_tile(level, index, damage, 200, Tile::Dirt) {
                    level.entities.spawn_item(
                        ItemStack::new(ItemId::Stone, (self.random.next_int(3) + 1) as u16),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                    if self.random.next_bool() {
                        level.entities.spawn_item(
                            ItemStack::new(ItemId::Coal, 1),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                }
            }
            Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall => {
                if self.levels[self.current_level].tiles[index] == Tile::ObsidianWall
                    && self.levels[self.current_level].depth == -3
                    && !self.air_wizard_defeated
                    && self.mode != GameMode::Creative
                {
                    self.notification = Some(("DEFEAT THE AIR WIZARD FIRST".to_owned(), 75));
                    return;
                }
                let required = if self.levels[self.current_level].tiles[index] == Tile::WoodWall {
                    ToolKind::Axe
                } else {
                    ToolKind::Pickaxe
                };
                let tool_damage = self.pay_tool_terrain_damage(required, 4);
                let damage = tool_damage.unwrap_or(0);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
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
            Tile::BossWall => {
                if !self.obsidian_knight_defeated && self.mode != GameMode::Creative {
                    self.notification = Some(("DEFEAT THE OBSIDIAN KNIGHT FIRST".to_owned(), 75));
                    return;
                }
                let Some(damage) = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 4) else {
                    self.notification = Some(("PICKAXE REQUIRED".to_owned(), 45));
                    return;
                };
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    100,
                    Tile::BossFloor,
                );
            }
            Tile::Sign => {
                let axe = self
                    .active_tool()
                    .is_some_and(|tool| tool.kind == ToolKind::Axe);
                if !axe {
                    self.sign_editor = Some(SignEditor {
                        level: self.current_level,
                        tile: index,
                        text: self.signs[self.current_level]
                            .get(&index)
                            .cloned()
                            .unwrap_or_default(),
                    });
                    return;
                }
                if self.pay_tool_terrain_damage(ToolKind::Axe, 4).is_none() {
                    return;
                }
                let underlying = Tile::from_id(self.levels[self.current_level].data[index] as u8)
                    .unwrap_or(Tile::Dirt);
                self.levels[self.current_level].tiles[index] = underlying;
                self.levels[self.current_level].data[index] = 0;
                self.signs[self.current_level].remove(&index);
                self.levels[self.current_level].entities.spawn_item(
                    ItemStack::new(ItemId::Sign, 1),
                    tile_x * TILE_SIZE + 8,
                    tile_y * TILE_SIZE + 8,
                );
            }
            Tile::IronOre | Tile::GoldOre | Tile::GemOre | Tile::LapisOre | Tile::CloudOre => {
                let ore_tile = self.levels[self.current_level].tiles[index];
                let tool_damage = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 6);
                let damage = tool_damage.unwrap_or(0);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let replacement = if ore_tile == Tile::CloudOre {
                    Tile::Cloud
                } else {
                    Tile::Dirt
                };
                let health = (self.random.next_int(10) * 4 + 20) as u16;
                let broken = damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    health,
                    replacement,
                );
                let count = self.random.next_int(2) as u16 + if broken { 2 } else { 0 };
                if count > 0 {
                    let item = match ore_tile {
                        Tile::IronOre => ItemId::IronOre,
                        Tile::GoldOre => ItemId::GoldOre,
                        Tile::GemOre => ItemId::Gem,
                        Tile::LapisOre => ItemId::Lapis,
                        Tile::CloudOre => ItemId::CloudOre,
                        _ => unreachable!(),
                    };
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(item, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
            }
            _ => {}
        }
    }

    fn creative_break_tile(&mut self, index: usize) -> bool {
        let tile = self.levels[self.current_level].tiles[index];
        let data = self.levels[self.current_level].data[index];
        let replacement = match tile {
            Tile::Tree | Tile::TreeSapling | Tile::Flower => Tile::Grass,
            Tile::Cactus | Tile::CactusSapling => Tile::Sand,
            Tile::Rock
            | Tile::HardRock
            | Tile::IronOre
            | Tile::GoldOre
            | Tile::GemOre
            | Tile::LapisOre => Tile::Dirt,
            Tile::CloudOre => Tile::Cloud,
            Tile::Wheat
            | Tile::Potato
            | Tile::Tomato
            | Tile::Carrot
            | Tile::HeavenlyBerries
            | Tile::HellishBerries => Tile::Farmland,
            Tile::WoodWall => Tile::WoodFloor,
            Tile::StoneWall => Tile::StoneFloor,
            Tile::ObsidianWall => Tile::ObsidianFloor,
            Tile::WoodDoor => Tile::WoodFloor,
            Tile::StoneDoor => Tile::StoneFloor,
            Tile::ObsidianDoor => Tile::ObsidianFloor,
            Tile::Torch | Tile::Sign | Tile::WoodFence | Tile::StoneFence | Tile::ObsidianFence => {
                Tile::from_id(data as u8).unwrap_or(Tile::Dirt)
            }
            _ => return false,
        };
        self.levels[self.current_level].tiles[index] = replacement;
        self.levels[self.current_level].data[index] = 0;
        if tile == Tile::Sign {
            self.signs[self.current_level].remove(&index);
        }
        true
    }

    fn pay_tool_melee_bonus(&mut self) -> u8 {
        let Some(ActiveItem::Tool(index)) = self.player.active_item else {
            return 0;
        };
        let Some(tool) = self.player.inventory.tools().get(index).copied() else {
            self.player.active_item = None;
            return 0;
        };
        if tool.kind == ToolKind::Shears || tool.is_depleted() {
            return 0;
        }
        let roll_bound = match tool.kind {
            ToolKind::Axe => 4,
            ToolKind::Sword => 2 + i32::from(tool.tier.level()).pow(2),
            ToolKind::Claymore => 4 + i32::from(tool.tier.level()).pow(2) * 3,
            ToolKind::Pickaxe => 2,
            _ => 1,
        };
        let roll = self.random.next_int(roll_bound) as u8;
        let bonus = tool.melee_bonus(roll);
        if self.mode == GameMode::Creative {
            bonus
        } else {
            let equipped = &mut self.player.inventory.tools_mut()[index];
            if equipped.pay_durability() { bonus } else { 0 }
        }
    }

    fn pay_basic_attack_tool_durability(&mut self) {
        if self.mode == GameMode::Creative {
            return;
        }
        let Some(ActiveItem::Tool(index)) = self.player.active_item else {
            return;
        };
        let Some(tool) = self.player.inventory.tools_mut().get_mut(index) else {
            self.player.active_item = None;
            return;
        };
        if tool.kind != ToolKind::Shears {
            tool.pay_durability();
        }
    }

    fn use_bucket(&mut self, item: ItemId, index: usize) -> bool {
        let target = self.levels[self.current_level].tiles[index];
        let replacement = match (item, target) {
            (ItemId::EmptyBucket, Tile::Water) => Some((Tile::Hole, ItemId::WaterBucket)),
            (ItemId::EmptyBucket, Tile::Lava) => Some((Tile::Hole, ItemId::LavaBucket)),
            (ItemId::WaterBucket, Tile::Hole) => Some((Tile::Water, ItemId::EmptyBucket)),
            (ItemId::LavaBucket, Tile::Hole) => Some((Tile::Lava, ItemId::EmptyBucket)),
            (ItemId::WaterBucket, Tile::Lava) => Some((Tile::ObsidianFloor, ItemId::EmptyBucket)),
            _ => None,
        };
        let Some((tile, bucket)) = replacement else {
            return false;
        };
        self.levels[self.current_level].tiles[index] = tile;
        self.levels[self.current_level].data[index] = 0;
        if item == ItemId::WaterBucket && target == Tile::Lava {
            self.unlock_achievement("minicraft.achievement.lava");
        }
        if self.mode == GameMode::Creative {
            return true;
        }
        self.consume_active_stack(item);
        self.player.inventory.add(bucket, 1);
        self.player.active_item = Some(ActiveItem::Stack(bucket));
        true
    }

    fn apply_explosion(&mut self, x: i32, y: i32, radius: i32, player_tnt: bool) {
        self.sound(SoundEffect::Explode);
        if player_tnt {
            self.unlock_achievement("minicraft.achievement.demolition");
        }
        let center_x = x.div_euclid(TILE_SIZE);
        let center_y = y.div_euclid(TILE_SIZE);
        for tile_y in center_y - radius..=center_y + radius {
            for tile_x in center_x - radius..=center_x + radius {
                if tile_x < 0
                    || tile_y < 0
                    || tile_x >= self.width as i32
                    || tile_y >= self.height as i32
                {
                    continue;
                }
                let index = tile_x as usize + tile_y as usize * self.width;
                if matches!(
                    self.levels[self.current_level].tiles[index],
                    Tile::StairsUp
                        | Tile::StairsDown
                        | Tile::BossWall
                        | Tile::BossDoor
                        | Tile::BossFloor
                        | Tile::InfiniteFall
                ) {
                    continue;
                }
                self.levels[self.current_level].tiles[index] = Tile::Exploded;
                self.levels[self.current_level].data[index] = 0;
                self.levels[self.current_level].entities.spawn_particle(
                    entity::ParticleKind::Fire,
                    tile_x * TILE_SIZE + 8,
                    tile_y * TILE_SIZE + 8,
                );
            }
        }
    }

    fn active_tool(&self) -> Option<crate::item::ToolItem> {
        let ActiveItem::Tool(index) = self.player.active_item? else {
            return None;
        };
        self.player.inventory.tools().get(index).copied()
    }

    fn pay_tool_terrain_damage(&mut self, required: ToolKind, stamina_cost: u8) -> Option<u16> {
        let ActiveItem::Tool(index) = self.player.active_item? else {
            return None;
        };
        let tool = *self.player.inventory.tools().get(index)?;
        if tool.kind != required || tool.is_depleted() {
            return None;
        }
        let cost = stamina_cost.saturating_sub(tool.tier.level());
        let infinite_energy = self.effect_active(PotionKind::Energy);
        if self.mode != GameMode::Creative && !infinite_energy && self.player.stamina == 0 {
            return None;
        }
        let mut damage = tool.terrain_damage(self.random.next_int(5) as u8);
        if self.effect_active(PotionKind::Haste) {
            damage = damage.saturating_mul(2);
        }
        if self.mode != GameMode::Creative
            && !self.player.inventory.tools_mut()[index].pay_durability()
        {
            return None;
        }
        if self.mode != GameMode::Creative && !infinite_energy {
            self.player.stamina = self.player.stamina.saturating_sub(cost);
        }
        Some(damage)
    }

    fn place_furniture(
        &mut self,
        kind: FurnitureKind,
        item: ItemId,
        offset_x: i32,
        offset_y: i32,
    ) -> bool {
        let tile_x = (self.player.x + offset_x) / TILE_SIZE;
        let tile_y = (self.player.y + offset_y) / TILE_SIZE;
        if tile_x < 0 || tile_y < 0 || tile_x >= self.width as i32 || tile_y >= self.height as i32 {
            return false;
        }
        let index = tile_x as usize + tile_y as usize * self.width;
        let tile = self.levels[self.current_level].tiles[index];
        if tile.solid(self.levels[self.current_level].data[index])
            || matches!(
                tile,
                Tile::Water
                    | Tile::Lava
                    | Tile::Hole
                    | Tile::InfiniteFall
                    | Tile::StairsUp
                    | Tile::StairsDown
            )
        {
            return false;
        }
        let x = tile_x * TILE_SIZE + 8;
        let y = tile_y * TILE_SIZE + 8;
        if self.levels[self.current_level]
            .entities
            .furniture_near(x, y, 8)
            .is_some()
        {
            return false;
        }
        if self.mode != GameMode::Creative && !self.player.inventory.remove(item, 1) {
            self.player.active_item = None;
            return false;
        }
        self.levels[self.current_level]
            .entities
            .spawn_furniture(kind, x, y);
        if self.mode != GameMode::Creative && self.player.inventory.count(item) == 0 {
            self.player.active_item = None;
        }
        true
    }

    fn respawn(&mut self) {
        if self.mode == GameMode::Score {
            self.score -= self.score / 3;
            self.score_multiplier = 1;
            self.multiplier_ticks = 300;
        }
        let drop_x = self.player.x;
        let drop_y = self.player.y;
        for stack in self.player.inventory.take_all() {
            self.levels[self.current_level]
                .entities
                .spawn_item(stack, drop_x, drop_y);
        }
        if let Some(armor) = self.player.armor_kind.take() {
            self.levels[self.current_level].entities.spawn_item(
                ItemStack::new(armor.item(), 1),
                drop_x,
                drop_y,
            );
        }
        self.current_level = 1;
        let (x, y) = find_spawn(
            &self.levels[self.current_level].tiles,
            self.width,
            self.height,
        );
        self.player.x = x * TILE_SIZE + 8;
        self.player.y = y * TILE_SIZE + 8;
        self.player.health = self.player.max_health;
        self.player.stamina = 10;
        self.player.hunger = 10;
        self.player.armor = 0;
        self.player.armor_damage_buffer = 0;
        self.player.hurt_time = 90;
        self.player.stamina_recharge = 0;
        self.player.stamina_recharge_delay = 0;
        self.player.hunger_stamina_count = HUNGER_STAMINA_STEPS[self.difficulty];
        self.player.hunger_ticks = MAX_HUNGER_TICKS;
        self.player.step_count = 0;
        self.player.attack_time = 0;
        self.player.attack_item = None;
        self.player.hunger_charge_delay = 0;
        self.player.hunger_starve_delay = 0;
        self.player.potion_effects = [0; PotionKind::ALL.len()];
        self.player.regen_tick = 0;
        self.player.fishing_level = None;
        self.player.watering_content = 0;
        self.player.fishing_ticks = 0;
    }

    fn use_target(&mut self) {
        let (offset_x, offset_y) = match self.player.direction {
            Direction::Down => (0, 12),
            Direction::Up => (0, -12),
            Direction::Left => (-12, 0),
            Direction::Right => (12, 0),
        };
        let target_tile_x = (self.player.x + offset_x).div_euclid(TILE_SIZE);
        let target_tile_y = (self.player.y + offset_y).div_euclid(TILE_SIZE);
        if target_tile_x >= 0
            && target_tile_y >= 0
            && target_tile_x < self.width as i32
            && target_tile_y < self.height as i32
        {
            let index = target_tile_x as usize + target_tile_y as usize * self.width;
            if self.levels[self.current_level].tiles[index] == Tile::Sign {
                self.sign_editor = Some(SignEditor {
                    level: self.current_level,
                    tile: index,
                    text: self.signs[self.current_level]
                        .get(&index)
                        .cloned()
                        .unwrap_or_default(),
                });
                return;
            }
        }
        if let Some(kind) = self.levels[self.current_level].entities.furniture_near(
            self.player.x + offset_x,
            self.player.y + offset_y,
            14,
        ) {
            if kind.crafting() {
                self.inventory_open = true;
                self.crafting_station = Some(kind);
                self.personal_crafting = false;
                self.inventory_pane = 1;
                self.inventory_selection = 0;
            } else if matches!(kind, FurnitureKind::Chest | FurnitureKind::DungeonChest) {
                let active = match self.player.active_item {
                    Some(ActiveItem::Stack(item)) => Some(item),
                    _ => None,
                };
                self.levels[self.current_level].entities.use_container_near(
                    self.player.x + offset_x,
                    self.player.y + offset_y,
                    &mut self.player.inventory,
                    active,
                );
                if let Some(item) = active
                    && self.player.inventory.count(item) == 0
                {
                    self.player.active_item = None;
                }
            } else if kind == FurnitureKind::Composter {
                let active = match self.player.active_item {
                    Some(ActiveItem::Stack(item)) => Some(item),
                    _ => None,
                };
                self.levels[self.current_level].entities.use_composter_near(
                    self.player.x + offset_x,
                    self.player.y + offset_y,
                    &mut self.player.inventory,
                    active,
                );
                if let Some(item) = active
                    && self.player.inventory.count(item) == 0
                {
                    self.player.active_item = None;
                }
            } else if kind == FurnitureKind::KnightStatue {
                let touches = self.levels[self.current_level]
                    .entities
                    .tap_statue_near(self.player.x + offset_x, self.player.y + offset_y);
                self.notification = touches.map(|touches| {
                    if touches >= 3 {
                        ("THE OBSIDIAN KNIGHT AWAKENS".to_owned(), 120)
                    } else {
                        (format!("THE STATUE STIRS {touches}/3"), 60)
                    }
                });
            } else if kind == FurnitureKind::Bed {
                if self.day_tick < 48_600 {
                    self.notification = Some(("IT IS TOO EARLY TO SLEEP".to_owned(), 60));
                } else {
                    self.sleeping = 120;
                }
            }
            return;
        }

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
            let depth = self.levels[target].depth;
            if depth == -3 {
                self.unlock_achievement("minicraft.achievement.lowest_caves");
            }
            if depth == -4 {
                self.unlock_achievement("minicraft.achievement.obsidian_dungeon");
            }
        }
    }

    fn current_boss_health(&self) -> Option<(u16, u16)> {
        let species = match self.levels[self.current_level].depth {
            1 => spawn::NaturalMob::AirWizard,
            -4 => spawn::NaturalMob::ObsidianKnight,
            _ => return None,
        };
        self.levels[self.current_level]
            .entities
            .active_boss(species)
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
        let light_scale = if self.effect_active(PotionKind::Light) {
            12
        } else {
            8
        };
        let mut lights = vec![(
            self.player.x - camera_x,
            self.player.y - camera_y,
            5 * light_scale,
        )];
        lights.extend(
            self.levels[self.current_level]
                .entities
                .light_sources()
                .map(|(x, y, radius)| (x - camera_x, y - camera_y, radius * light_scale)),
        );

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
                        light_radius * light_scale,
                    ));
                }
            }
        }

        let mut entities = self.levels[self.current_level]
            .entities
            .entities()
            .iter()
            .collect::<Vec<_>>();
        entities.sort_by_key(|entity| (entity.y, entity.id));
        for entity in entities
            .iter()
            .copied()
            .take_while(|entity| entity.y <= self.player.y)
        {
            render_entity(screen, assets, entity, camera_x, camera_y, self.tick);
        }

        let (source_x, flip) =
            player_sprite_frame(self.player.direction, self.player.walk_distance);
        let player_x = self.player.x - 8 - camera_x;
        let player_y = self.player.y - 11 - camera_y;
        screen.blit_region(
            &assets.skin,
            player_x,
            player_y,
            source_x,
            assets.skin_row,
            16,
            16,
            flip,
        );
        if self.player.attack_time > 0 {
            render_attack_slash(
                screen,
                &assets.hud,
                player_x,
                player_y,
                self.player.attack_direction,
            );
            let image = self.player.attack_item.and_then(|active| match active {
                ActiveItem::Stack(item) => Some(assets.item(item)),
                ActiveItem::Tool(index) => self
                    .player
                    .inventory
                    .tools()
                    .get(index)
                    .map(|tool| assets.tool(*tool)),
            });
            let (held_x, held_y, flip_item) = match self.player.attack_direction {
                Direction::Up => (player_x + 4, player_y - 4, true),
                Direction::Left => (player_x - 4, player_y + 4, true),
                Direction::Right => (player_x + 12, player_y + 4, false),
                Direction::Down => (player_x + 4, player_y + 12, false),
            };
            if let Some(image) = image {
                screen.blit_region(
                    image,
                    held_x,
                    held_y,
                    0,
                    0,
                    image.width,
                    image.height,
                    flip_item,
                );
            }
        }
        for entity in entities
            .iter()
            .copied()
            .skip_while(|entity| entity.y <= self.player.y)
        {
            render_entity(screen, assets, entity, camera_x, camera_y, self.tick);
        }

        let depth = self.levels[self.current_level].depth;
        let darkness = if self.mode == GameMode::Creative {
            0
        } else if depth == 0 {
            surface_darkness(self.day_tick)
        } else {
            176
        };
        screen.darken_with_lights(&lights, darkness);

        render_hud(
            screen,
            assets,
            &self.player,
            self.mode,
            self.seed,
            depth,
            self.day_tick,
            self.days,
        );
        if self.mode == GameMode::Score {
            render_score_overlay(
                screen,
                assets,
                self.score_ticks,
                self.score,
                self.score_multiplier,
            );
        }
        let mut potion_offset = 0;
        if self.show_quests && (self.tutorials_enabled || self.quests_enabled) {
            let inventory = self.inventory_progress();
            let progress = if self.tutorials_enabled {
                self.progress.current_tutorial().map(|step| {
                    format!(
                        "TUTORIAL {}",
                        short_progress_id(&step.id).to_ascii_uppercase()
                    )
                })
            } else {
                None
            }
            .or_else(|| {
                self.quests_enabled.then(|| {
                    self.progress.current_quest(&inventory).map(|quest| {
                        format!(
                            "QUEST {}",
                            short_progress_id(&quest.id).to_ascii_uppercase()
                        )
                    })
                })?
            });
            if let Some(progress) = progress {
                render_quest_overlay(screen, assets, &progress);
                potion_offset = 33;
            }
        }
        render_potion_overlay(screen, assets, &self.player, potion_offset);
        if let Some((health, max_health)) = self.current_boss_health() {
            render_boss_bar(
                screen,
                assets,
                health,
                max_health,
                if depth == 1 {
                    "Air wizard"
                } else {
                    "Obsidian Knight"
                },
            );
        }
        if self.story_complete {
            screen.rect(WIDTH as i32 - 82, 33, 80, 10, 0x101018);
            screen.text(&assets.font, "VICTORY", WIDTH as i32 - 76, 34);
        }
        if let Some((message, _)) = &self.notification {
            screen.centered_text(&assets.font, message, HEIGHT as i32 * 2 / 5);
        }
        if self.inventory_open {
            render_inventory(
                screen,
                assets,
                &self.player,
                self.inventory_item_selection,
                self.inventory_selection,
                self.inventory_pane,
                self.crafting_station,
                self.personal_crafting,
                self.mode,
            );
        }
        if self.paused {
            render_pause(screen, assets, self);
        }
        if let Some((book, page)) = self.book_open {
            render_world_book(screen, assets, book, page);
        }
        if let Some(editor) = &self.sign_editor {
            render_sign_editor(screen, assets, &editor.text);
        }
        if self.sleeping > 0 {
            screen.rect(0, 0, WIDTH as i32, HEIGHT as i32, 0x080812);
            screen.centered_text(&assets.font, "SLEEPING...", HEIGHT as i32 / 2);
        }
        if self.game_over {
            screen.rect(54, 76, WIDTH as i32 - 108, 74, 0x101018);
            screen.centered_text(
                &assets.font,
                if self.mode == GameMode::Hardcore {
                    "HARDCORE WORLD ENDED"
                } else {
                    "TIME IS UP"
                },
                91,
            );
            if self.mode == GameMode::Score {
                screen.centered_text(&assets.font, &format!("FINAL SCORE {}", self.score), 108);
            }
            screen.centered_text(&assets.font, "SELECT TO RETURN", 132);
        }
    }
}

fn render_world_book(screen: &mut Screen, assets: &Assets, book: Book, page: usize) {
    let pages = book.pages();
    let page = page.min(pages.len().saturating_sub(1));
    screen.rect(18, 17, WIDTH as i32 - 36, HEIGHT as i32 - 34, 0xD8C68F);
    screen.rect(22, 21, WIDTH as i32 - 44, HEIGHT as i32 - 42, 0x33291C);
    screen.centered_text(&assets.font, book.title(), 27);
    for (row, line) in pages[page].iter().enumerate() {
        screen.text(&assets.font, line, 29, 42 + row as i32 * 8);
    }
    screen.centered_text(
        &assets.font,
        &format!("< PAGE {}/{} >  SELECT CLOSE", page + 1, pages.len()),
        HEIGHT as i32 - 25,
    );
}

fn render_sign_editor(screen: &mut Screen, assets: &Assets, text: &str) {
    screen.rect(36, 72, WIDTH as i32 - 72, 69, 0x101018);
    screen.centered_text(&assets.font, "EDIT SIGN", 81);
    let visible = text.chars().take(28).collect::<String>();
    screen.rect(47, 99, WIDTH as i32 - 94, 13, 0x493A28);
    screen.centered_text(&assets.font, &format!("{visible}_"), 102);
    screen.centered_text(&assets.font, "TYPE  BACKSPACE  SELECT SAVE", 124);
}

fn short_progress_id(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

fn damage_tile(
    level: &mut Level,
    index: usize,
    damage: u16,
    health: u16,
    replacement: Tile,
) -> bool {
    let total = level.data[index].saturating_add(damage);
    if total >= health {
        level.tiles[index] = replacement;
        level.data[index] = 0;
        true
    } else {
        level.data[index] = total;
        false
    }
}

fn furniture_for_item(item: ItemId) -> Option<FurnitureKind> {
    Some(match item {
        ItemId::Workbench => FurnitureKind::Workbench,
        ItemId::Oven => FurnitureKind::Oven,
        ItemId::Furnace => FurnitureKind::Furnace,
        ItemId::Anvil => FurnitureKind::Anvil,
        ItemId::Enchanter => FurnitureKind::Enchanter,
        ItemId::Loom => FurnitureKind::Loom,
        ItemId::Chest => FurnitureKind::Chest,
        ItemId::DungeonChest => FurnitureKind::DungeonChest,
        ItemId::Lantern => FurnitureKind::Lantern,
        ItemId::IronLantern => FurnitureKind::IronLantern,
        ItemId::GoldLantern => FurnitureKind::GoldLantern,
        ItemId::Tnt => FurnitureKind::Tnt,
        ItemId::Bed => FurnitureKind::Bed,
        ItemId::Composter => FurnitureKind::Composter,
        ItemId::CowSpawner => FurnitureKind::CowSpawner,
        ItemId::PigSpawner => FurnitureKind::PigSpawner,
        ItemId::SheepSpawner => FurnitureKind::SheepSpawner,
        ItemId::SlimeSpawner => FurnitureKind::SlimeSpawner,
        ItemId::ZombieSpawner => FurnitureKind::ZombieSpawner,
        ItemId::CreeperSpawner => FurnitureKind::CreeperSpawner,
        ItemId::SkeletonSpawner => FurnitureKind::SkeletonSpawner,
        ItemId::SnakeSpawner => FurnitureKind::SnakeSpawner,
        ItemId::KnightSpawner => FurnitureKind::KnightSpawner,
        _ => return None,
    })
}

fn item_for_furniture(kind: FurnitureKind) -> Option<ItemId> {
    Some(match kind {
        FurnitureKind::Workbench => ItemId::Workbench,
        FurnitureKind::Oven => ItemId::Oven,
        FurnitureKind::Furnace => ItemId::Furnace,
        FurnitureKind::Anvil => ItemId::Anvil,
        FurnitureKind::Enchanter => ItemId::Enchanter,
        FurnitureKind::Loom => ItemId::Loom,
        FurnitureKind::Chest => ItemId::Chest,
        FurnitureKind::DungeonChest => ItemId::DungeonChest,
        FurnitureKind::Lantern => ItemId::Lantern,
        FurnitureKind::IronLantern => ItemId::IronLantern,
        FurnitureKind::GoldLantern => ItemId::GoldLantern,
        FurnitureKind::Tnt => ItemId::Tnt,
        FurnitureKind::Bed => ItemId::Bed,
        FurnitureKind::Composter => ItemId::Composter,
        FurnitureKind::CowSpawner => ItemId::CowSpawner,
        FurnitureKind::PigSpawner => ItemId::PigSpawner,
        FurnitureKind::SheepSpawner => ItemId::SheepSpawner,
        FurnitureKind::SlimeSpawner => ItemId::SlimeSpawner,
        FurnitureKind::ZombieSpawner => ItemId::ZombieSpawner,
        FurnitureKind::CreeperSpawner => ItemId::CreeperSpawner,
        FurnitureKind::SkeletonSpawner => ItemId::SkeletonSpawner,
        FurnitureKind::SnakeSpawner => ItemId::SnakeSpawner,
        FurnitureKind::KnightSpawner => ItemId::KnightSpawner,
        FurnitureKind::KnightStatue => return None,
    })
}

fn tile_progress_name(tile: Tile) -> String {
    match tile {
        Tile::Wheat => "wheat".to_owned(),
        Tile::Potato => "potato".to_owned(),
        _ => tile.asset_name().replace('_', " ").to_ascii_lowercase(),
    }
}

fn tile_placement(item: ItemId, target: Tile) -> Option<(Tile, u16)> {
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
    let mob_count = level.entities.mob_count() + level.pending_spawns.len();
    if mob_count >= level.max_mob_count {
        return;
    }
    let skip = spawn::spawn_skip_chance(mob_count, level.max_mob_count);
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
            level.pending_spawns.push(spawn::SpawnIntent {
                kind: spawn::choose_hostile(level.depth, roll),
                x: pixel_x,
                y: pixel_y,
            });
            return;
        }
        if spawn::passive_allowed(level.depth, tile) {
            level.pending_spawns.push(spawn::SpawnIntent {
                kind: spawn::choose_passive(day_tick >= 48_600, roll),
                x: pixel_x,
                y: pixel_y,
            });
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

fn wrap_index(current: usize, direction: i32, length: usize) -> usize {
    (current as i32 + direction).rem_euclid(length as i32) as usize
}

fn player_sprite_frame(direction: Direction, walk_distance: u32) -> (usize, bool) {
    let frame = walk_distance as usize / 8 % 2;
    match direction {
        Direction::Down => (0, frame == 1),
        Direction::Up => (16, frame == 1),
        Direction::Left => (32 + frame * 16, true),
        Direction::Right => (32 + frame * 16, false),
    }
}

fn render_attack_slash(
    screen: &mut Screen,
    hud: &crate::gfx::Image,
    x: i32,
    y: i32,
    direction: Direction,
) {
    let mut part = |source_x, draw_x, draw_y, flip_x, flip_y| {
        screen.blit_region_transformed(hud, source_x, 0, 8, 8, draw_x, draw_y, flip_x, flip_y);
    };
    match direction {
        Direction::Up => {
            part(24, x, y - 4, false, false);
            part(24, x + 8, y - 4, true, false);
        }
        Direction::Left => {
            part(32, x - 4, y, true, false);
            part(32, x - 4, y + 8, true, true);
        }
        Direction::Right => {
            part(32, x + 12, y, false, false);
            part(32, x + 12, y + 8, false, true);
        }
        Direction::Down => {
            part(24, x, y + 12, false, true);
            part(24, x + 8, y + 12, true, true);
        }
    }
}

fn render_score_overlay(
    screen: &mut Screen,
    assets: &Assets,
    ticks: u32,
    score: u32,
    multiplier: u8,
) {
    let total_seconds = ticks.div_ceil(60);
    let total_minutes = total_seconds / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let seconds = total_seconds % 60;
    let time = format!(
        "TIME LEFT {}{}m {}s",
        if hours > 0 {
            format!("{hours}h ")
        } else {
            String::new()
        },
        minutes,
        seconds
    );
    screen.text(&assets.font, &time, WIDTH as i32 / 2 - 72, 2);
    let score = format!("CURRENT SCORE: {score}");
    screen.text(
        &assets.font,
        &score,
        WIDTH as i32 - score.chars().count() as i32 * 8 - 2,
        11,
    );
    if multiplier > 1 {
        let multiplier = format!("X{multiplier}");
        screen.text(
            &assets.font,
            &multiplier,
            WIDTH as i32 - multiplier.chars().count() as i32 * 8 - 2,
            20,
        );
    }
}

fn render_quest_overlay(screen: &mut Screen, assets: &Assets, progress: &str) {
    let label = progress.chars().take(30).collect::<String>();
    let width =
        (((label.chars().count().max(6) as i32 * 8 + 32) / 8) * 8).clamp(72, WIDTH as i32 - 18);
    let x = WIDTH as i32 - 9 - width;
    render_menu_frame(screen, &assets.hud, x, 9, width, 24);
    screen.text(&assets.font, "QUESTS", x + (width - 48) / 2, 9);
    screen.text(&assets.font, &label, x + 16, 17);
}

fn render_potion_overlay(screen: &mut Screen, assets: &Assets, player: &Player, offset: i32) {
    for (row, kind) in PotionKind::ALL
        .into_iter()
        .filter(|kind| player.potion_effects[kind.id()] > 0)
        .enumerate()
    {
        let total_seconds = u32::from(player.potion_effects[kind.id()]) / 60;
        let label = format!(
            "{} ({}:{:02})",
            kind.display_name(),
            total_seconds / 60,
            total_seconds % 60
        );
        let x = 180;
        let y = 17 + offset + row as i32 * 8;
        screen.rect(
            x - 2,
            y - 1,
            label.chars().count() as i32 * 8 + 4,
            10,
            0x101018,
        );
        screen.text(&assets.font, &label, x, y);
    }
}

fn render_boss_bar(
    screen: &mut Screen,
    assets: &Assets,
    health: u16,
    max_health: u16,
    title: &str,
) {
    let x = WIDTH as i32 / 4 - 24;
    let y = HEIGHT as i32 / 8 - 24;
    let length = if max_health == 0 {
        0
    } else {
        usize::from(health) * 100 / usize::from(max_health)
    };
    screen.blit_region_transformed(&assets.hud, 0, 32, 8, 8, x + 200, y, true, false);
    for column in 0..100 {
        screen.blit_region(&assets.hud, x + column * 2, y, 24, 32, 8, 8, false);
    }
    screen.blit_region(&assets.hud, x - 5, y, 0, 40, 8, 8, false);
    for column in 0..length {
        screen.blit_region(&assets.hud, x + column as i32 * 2, y, 24, 40, 8, 8, false);
    }
    screen.centered_text(&assets.font, title, y + 8);
}

#[allow(clippy::too_many_arguments)]
fn render_hud(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    mode: GameMode,
    _seed: i64,
    _depth: i8,
    _day_tick: u32,
    _days: u32,
) {
    let bottom = HEIGHT as i32;

    if mode != GameMode::Creative || player.active_item.is_some() {
        for x in 10..26 {
            screen.blit_region(&assets.hud, x * 8, bottom - 8, 40, 16, 8, 8, false);
        }
    }

    if let Some(active) = player.active_item {
        let (image, label) = match active {
            ActiveItem::Stack(item) => (Some(assets.item(item)), item.display_name().to_owned()),
            ActiveItem::Tool(index) => player
                .inventory
                .tools()
                .get(index)
                .map(|tool| (assets.tool(*tool), tool.display_name()))
                .map_or((None, String::new()), |(image, label)| (Some(image), label)),
        };
        if let Some(image) = image {
            screen.blit(image, 80, bottom - 8);
            screen.text(&assets.font, &label, 88, bottom - 8);
        }
    }

    if let Some(ActiveItem::Tool(index)) = player.active_item
        && let Some(tool) = player.inventory.tools().get(index)
    {
        let durability = u32::from(tool.durability) * 100 / u32::from(tool.max_durability.max(1));
        screen.text(&assets.font, &format!("{durability}%"), 164, bottom - 16);
        if tool.kind == ToolKind::Bow {
            screen.text(
                &assets.font,
                &format!(" x{}", player.inventory.count(ItemId::Arrow)),
                84,
                bottom - 16,
            );
            screen.blit_region(&assets.hud, 84, bottom - 16, 32, 8, 8, 8, false);
        }
    }
    if player.active_item == Some(ActiveItem::Stack(ItemId::WateringCan)) {
        let content = u32::from(player.watering_content) * 100 / 1_800;
        screen.text(&assets.font, &format!("{content}%"), 164, bottom - 16);
    }

    let effects = PotionKind::ALL
        .into_iter()
        .filter(|effect| player.potion_effects[effect.id()] > 0)
        .map(|effect| {
            format!(
                "{} {}",
                &effect.display_name()[..effect.display_name().len().min(3)],
                player.potion_effects[effect.id()].div_ceil(60)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    if !effects.is_empty() {
        let width = effects.chars().count() as i32 * 8 + 4;
        screen.rect(WIDTH as i32 - width - 2, 8, width, 10, 0x101018);
        screen.text(&assets.font, &effects, WIDTH as i32 - width, 9);
    }

    for index in 0..10 {
        let health_source_y = if player.health > 20 {
            24
        } else if player.health > 10 {
            16
        } else {
            0
        };
        screen.blit_region(&assets.hud, index * 8, bottom - 16, 0, 8, 8, 8, false);
        let tier_start = if player.health > 20 {
            20
        } else if player.health > 10 {
            10
        } else {
            0
        };
        if usize::from(player.health.saturating_sub(tier_start)) > index as usize {
            screen.blit_region(
                &assets.hud,
                index * 8,
                bottom - 16,
                0,
                health_source_y,
                8,
                8,
                false,
            );
        }

        if let Some(armor) = player.armor_kind {
            let armor_icons = u16::from(player.armor) * 10 / u16::from(armor.durability().max(1));
            if index <= i32::from(armor_icons) {
                screen.blit(assets.item(armor.item()), index * 8, bottom - 24);
            }
        }

        let stamina_source_y = if player.stamina_recharge_delay > 0 {
            if (player.stamina_recharge_delay / 4).is_multiple_of(2) {
                16
            } else {
                8
            }
        } else if index < i32::from(player.stamina) {
            0
        } else {
            8
        };
        screen.blit_region(
            &assets.hud,
            index * 8,
            bottom - 8,
            8,
            stamina_source_y,
            8,
            8,
            false,
        );
        screen.blit_region(
            &assets.hud,
            WIDTH as i32 - 80 + index * 8,
            bottom - 16,
            16,
            if index < i32::from(player.hunger) {
                0
            } else {
                8
            },
            8,
            8,
            false,
        );
    }
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

#[cfg(test)]
fn time_name(tick: u32) -> &'static str {
    match tick / (DAY_LENGTH / 4) {
        0 => "MORN",
        1 => "DAY",
        2 => "EVE",
        _ => "NIGHT",
    }
}

#[allow(clippy::too_many_arguments)]
fn render_inventory(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    item_selection: usize,
    craft_selection: usize,
    pane: usize,
    station: Option<FurnitureKind>,
    personal_crafting: bool,
    mode: GameMode,
) {
    if personal_crafting || station.is_some() {
        render_crafting_menu(screen, assets, player, station, craft_selection);
        return;
    }

    let mut player_entries = player
        .inventory
        .slots()
        .iter()
        .map(|stack| {
            (
                assets.item(stack.item),
                format!(" {} {}", stack.count.min(999), stack.item.display_name()),
            )
        })
        .collect::<Vec<_>>();
    player_entries.extend(
        player
            .inventory
            .tools()
            .iter()
            .map(|tool| (assets.tool(*tool), format!(" {}", tool.display_name()))),
    );

    let player_width = inventory_panel_width(&player_entries, "INVENTORY");
    if mode != GameMode::Creative || pane == 0 {
        render_inventory_panel(
            screen,
            assets,
            &player_entries,
            "INVENTORY",
            9,
            Some(item_selection),
        );
        render_inventory_counter(
            screen,
            &assets.inventory_counter,
            9 + player_width,
            9,
            player_entries.len(),
            player.inventory.capacity(),
        );
        if mode != GameMode::Creative {
            screen.text_colored(&assets.font, "Z: Crafting", 17, 97, 0x686868);
        }
        return;
    }

    let creative = creative_entries();
    let creative_entries = creative
        .iter()
        .map(|entry| match entry {
            CreativeEntry::Stack(item) => (assets.item(*item), format!(" {}", item.display_name())),
            CreativeEntry::Tool(tool) => (assets.tool(*tool), format!(" {}", tool.display_name())),
        })
        .collect::<Vec<_>>();
    let creative_width = inventory_panel_width(&creative_entries, "ITEMS");
    let creative_x = WIDTH as i32 - 10 - creative_width;
    let player_x = creative_x - 10 - player_width;
    render_inventory_panel(screen, assets, &player_entries, "INVENTORY", player_x, None);
    render_inventory_panel(
        screen,
        assets,
        &creative_entries,
        "ITEMS",
        creative_x,
        Some(craft_selection),
    );
    render_inventory_counter_minimized(
        screen,
        &assets.inventory_counter,
        player_x + player_width,
        9,
        player_entries.len(),
    );
}

fn creative_entries() -> Vec<CreativeEntry> {
    let mut entries = ItemId::ALL
        .iter()
        .copied()
        .filter(|item| *item != ItemId::PowerGlove)
        .map(CreativeEntry::Stack)
        .collect::<Vec<_>>();
    for kind in ToolKind::ALL {
        for tier in ToolTier::ALL {
            if kind != ToolKind::Shears || tier == ToolTier::Wood {
                entries.push(CreativeEntry::Tool(ToolItem::new(kind, tier)));
            }
        }
    }
    entries
}

fn inventory_panel_width(entries: &[(&crate::gfx::Image, String)], title: &str) -> i32 {
    let content_width = entries
        .iter()
        .map(|(_, label)| label.chars().count())
        .chain([title.len()])
        .max()
        .unwrap_or(title.len()) as i32
        * 8;
    ((content_width + 48).clamp(80, WIDTH as i32 - 18) / 8) * 8
}

fn render_inventory_panel(
    screen: &mut Screen,
    assets: &Assets,
    entries: &[(&crate::gfx::Image, String)],
    title: &str,
    x: i32,
    selection: Option<usize>,
) {
    let width = inventory_panel_width(entries, title);
    let (y, height) = (9, 104);
    render_menu_frame(screen, &assets.hud, x, y, width, height);
    render_menu_title(screen, assets, title, x, y, width, 0xFFFFFF);

    let selected = selection.unwrap_or(0);
    let first = selected
        .saturating_sub(4)
        .min(entries.len().saturating_sub(9));
    for (row, (image, label)) in entries.iter().skip(first).take(9).enumerate() {
        let index = first + row;
        let entry_x = x + 24;
        let entry_y = y + 8 + row as i32 * 8;
        if selection == Some(index) {
            screen.text(&assets.font, ">", entry_x - 8, entry_y);
            screen.text(
                &assets.font,
                "<",
                entry_x + label.chars().count() as i32 * 8,
                entry_y,
            );
        }
        screen.text(&assets.font, label, entry_x, entry_y);
        screen.blit(image, entry_x, entry_y);
    }
}

fn render_crafting_menu(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    station: Option<FurnitureKind>,
    selection: usize,
) {
    let recipes = recipe_views(station);
    let title = station.map_or("Crafting", FurnitureKind::display_name);
    let content_width = recipes
        .iter()
        .map(|(output, _)| output.chars().count())
        .chain([title.len()])
        .max()
        .unwrap_or(title.len()) as i32
        * 8;
    let width = (content_width + 48).clamp(96, 176);
    let (x, y, height) = (9, 9, 88);
    render_menu_frame(screen, &assets.hud, x, y, width, height);
    render_menu_title(screen, assets, title, x, y, width, 0xFFFFFF);
    let first = selection
        .saturating_sub(4)
        .min(recipes.len().saturating_sub(9));
    for (row, (output, _)) in recipes.iter().skip(first).take(9).enumerate() {
        let index = first + row;
        let entry_x = x + 24;
        let entry_y = y + 8 + row as i32 * 8;
        if index == selection {
            screen.text(&assets.font, ">", entry_x - 8, entry_y);
            screen.text(
                &assets.font,
                "<",
                entry_x + output.chars().count() as i32 * 8,
                entry_y,
            );
        }
        screen.text(&assets.font, output, entry_x, entry_y);
    }

    if let Some((output, costs)) = recipes.get(selection) {
        let panel_x = x + width + 8;
        let panel_width = ((WIDTH as i32 - panel_x - 9) / 8) * 8;
        if panel_width >= 56 {
            render_menu_frame(screen, &assets.hud, panel_x, y, panel_width, 32);
            render_menu_title(screen, assets, "HAVE", panel_x, y, panel_width, 0xFFFFFF);
            screen.text(
                &assets.font,
                &format!("{} {}", output_count(player, output), output),
                panel_x + 8,
                y + 12,
            );
            let cost_height = (costs.len().max(1) as i32 * 8 + 16).max(32);
            render_menu_frame(
                screen,
                &assets.hud,
                panel_x,
                y + 40,
                panel_width,
                cost_height,
            );
            render_menu_title(
                screen,
                assets,
                "COST",
                panel_x,
                y + 40,
                panel_width,
                0xFFFFFF,
            );
            for (row, cost) in costs.iter().enumerate() {
                screen.blit(assets.item(cost.item), panel_x + 8, y + 52 + row as i32 * 8);
                screen.text(
                    &assets.font,
                    &format!(" {}/{}", player.inventory.count(cost.item), cost.count),
                    panel_x + 8,
                    y + 52 + row as i32 * 8,
                );
            }
        }
    }
}

fn output_count(player: &Player, output: &str) -> usize {
    let tool_count = player
        .inventory
        .tools()
        .iter()
        .filter(|tool| tool.display_name() == output)
        .count();
    if tool_count > 0 {
        return tool_count;
    }
    ItemId::ALL
        .iter()
        .copied()
        .filter(|item| {
            output == item.display_name()
                || output
                    .strip_prefix(item.display_name())
                    .is_some_and(|suffix| suffix.starts_with(" x"))
        })
        .max_by_key(|item| item.display_name().len())
        .map_or(0, |item| usize::from(player.inventory.count(item)))
}

fn render_menu_frame(
    screen: &mut Screen,
    hud: &crate::gfx::Image,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let right = x + width - 8;
    let bottom = y + height - 8;
    for draw_y in (y..=bottom).step_by(8) {
        for draw_x in (x..=right).step_by(8) {
            let edge_x = draw_x == x || draw_x == right;
            let edge_y = draw_y == y || draw_y == bottom;
            let source_x = if edge_x && edge_y {
                0
            } else if edge_y {
                8
            } else if edge_x {
                16
            } else {
                24
            };
            screen.blit_region_transformed(
                hud,
                source_x,
                48,
                8,
                8,
                draw_x,
                draw_y,
                draw_x == right,
                draw_y == bottom,
            );
        }
    }
}

fn render_menu_title(
    screen: &mut Screen,
    assets: &Assets,
    title: &str,
    x: i32,
    y: i32,
    width: i32,
    color: u32,
) {
    let title_width = title.chars().count() as i32 * 8;
    let title_x = x + (width - title_width) / 2;
    for offset in (0..title_width).step_by(8) {
        screen.blit_region(&assets.hud, title_x + offset, y, 24, 48, 8, 8, false);
    }
    screen.text_colored(&assets.font, title, title_x, y, color);
}

fn render_inventory_counter(
    screen: &mut Screen,
    counter: &crate::gfx::Image,
    right: i32,
    top: i32,
    size: usize,
    capacity: usize,
) {
    let background_x = if size < 10 { right - 16 } else { right - 21 };
    let background_width = if size < 10 { 18 } else { 23 };
    screen.blit_region(
        counter,
        background_x,
        top - 3,
        56 - background_width as usize,
        12,
        background_width as usize,
        13,
        false,
    );
    render_counter_digits(
        screen,
        counter,
        right - if size < 10 { 14 } else { 19 },
        top - 1,
        5,
        5,
        7,
        size,
    );
    render_counter_digits(screen, counter, right - 8, top + 3, 0, 4, 5, capacity);
}

fn render_inventory_counter_minimized(
    screen: &mut Screen,
    counter: &crate::gfx::Image,
    right: i32,
    top: i32,
    size: usize,
) {
    if size < 10 {
        screen.blit_region(counter, right - 12, top - 1, 0, 12, 4, 9, false);
        screen.blit_region(counter, right - 8, top - 1, 8, 12, 4, 9, false);
        render_counter_digits(screen, counter, right - 10, top + 1, 0, 4, 5, size);
    } else {
        screen.blit_region(counter, right - 16, top - 1, 0, 12, 12, 9, false);
        render_counter_digits(screen, counter, right - 14, top + 1, 0, 4, 5, size);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_counter_digits(
    screen: &mut Screen,
    counter: &crate::gfx::Image,
    x: i32,
    y: i32,
    source_y: usize,
    width: usize,
    height: usize,
    number: usize,
) {
    for (index, digit) in number.to_string().bytes().enumerate() {
        screen.blit_region(
            counter,
            x + index as i32 * width as i32,
            y,
            usize::from(digit - b'0') * width,
            source_y,
            width,
            height,
            false,
        );
    }
}

fn recipe_views(station: Option<FurnitureKind>) -> Vec<(String, &'static [ItemStack])> {
    let stack = |recipe: &crate::item::Recipe| {
        (
            format!("{} x{}", recipe.output.item, recipe.output.count),
            recipe.costs,
        )
    };
    let tool = |recipe: &crate::item::ToolRecipe| (recipe.output.display_name(), recipe.costs);
    match station {
        None => HAND_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Workbench) => WORKBENCH_STATION_RECIPES
            .iter()
            .map(stack)
            .chain(WORKBENCH_TOOL_RECIPES.iter().map(tool))
            .collect(),
        Some(FurnitureKind::Oven) => OVEN_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Furnace) => FURNACE_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Anvil) => ANVIL_RECIPES
            .iter()
            .map(stack)
            .chain(ANVIL_TOOL_RECIPES.iter().map(tool))
            .collect(),
        Some(FurnitureKind::Enchanter) => ENCHANTER_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Loom) => LOOM_RECIPES.iter().map(stack).collect(),
        _ => Vec::new(),
    }
}

fn render_entity(
    screen: &mut Screen,
    assets: &Assets,
    entity: &entity::Entity,
    camera_x: i32,
    camera_y: i32,
    tick: u64,
) {
    match &entity.kind {
        EntityKind::Item(stack) => {
            let image = assets.item(stack.item);
            let Some(motion) = &entity.item_motion else {
                screen.blit(
                    image,
                    entity.x - image.width as i32 / 2 - camera_x,
                    entity.y - image.height as i32 / 2 - camera_y,
                );
                return;
            };
            if entity.age >= motion.lifetime.saturating_sub(120)
                && (entity.age / 6).is_multiple_of(2)
            {
                return;
            }
            let x = entity.x - image.width as i32 / 2 - camera_x;
            let ground_y = entity.y - image.height as i32 / 2 - camera_y;
            screen.blit_tinted(image, x, ground_y, 0x202020);
            screen.blit(image, x, ground_y - motion.height as i32);
        }
        EntityKind::Mob(mob) => {
            if mob.hurt_time > 0 && tick.is_multiple_of(2) {
                return;
            }
            let image = assets.mob(mob.species);
            let frame = mob.walk_distance as usize / 8 % 2;
            let (source_x, flip) = if image.width >= 64 {
                if mob.y_move < 0 {
                    (16, frame == 1)
                } else if mob.x_move < 0 {
                    (32 + frame * 16, true)
                } else if mob.x_move > 0 {
                    (32 + frame * 16, false)
                } else {
                    (0, frame == 1)
                }
            } else {
                (frame * 16, false)
            };
            let source_y = if mob.species == spawn::NaturalMob::Sheep && mob.sheared {
                32
            } else {
                0
            };
            let jump = if mob.species == spawn::NaturalMob::Slime && entity.age % 20 < 10 {
                -4
            } else {
                0
            };
            screen.blit_region(
                image,
                entity.x - 8 - camera_x,
                entity.y - 11 - camera_y + jump,
                source_x,
                source_y,
                16,
                16,
                flip,
            );
            if mob.health < mob.max_health {
                screen.rect(
                    entity.x - 7 - camera_x,
                    entity.y - 14 - camera_y,
                    14,
                    2,
                    0x3A1010,
                );
                screen.rect(
                    entity.x - 7 - camera_x,
                    entity.y - 14 - camera_y,
                    14 * i32::from(mob.health) / i32::from(mob.max_health),
                    2,
                    0xD84848,
                );
            }
            if matches!(
                mob.species,
                spawn::NaturalMob::AirWizard | spawn::NaturalMob::ObsidianKnight
            ) {
                let percent = (mob.health.saturating_mul(100) / mob.max_health).max(1);
                screen.text(
                    &assets.font,
                    &format!("{percent}%"),
                    entity.x - 12 - camera_x,
                    entity.y - 23 - camera_y,
                );
            }
        }
        EntityKind::Furniture(kind) => {
            screen.blit(
                assets.furniture(*kind),
                entity.x - 8 - camera_x,
                entity.y - 8 - camera_y,
            );
        }
        EntityKind::Projectile(projectile) => {
            let color = match projectile.kind {
                entity::ProjectileKind::Arrow => 0xD8C48A,
                entity::ProjectileKind::Spark => 0xC8E8FF,
                entity::ProjectileKind::FireSpark => 0xFF6A24,
            };
            screen.rect(
                entity.x - 2 - camera_x,
                entity.y - 2 - camera_y,
                4,
                4,
                color,
            );
        }
        EntityKind::Particle(kind) => match kind {
            entity::ParticleKind::Smash => {
                screen.blit(&assets.smash, entity.x - camera_x, entity.y - camera_y);
            }
            entity::ParticleKind::Fire => {
                screen.rect(entity.x - camera_x, entity.y - camera_y, 2, 2, 0xFF8A30);
            }
        },
        EntityKind::TextParticle(text) => {
            let (offset_x, offset_y, z) = text_particle_motion(entity.id, entity.age);
            let x = entity.x - camera_x - text.chars().count() as i32 * 4 + offset_x;
            let y = entity.y - camera_y + offset_y - z;
            screen.text_colored(&assets.font, text, x + 1, y + 1, 0x000000);
            screen.text_colored(&assets.font, text, x, y, 0xFF3030);
        }
    }
}

fn text_particle_motion(id: u64, age: u32) -> (i32, i32, i32) {
    let seed = id.wrapping_mul(0x9E37_79B9);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 2.0;
    let mut x_velocity = (seed % 601) as f64 / 1000.0 - 0.3;
    let mut y_velocity = (seed.rotate_left(17) % 401) as f64 / 1000.0 - 0.2;
    let mut z_velocity = 2.0 + (seed.rotate_left(31) % 701) as f64 / 1000.0;
    for _ in 0..age {
        x += x_velocity;
        y += y_velocity;
        z += z_velocity;
        if z < 0.0 {
            z = 0.0;
            z_velocity *= -0.5;
            x_velocity *= 0.6;
            y_velocity *= 0.6;
        }
        z_velocity -= 0.15;
    }
    (x as i32, y as i32, z as i32)
}

fn render_pause(screen: &mut Screen, assets: &Assets, world: &World) {
    let (x, y, width, height) = (48, 32, 192, 128);
    render_menu_frame(screen, &assets.hud, x, y, width, height);
    let (title, labels): (&str, Vec<String>) = match world.pause_page {
        PausePage::Main => (
            "Paused",
            pause_actions(world.quests_enabled)
                .into_iter()
                .map(|action| match action {
                    PauseAction::Return => "Return to Game",
                    PauseAction::Options => "Options",
                    PauseAction::Achievements => "Achievements",
                    PauseAction::Quests => "Quests",
                    PauseAction::Save => "Save Game",
                    PauseAction::MainMenu => "Main Menu",
                })
                .map(str::to_owned)
                .collect(),
        ),
        PausePage::Options => (
            "Options",
            vec![
                format!(
                    "Difficulty: {}",
                    ["Easy", "Normal", "Hard"][world.difficulty.min(2)]
                ),
                format!(
                    "Show Quest Panel: {}",
                    if world.show_quests { "On" } else { "Off" }
                ),
                format!(
                    "Tutorials: {}",
                    if world.tutorials_enabled { "On" } else { "Off" }
                ),
                format!(
                    "Quests: {}",
                    if world.quests_enabled { "On" } else { "Off" }
                ),
                "Back".to_owned(),
            ],
        ),
        PausePage::Achievements => (
            "Achievements",
            vec![
                format!("COMPLETED: {}", world.progress.achievement_count()),
                "ENTER OR ESC TO RETURN".to_owned(),
            ],
        ),
        PausePage::Quests => (
            "Quests",
            vec![
                format!(
                    "TUTORIALS: {}/{}",
                    world.progress.tutorial_completed_count(),
                    world.progress.tutorial_count()
                ),
                format!(
                    "QUESTS: {}/{}",
                    world.progress.quest_completed_count(),
                    world.progress.quest_count()
                ),
                "ENTER OR ESC TO RETURN".to_owned(),
            ],
        ),
    };
    render_menu_title(screen, assets, title, x, y, width, 0xFFFF00);
    for (index, label) in labels.iter().enumerate() {
        let draw_y = y + 24 + index as i32 * 12;
        let draw_x = x + (width - label.chars().count() as i32 * 8) / 2;
        if index == world.pause_selection
            && matches!(world.pause_page, PausePage::Main | PausePage::Options)
        {
            screen.text(&assets.font, ">", draw_x - 8, draw_y);
            screen.text(
                &assets.font,
                "<",
                draw_x + label.chars().count() as i32 * 8,
                draw_y,
            );
        }
        screen.text(&assets.font, label, draw_x, draw_y);
    }
    if world.pause_page == PausePage::Main {
        screen.text_colored(
            &assets.font,
            "UP and DOWN to Scroll",
            (WIDTH as i32 - 21 * 8) / 2,
            y + height - 24,
            0x686868,
        );
        screen.text_colored(
            &assets.font,
            "ENTER: Choose",
            (WIDTH as i32 - 13 * 8) / 2,
            y + height - 14,
            0x686868,
        );
    }
    if world.pause_confirm {
        let (popup_x, popup_y, popup_width, popup_height) = (20, 56, 248, 80);
        screen.rect(popup_x, popup_y, popup_width, popup_height, 0x08080C);
        screen.frame(popup_x, popup_y, popup_width, popup_height, 0xC8C8C8);
        screen.centered_text(&assets.font, "Are you sure you want to", popup_y + 8);
        screen.centered_text(&assets.font, "exit the game?", popup_y + 18);
        let warning = "All unsaved progress will be lost";
        screen.text_colored(
            &assets.font,
            warning,
            (WIDTH as i32 - warning.chars().count() as i32 * 8) / 2,
            popup_y + 30,
            0xFF3030,
        );
        for (index, label) in ["Cancel", "Quit without saving"].iter().enumerate() {
            let draw_y = popup_y + 48 + index as i32 * 12;
            let draw_x = (WIDTH as i32 - label.len() as i32 * 8) / 2;
            if index == world.pause_selection {
                screen.text(&assets.font, ">", draw_x - 8, draw_y);
                screen.text(&assets.font, "<", draw_x + label.len() as i32 * 8, draw_y);
            }
            screen.text(&assets.font, label, draw_x, draw_y);
        }
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
        ActiveItem, DAY_LENGTH, Direction, FurnitureKind, GameMode, HUNGER_STAMINA_STEPS, Level,
        MAX_HUNGER_TICKS, PausePage, Player, STARVATION_HEALTH_FLOORS, TILE_SIZE, Tile, World,
        creative_entries,
        entity::{EntityArena, EntityKind},
        player_sprite_frame,
        random::JavaRandom,
        spawn, surface_darkness, time_name, try_queue_natural_spawn,
    };
    use crate::{
        audio::SoundEffect,
        input::Input,
        item::{
            ArmorKind, Inventory, ItemId, PotionKind, ToolItem, ToolKind, ToolTier,
            WORKBENCH_STATION_RECIPES,
        },
    };

    fn tiny_world() -> World {
        World {
            width: 16,
            height: 16,
            levels: vec![Level {
                depth: 0,
                tiles: vec![Tile::Grass; 16 * 16],
                data: vec![0; 16 * 16],
                max_mob_count: 100,
                pending_spawns: Vec::new(),
                entities: EntityArena::default(),
            }],
            current_level: 0,
            player: Player {
                x: 8 * 16 + 8,
                y: 8 * 16 + 8,
                direction: Direction::Right,
                walk_distance: 0,
                attack_time: 0,
                attack_direction: Direction::Down,
                attack_item: None,
                health: 10,
                max_health: 10,
                stamina: 10,
                hunger: 10,
                armor: 0,
                armor_kind: None,
                armor_damage_buffer: 0,
                hurt_time: 0,
                stamina_recharge: 0,
                stamina_recharge_delay: 0,
                hunger_stamina_count: HUNGER_STAMINA_STEPS[1],
                hunger_ticks: MAX_HUNGER_TICKS,
                step_count: 0,
                hunger_charge_delay: 0,
                hunger_starve_delay: 0,
                potion_effects: [0; PotionKind::ALL.len()],
                regen_tick: 0,
                fishing_level: None,
                fishing_ticks: 0,
                watering_content: 0,
                clothing: ItemId::RegularClothes,
                inventory: Inventory::new(27),
                active_item: None,
            },
            seed: 1,
            tick: 0,
            day_tick: 0,
            days: 1,
            difficulty: 1,
            mode: super::GameMode::Survival,
            score: 0,
            score_ticks: 20 * 60 * 60,
            score_multiplier: 1,
            multiplier_ticks: 300,
            game_over: false,
            story_complete: false,
            sleeping: 0,
            tutorials_enabled: true,
            quests_enabled: true,
            show_quests: true,
            progress: crate::content::ProgressState::load().unwrap(),
            signs: vec![std::collections::HashMap::new()],
            sign_editor: None,
            book_open: None,
            paused: false,
            pause_selection: 0,
            pause_page: PausePage::Main,
            pause_confirm: false,
            inventory_open: false,
            inventory_selection: 0,
            inventory_item_selection: 0,
            inventory_pane: 0,
            crafting_station: None,
            personal_crafting: false,
            notification: None,
            air_wizard_defeated: false,
            obsidian_knight_defeated: false,
            random: JavaRandom::new(1),
            sound_events: Vec::new(),
        }
    }

    #[test]
    fn player_skin_frames_match_java_four_direction_animation() {
        assert_eq!(player_sprite_frame(Direction::Down, 0), (0, false));
        assert_eq!(player_sprite_frame(Direction::Down, 8), (0, true));
        assert_eq!(player_sprite_frame(Direction::Up, 0), (16, false));
        assert_eq!(player_sprite_frame(Direction::Up, 8), (16, true));
        assert_eq!(player_sprite_frame(Direction::Left, 0), (32, true));
        assert_eq!(player_sprite_frame(Direction::Left, 8), (48, true));
        assert_eq!(player_sprite_frame(Direction::Right, 0), (32, false));
        assert_eq!(player_sprite_frame(Direction::Right, 8), (48, false));
    }

    #[test]
    fn creative_inventory_is_a_separate_unlimited_catalogue() {
        let mut world = tiny_world();
        world.mode = GameMode::Creative;

        world.tick(&Input {
            menu: true,
            ..Input::default()
        });
        assert!(world.inventory_open);
        assert_eq!(world.inventory_pane, 1);
        assert_eq!(world.player.inventory.capacity(), 27);
        assert_eq!(world.player.inventory.used_slots(), 0);

        assert!(
            creative_entries()
                .iter()
                .all(|entry| !matches!(entry, super::CreativeEntry::Stack(ItemId::PowerGlove)))
        );
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.count(ItemId::Wood), 2);
        assert_eq!(world.player.inventory.used_slots(), 1);

        world.tick(&Input {
            left_pressed: true,
            ..Input::default()
        });
        assert_eq!(world.inventory_pane, 0);
        world.tick(&Input {
            attack: true,
            ..Input::default()
        });
        assert!(!world.inventory_open);
        assert_eq!(
            world.player.active_item,
            Some(ActiveItem::Stack(ItemId::Wood))
        );
    }

    #[test]
    fn enter_equips_inventory_items_and_z_opens_personal_crafting() {
        let mut world = tiny_world();
        world.player.inventory.add(ItemId::Wood, 10);
        world.tick(&Input {
            menu: true,
            ..Input::default()
        });
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert!(!world.inventory_open);
        assert_eq!(
            world.player.active_item,
            Some(ActiveItem::Stack(ItemId::Wood))
        );

        world.tick(&Input {
            craft: true,
            ..Input::default()
        });
        assert!(world.inventory_open);
        assert!(world.personal_crafting);
        assert_eq!(world.crafting_station, None);
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
        assert!(world.take_sound_events().contains(&SoundEffect::Craft));
    }

    #[test]
    fn z_switches_an_open_player_inventory_to_workbench_crafting() {
        let mut world = tiny_world();
        world.player.inventory.add(ItemId::Wood, 10);
        world.tick(&Input {
            menu: true,
            ..Input::default()
        });
        assert!(world.inventory_open && !world.personal_crafting);

        world.tick(&Input {
            craft: true,
            ..Input::default()
        });
        assert!(world.inventory_open && world.personal_crafting);
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.count(ItemId::Wood), 0);
        assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
        assert!(
            world
                .notification
                .as_ref()
                .is_none_or(|(message, _)| !message.starts_with("CRAFTED"))
        );
    }

    #[test]
    fn an_item_equipped_with_enter_can_be_used_after_closing_inventory() {
        let mut world = tiny_world();
        world.player.hunger = 5;
        world.player.inventory.add(ItemId::Apple, 1);
        world.tick(&Input {
            menu: true,
            ..Input::default()
        });
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        world.tick(&Input {
            attack: true,
            ..Input::default()
        });
        assert_eq!(world.player.hunger, 6);
        assert_eq!(world.player.inventory.count(ItemId::Apple), 0);
    }

    #[test]
    fn pickup_uses_the_power_glove_slash_without_subtitles() {
        let mut world = tiny_world();
        world.tutorials_enabled = false;
        world.quests_enabled = false;
        world.notification = None;
        world.tick(&Input {
            pickup: true,
            ..Input::default()
        });
        assert_eq!(world.player.attack_time, 5);
        assert!(world.notification.is_none());

        world.levels[0].entities.spawn_furniture(
            FurnitureKind::Workbench,
            world.player.x + 12,
            world.player.y,
        );
        world.tick(&Input {
            pickup: true,
            ..Input::default()
        });
        assert_eq!(
            world.player.active_item,
            Some(ActiveItem::Stack(ItemId::Workbench))
        );
        assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
        assert!(world.notification.is_none());
        assert!(world.take_sound_events().contains(&SoundEffect::Pickup));
    }

    #[test]
    fn pause_navigation_is_available_to_the_render_frame_without_a_tick_delay() {
        let mut world = tiny_world();
        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        let down = Input {
            down_pressed: true,
            ..Input::default()
        };
        assert_eq!(world.immediate_menu_sound(&down), Some(SoundEffect::Select));
        world.tick(&down);
        let confirm = Input {
            select: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sound(&confirm),
            Some(SoundEffect::Confirm)
        );
        world.tick(&confirm);
        assert!(world.take_sound_events().is_empty());
    }

    #[test]
    fn pause_menu_matches_java_save_and_confirmed_quit_flow() {
        let mut world = tiny_world();
        assert!(matches!(
            world.tick(&Input {
                exit: true,
                ..Input::default()
            }),
            super::WorldAction::None
        ));
        assert!(world.paused);
        world.pause_selection = 4;
        assert!(matches!(
            world.tick(&Input {
                select: true,
                ..Input::default()
            }),
            super::WorldAction::SaveGame
        ));
        assert!(!world.paused);

        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        world.pause_selection = 5;
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert!(world.pause_confirm);
        world.tick(&Input {
            down_pressed: true,
            ..Input::default()
        });
        assert!(matches!(
            world.tick(&Input {
                select: true,
                ..Input::default()
            }),
            super::WorldAction::QuitWithoutSaving
        ));
    }

    #[test]
    fn ordinary_tile_hit_uses_java_damage_timing_and_tool_wear() {
        let mut world = tiny_world();
        let target = 9 + 8 * world.width;
        world.levels[0].tiles[target] = Tile::Tree;
        world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Sword, ToolTier::Wood));
        world.player.active_item = Some(ActiveItem::Tool(0));

        world.attack();

        assert!((1..=3).contains(&world.levels[0].data[target]));
        assert_eq!(world.player.inventory.tools()[0].durability, 51);
        assert_eq!(world.player.attack_time, 5);
        assert_eq!(world.player.walk_distance, 8);
        assert_eq!(world.player.attack_item, Some(ActiveItem::Tool(0)));
        assert_eq!(
            world.take_sound_events(),
            vec![crate::audio::SoundEffect::MonsterHurt]
        );
    }

    #[test]
    fn successful_tool_tile_interaction_keeps_java_ten_tick_slash() {
        let mut world = tiny_world();
        let target = 9 + 8 * world.width;
        world.levels[0].tiles[target] = Tile::Tree;
        world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood));
        world.player.active_item = Some(ActiveItem::Tool(0));

        world.attack();

        assert!((10..=14).contains(&world.levels[0].data[target]));
        assert_eq!(world.player.inventory.tools()[0].durability, 33);
        assert_eq!(world.player.attack_time, 10);
        assert_eq!(
            world.take_sound_events(),
            vec![crate::audio::SoundEffect::MonsterHurt]
        );
    }

    #[test]
    fn workbench_placement_and_tool_use_are_in_the_normal_world_path() {
        let mut world = tiny_world();
        world.player.inventory.add(ItemId::Workbench, 1);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::Workbench));
        world.attack();
        assert_eq!(world.player.inventory.count(ItemId::Workbench), 0);
        assert_eq!(
            world.levels[0].entities.furniture_near(152, 136, 1),
            Some(FurnitureKind::Workbench)
        );

        world.use_target();
        assert!(world.inventory_open);
        assert_eq!(world.crafting_station, Some(FurnitureKind::Workbench));

        world.player.inventory.add(ItemId::Wood, 5);
        world.inventory_selection = WORKBENCH_STATION_RECIPES.len() + 1;
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        let Some(ActiveItem::Tool(tool_index)) = world.player.active_item else {
            panic!("crafted axe was not equipped");
        };
        assert_eq!(
            world.player.inventory.tools()[tool_index].kind,
            ToolKind::Axe
        );
        let damage = world.pay_tool_terrain_damage(ToolKind::Axe, 4).unwrap();
        assert!((10..=14).contains(&damage));
        assert_eq!(world.player.inventory.tools()[tool_index].durability, 33);
        assert_eq!(world.player.stamina, 6);
    }

    #[test]
    fn ore_requires_a_pickaxe_and_consumes_its_durability() {
        let mut world = tiny_world();
        let ore_index = 9 + 8 * 16;
        world.levels[0].tiles[ore_index] = Tile::IronOre;
        world.attack();
        assert_eq!(world.levels[0].data[ore_index], 0);

        let index = world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Pickaxe, ToolTier::Wood))
            .unwrap();
        world.player.active_item = Some(ActiveItem::Tool(index));
        world.attack();
        assert!(world.levels[0].data[ore_index] > 0);
        assert_eq!(world.player.inventory.tools()[index].durability, 37);
    }

    #[test]
    fn furnace_placement_smelts_ore_through_the_station_menu() {
        let mut world = tiny_world();
        world.player.inventory.add(ItemId::Furnace, 1);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::Furnace));
        world.attack();
        assert_eq!(
            world.levels[0].entities.furniture_near(152, 136, 1),
            Some(FurnitureKind::Furnace)
        );
        world.use_target();
        assert_eq!(world.crafting_station, Some(FurnitureKind::Furnace));

        world.player.inventory.add(ItemId::IronOre, 3);
        world.player.inventory.add(ItemId::Coal, 1);
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.count(ItemId::IronOre), 0);
        assert_eq!(world.player.inventory.count(ItemId::Coal), 0);
        assert_eq!(world.player.inventory.count(ItemId::IronIngot), 1);
    }

    #[test]
    fn equipped_food_consumes_stamina_and_restores_hunger() {
        let mut world = tiny_world();
        world.player.hunger = 5;
        world.player.inventory.add(ItemId::CookedPork, 2);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::CookedPork));
        world.tick(&Input {
            attack: true,
            ..Input::default()
        });
        assert_eq!(world.player.hunger, 8);
        assert_eq!(world.player.stamina, 7);
        assert_eq!(world.player.inventory.count(ItemId::CookedPork), 1);
        assert!(world.notification.is_none());
    }

    #[test]
    fn food_uses_java_partial_stamina_payment_and_never_attacks() {
        let mut world = tiny_world();
        let target = 9 + 8 * 16;
        world.levels[0].tiles[target] = Tile::Rock;
        world.player.hunger = 5;
        world.player.stamina = 1;
        world.player.inventory.add(ItemId::Apple, 1);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::Apple));

        world.tick(&Input {
            attack: true,
            ..Input::default()
        });

        assert_eq!(world.player.hunger, 5);
        assert_eq!(world.player.stamina, 0);
        assert_eq!(world.player.inventory.count(ItemId::Apple), 1);
        assert_eq!(world.levels[0].data[target], 0);
    }

    #[test]
    fn every_attack_input_pays_the_java_base_stamina_cost() {
        let mut world = tiny_world();
        world.tick(&Input {
            attack: true,
            ..Input::default()
        });
        assert_eq!(world.player.stamina, 9);
        assert_eq!(world.player.stamina_recharge, 0);

        world.player.stamina = 0;
        world.player.attack_time = 0;
        world.tick(&Input {
            attack: true,
            ..Input::default()
        });
        assert_eq!(world.player.attack_time, 0);
    }

    #[test]
    fn tool_actions_accept_java_partial_stamina_payment_after_the_base_cost() {
        let mut world = tiny_world();
        let target = 9 + 8 * world.width;
        world.levels[0].tiles[target] = Tile::Tree;
        let axe = world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
            .unwrap();
        world.player.active_item = Some(ActiveItem::Tool(axe));
        world.player.stamina = 2;

        world.tick(&Input {
            attack: true,
            ..Input::default()
        });

        assert_eq!(world.player.stamina, 0);
        assert!((10..=14).contains(&world.levels[0].data[target]));
        assert_eq!(world.player.attack_time, 10);
    }

    #[test]
    fn starvation_respects_the_difficulty_health_floor() {
        let mut world = tiny_world();
        world.player.hunger = 0;
        world.player.health = 10;
        world.player.hunger_starve_delay = 1;
        world.tick_survival_stats();
        assert_eq!(world.player.health, 9);

        world.player.health = STARVATION_HEALTH_FLOORS[world.difficulty];
        world.player.hunger_starve_delay = 1;
        world.tick_survival_stats();
        assert_eq!(world.player.health, 3);
    }

    #[test]
    fn armor_and_potions_run_through_the_equipped_item_path() {
        let mut world = tiny_world();
        world.player.inventory.add(ItemId::LeatherArmor, 1);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::LeatherArmor));
        assert!(world.use_active_self_item());
        assert_eq!(world.player.armor_kind, Some(ArmorKind::Leather));
        assert_eq!(world.player.armor, 30);
        assert_eq!(world.player.stamina, 1);
        assert!(world.hurt_player(4, false));
        assert_eq!(world.player.armor, 26);
        assert_eq!(world.player.health, 8);

        world.player.inventory.add(ItemId::SpeedPotion, 1);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::SpeedPotion));
        assert!(world.use_active_self_item());
        assert_eq!(
            world.player.potion_effects[PotionKind::Speed.id()],
            PotionKind::Speed.duration()
        );
        assert_eq!(world.player.inventory.count(ItemId::GlassBottle), 1);
    }

    #[test]
    fn hoe_plant_growth_and_harvest_form_a_playable_farm_loop() {
        let mut world = tiny_world();
        let target = 9 + 8 * 16;
        world.levels[0].tiles[target] = Tile::Dirt;
        let hoe = world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Hoe, ToolTier::Wood))
            .unwrap();
        world.player.active_item = Some(ActiveItem::Tool(hoe));
        world.attack();
        assert_eq!(world.levels[0].tiles[target], Tile::Farmland);
        assert_eq!(world.player.inventory.tools()[hoe].durability, 29);

        world.player.inventory.add(ItemId::WheatSeeds, 1);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::WheatSeeds));
        world.attack();
        assert_eq!(world.levels[0].tiles[target], Tile::Wheat);
        world.levels[0].data[target] = 7 << 3;
        world.player.active_item = None;
        world.attack();
        assert_eq!(world.levels[0].tiles[target], Tile::Farmland);
        for _ in 0..31 {
            world.tick(&Input::default());
        }
        let drops = world.levels[0]
            .entities
            .entities()
            .iter()
            .filter(|entity| matches!(entity.kind, super::EntityKind::Item(_)))
            .map(|entity| (entity.x, entity.y))
            .collect::<Vec<_>>();
        for (x, y) in drops {
            world.levels[0]
                .entities
                .collect_near(x, y, &mut world.player.inventory);
        }
        assert!(world.player.inventory.count(ItemId::Wheat) >= 2);
        assert_eq!(world.player.inventory.count(ItemId::WheatSeeds), 1);
    }

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
        assert!(
            world.levels[0]
                .entities
                .has_mob(spawn::NaturalMob::AirWizard)
        );
        assert!(
            world.levels[5]
                .entities
                .has_furniture(FurnitureKind::KnightStatue)
        );
        assert!(world.levels[2..=5].iter().any(|level| {
            level.entities.entities().iter().any(|entity| {
                matches!(
                    entity.kind,
                    super::EntityKind::Furniture(kind) if kind.spawner_mob().is_some()
                )
            })
        }));
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
            entities: crate::world::entity::EntityArena::default(),
        };
        let mut random = JavaRandom::new(7);
        try_queue_natural_spawn(&mut level, 128, 128, 8, 8, 0, 1, &mut random);
        assert_eq!(level.pending_spawns.len(), 1);
        try_queue_natural_spawn(&mut level, 128, 128, 8, 8, 0, 1, &mut random);
        assert_eq!(level.pending_spawns.len(), 1);
    }

    #[test]
    fn phase_six_modes_apply_their_distinct_runtime_rules() {
        let mut creative = tiny_world();
        creative.mode = GameMode::Creative;
        creative.player.inventory.add(ItemId::Workbench, 1);
        creative.player.active_item = Some(ActiveItem::Stack(ItemId::Workbench));
        creative.attack();
        assert_eq!(creative.player.inventory.count(ItemId::Workbench), 1);
        assert!(!creative.hurt_player(5, false));
        assert_eq!(creative.player.health, 10);

        let mut score = tiny_world();
        score.mode = GameMode::Score;
        score.add_score(50, 1);
        assert_eq!(score.score, 50);
        assert_eq!(score.score_multiplier, 2);
        score.add_score(50, 0);
        assert_eq!(score.score, 150);

        let mut hardcore = tiny_world();
        hardcore.mode = GameMode::Hardcore;
        hardcore.player.health = 0;
        hardcore.tick(&Input::default());
        assert!(hardcore.game_over);
    }

    #[test]
    fn beds_signs_and_world_books_are_interactive() {
        let mut world = tiny_world();
        world.levels[0].entities.spawn_furniture(
            FurnitureKind::Bed,
            9 * TILE_SIZE + 8,
            8 * TILE_SIZE + 8,
        );
        world.day_tick = 48_600;
        world.use_target();
        assert_eq!(world.sleeping, 120);

        world.sleeping = 0;
        world.player.direction = Direction::Down;
        let sign = 8 + 9 * 16;
        world.levels[0].tiles[sign] = Tile::Sign;
        world.use_target();
        assert!(world.sign_editor.is_some());
        let typing = Input {
            text: "HELLO".chars().collect(),
            ..Input::default()
        };
        assert!(world.tick_sign_editor(&typing));
        let save = Input {
            select: true,
            ..Input::default()
        };
        assert!(world.tick_sign_editor(&save));
        assert_eq!(world.signs[0].get(&sign).map(String::as_str), Some("HELLO"));

        world.player.inventory.add(ItemId::AntidiousBook, 1);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::AntidiousBook));
        assert!(world.use_active_self_item());
        assert!(world.book_open.is_some());
    }

    #[test]
    fn versioned_world_snapshot_round_trips_complete_runtime_state() {
        let mut world = tiny_world();
        world.tick = 1_801;
        world.day_tick = 49_000;
        world.days = 7;
        world.mode = GameMode::Score;
        world.score = 4_321;
        world.score_multiplier = 9;
        world.player.health = 6;
        let _ = world.player.inventory.set_capacity(32);
        world.player.inventory.add(ItemId::Gem, 12);
        world.signs[0].insert(7, "ROUND TRIP".to_owned());
        world
            .progress
            .unlock_achievement("minicraft.achievement.find_gem");
        world.levels[0]
            .entities
            .spawn_furniture(FurnitureKind::Chest, 40, 56);

        let text = world.to_save_string().unwrap();
        let mut loaded = World::from_save_string(&text).unwrap();
        assert_eq!(loaded.tick, 1_801);
        assert_eq!(loaded.day_tick, 49_000);
        assert_eq!(loaded.days, 7);
        assert_eq!(loaded.mode, GameMode::Score);
        assert_eq!(loaded.score, 4_321);
        assert_eq!(loaded.score_multiplier, 9);
        assert_eq!(loaded.player.health, 6);
        assert_eq!(loaded.player.inventory.capacity(), 27);
        assert_eq!(loaded.player.inventory.count(ItemId::Gem), 12);
        assert_eq!(
            loaded.signs[0].get(&7).map(String::as_str),
            Some("ROUND TRIP")
        );
        assert!(
            loaded
                .progress
                .achievement_unlocked("minicraft.achievement.find_gem")
        );
        assert!(
            loaded.levels[0]
                .entities
                .has_furniture(FurnitureKind::Chest)
        );
        assert_eq!(
            world.random.next_int(10_000),
            loaded.random.next_int(10_000)
        );

        loaded.levels[0].tiles.pop();
        let corrupt = loaded.to_save_string().unwrap();
        assert!(World::from_save_string(&corrupt).is_err());
    }

    #[test]
    fn oversized_legacy_inventory_is_migrated_to_27_slots_without_deleting_stacks() {
        let mut world = tiny_world();
        let _ = world.player.inventory.set_capacity(32);
        for item in ItemId::ALL.iter().copied().take(30) {
            assert_eq!(world.player.inventory.add(item, 1), 0);
        }
        let total_before = world
            .player
            .inventory
            .slots()
            .iter()
            .map(|stack| usize::from(stack.count))
            .sum::<usize>();

        let loaded = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        let held = loaded
            .player
            .inventory
            .slots()
            .iter()
            .map(|stack| usize::from(stack.count))
            .sum::<usize>();
        let dropped = loaded.levels[loaded.current_level]
            .entities
            .entities()
            .iter()
            .filter_map(|entity| match entity.kind {
                EntityKind::Item(stack) => Some(usize::from(stack.count)),
                _ => None,
            })
            .sum::<usize>();
        assert_eq!(loaded.player.inventory.capacity(), 27);
        assert_eq!(loaded.player.inventory.used_slots(), 27);
        assert_eq!(held + dropped, total_before);
    }

    #[test]
    fn malformed_world_snapshot_mutations_never_panic() {
        let valid = tiny_world().to_save_string().unwrap().into_bytes();
        for case in 0..512_usize {
            let mut mutated = valid.clone();
            if case % 3 == 0 {
                mutated.truncate(case.wrapping_mul(7_919) % mutated.len());
            } else if case % 3 == 1 {
                let index = case.wrapping_mul(104_729) % mutated.len();
                mutated[index] ^= 1 << (case % 8);
            } else {
                let index = case.wrapping_mul(65_537) % mutated.len();
                mutated.splice(index..index, [b'{', b'[', 0xff, b']', b'}']);
            }
            let text = String::from_utf8_lossy(&mutated);
            assert!(
                std::panic::catch_unwind(|| World::from_save_string(&text)).is_ok(),
                "mutation {case} panicked"
            );
        }
    }

    #[test]
    fn save_resume_is_deterministic_and_two_day_soak_stays_bounded() {
        let mut continuous = tiny_world();
        continuous.mode = GameMode::Creative;
        continuous.tutorials_enabled = false;
        continuous.quests_enabled = false;
        continuous.levels[0].max_mob_count = 0;

        for _ in 0..10_000 {
            continuous.tick(&Input::default());
        }
        let mut resumed = World::from_save_string(&continuous.to_save_string().unwrap()).unwrap();
        for _ in 0..10_000 {
            continuous.tick(&Input::default());
            resumed.tick(&Input::default());
        }
        assert_eq!(
            continuous.to_save_string().unwrap(),
            resumed.to_save_string().unwrap()
        );

        let started = std::time::Instant::now();
        for _ in 20_000..(DAY_LENGTH as usize * 2) {
            continuous.tick(&Input::default());
        }
        assert_eq!(continuous.tick, u64::from(DAY_LENGTH) * 2);
        assert_eq!(continuous.days, 3);
        assert_eq!(continuous.day_tick, 0);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(15),
            "headless two-day soak exceeded the generous regression budget"
        );
        assert!(continuous.levels[0].entities.entities().len() < 32);
    }
}
