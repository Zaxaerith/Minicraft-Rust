use minicraft_client::core::notifications::Notifications;
use minicraft_client::core::updater::Updater;
use minicraft_client::screen::pause::{PauseAction, PausePage, PauseSession, pause_actions};

mod clock_save;
mod core_update;
mod end_game;
mod level_update;
pub(crate) mod notifications;
mod pause;
mod pause_save;
mod player_update;
mod render;
mod sleep;

mod entity;
mod generation;
mod java_save;
mod quest_tree;
pub(crate) mod random;
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
    localization::Localization,
};

pub use entity::FurnitureKind;
use entity::{EntityArena, EntityKind};
pub use java_save::{import_java_save, probe_java_save};
use random::JavaRandom;

const TILE_SIZE: i32 = 16;
const DAY_LENGTH: u32 = minicraft_client::core::updater::DAY_LENGTH;
const MAX_STAT: u8 = 10;
fn default_render_gui() -> bool {
    true
}
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

pub(crate) struct CraftingParityState {
    pub capacity: usize,
    pub used_slots: usize,
    pub selection: usize,
    pub items: Vec<(String, u32)>,
    pub tools: Vec<(String, u32)>,
    pub recipe_order: Vec<String>,
    pub craftable: Vec<bool>,
    pub selected_have: usize,
    pub selected_costs: Vec<(String, u16, u16)>,
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

fn direction_for_move(horizontal: i32, vertical: i32, current: Direction) -> Direction {
    if horizontal == 0 && vertical == 0 {
        return current;
    }
    if horizontal.abs() > vertical.abs() {
        if horizontal < 0 {
            Direction::Left
        } else {
            Direction::Right
        }
    } else if vertical < 0 {
        Direction::Up
    } else {
        Direction::Down
    }
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

#[derive(Serialize, Deserialize)]
pub struct World {
    width: usize,
    height: usize,
    levels: Vec<Level>,
    current_level: usize,
    player: Player,
    seed: i64,
    tick: u64,
    #[serde(flatten, with = "clock_save")]
    clock: minicraft_client::core::clock::SharedClock,
    #[serde(skip, default = "default_render_gui")]
    render_gui: bool,
    #[serde(default, with = "core_update")]
    updater: Updater,
    days: u32,
    difficulty: usize,
    mode: GameMode,
    score: u32,
    score_ticks: i32,
    score_multiplier: u8,
    multiplier_ticks: u16,
    game_over: bool,
    #[serde(default, with = "end_game")]
    end_game: Option<minicraft_client::screen::end_game::EndGameDisplay>,
    story_complete: bool,
    sleeping: u16,
    #[serde(default)]
    bed_level: Option<usize>,
    #[serde(default)]
    respawn_tile: Option<(i32, i32)>,
    #[serde(skip)]
    sleep_speed: u8,
    tutorials_enabled: bool,
    quests_enabled: bool,
    show_quests: bool,
    progress: ProgressState,
    signs: Vec<HashMap<usize, String>>,
    sign_editor: Option<SignEditor>,
    book_open: Option<(Book, usize)>,
    #[serde(flatten, with = "pause_save")]
    pause: PauseSession,
    #[serde(skip)]
    quest_tree_state: quest_tree::State,
    inventory_open: bool,
    inventory_selection: usize,
    inventory_item_selection: usize,
    inventory_pane: usize,
    crafting_station: Option<FurnitureKind>,
    #[serde(default)]
    crafting_order: Vec<usize>,
    #[serde(default)]
    personal_crafting: bool,
    #[serde(default, rename = "notification", with = "notifications")]
    notifications: Notifications,
    #[serde(skip)]
    notification_context: notifications::Context,
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
        pause_save::normalize(&mut world.pause, world.quests_enabled);
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

    pub fn achievement_unlocked(&self, id: &str) -> bool {
        self.progress.achievement_unlocked(id)
    }

    pub fn take_sound_events(&mut self) -> Vec<SoundEffect> {
        std::mem::take(&mut self.sound_events)
    }

    /// Menu feedback is sampled on the render/input frame so it does not wait for
    /// the next fixed 60 Hz world tick. Gameplay sounds remain tick-driven.
    pub fn immediate_menu_sounds(&self, input: &Input) -> Vec<SoundEffect> {
        if self.pause.is_open() {
            if input.exit {
                // Game.exitDisplay() in 2.2.4 always plays craft.wav when closing a display.
                return if self.pause.view().page == PausePage::Achievements {
                    // AchievementsDisplay.onExit() follows Game.exitDisplay().
                    vec![SoundEffect::Craft, SoundEffect::Confirm]
                } else {
                    vec![SoundEffect::Craft]
                };
            }
            if self.pause.view().page == PausePage::QuestSeriesViewer {
                let mut selection = self.pause.view().selection;
                let mut state = self.quest_tree_state.clone();
                return match self.quest_tree().tick(&mut selection, &mut state, input) {
                    quest_tree::Action::Moved => vec![SoundEffect::Select],
                    quest_tree::Action::Open => vec![SoundEffect::Confirm],
                    quest_tree::Action::None => Vec::new(),
                };
            }
            if input.select {
                if matches!(
                    self.pause.view().page,
                    PausePage::Achievements | PausePage::QuestDetail
                ) {
                    // Achievement entries have a null SelectEntry callback.
                    return Vec::new();
                }
                if self.pause.view().page == PausePage::Quests
                    && self.quest_group_views_for_current_tab().is_empty()
                {
                    return Vec::new();
                }
                if self.pause.view().confirm && self.pause.view().selection == 0 {
                    // SelectEntry plays confirm before Cancel calls Game.exitDisplay().
                    return vec![SoundEffect::Confirm, SoundEffect::Craft];
                }
                return vec![SoundEffect::Confirm];
            }
            if input.up_pressed || input.down_pressed {
                let selectable = match self.pause.view().page {
                    PausePage::Quests => !self.quest_group_views_for_current_tab().is_empty(),
                    PausePage::QuestDetail => false,
                    _ => true,
                };
                if selectable {
                    return vec![SoundEffect::Select];
                }
            }
            if self.pause.view().page == PausePage::Options
                && (input.left_pressed || input.right_pressed)
            {
                return vec![SoundEffect::Select];
            }
        } else if self.inventory_open {
            if input.exit || input.menu || (self.personal_crafting && input.craft) {
                return vec![SoundEffect::Craft];
            }
            if input.up_pressed || input.down_pressed || input.left_pressed || input.right_pressed {
                return vec![SoundEffect::Select];
            }
            if self.personal_crafting || self.crafting_station.is_some() {
                if input.select || input.attack {
                    let recipes = ordered_recipe_views(
                        self.crafting_station,
                        &self.player.inventory,
                        self.mode == GameMode::Creative,
                        &self.crafting_order,
                    );
                    if recipes.get(self.inventory_selection).is_some_and(|recipe| {
                        recipe_can_craft(
                            recipe,
                            &self.player.inventory,
                            self.mode == GameMode::Creative,
                        )
                    }) {
                        return vec![SoundEffect::Craft];
                    }
                }
            } else if (input.select || input.attack)
                && self.inventory_item_selection < self.player.inventory.used_slots()
            {
                // PlayerInvDisplay equips the selected item and closes through
                // Game.exitDisplay(), which plays craft.wav in Java 2.2.4.
                return vec![SoundEffect::Craft];
            }
        }
        Vec::new()
    }

    pub(crate) fn pause_state_for_parity(&self) -> (bool, &'static str, usize) {
        if !self.pause.is_open() {
            return (false, "gameplay", 0);
        }
        if self.pause.view().confirm {
            return (true, "exit_confirm", self.pause.view().selection);
        }
        let screen = match self.pause.view().page {
            PausePage::Main => "pause",
            PausePage::Options => "pause_options",
            PausePage::Achievements => "achievements",
            PausePage::Quests => "quests",
            PausePage::QuestSeriesInfo => "quest_series_info",
            PausePage::QuestSeriesViewer => "quest_series_viewer",
            PausePage::QuestDetail => "quest_detail",
        };
        let selection = if self.pause.view().page == PausePage::Quests {
            self.pause.view().quest_tab
        } else {
            self.pause.view().selection
        };
        (true, screen, selection)
    }

    pub(crate) fn prepare_player_motion_parity(&mut self) {
        let level = &mut self.levels[self.current_level];
        level.tiles.fill(Tile::Grass);
        level.data.fill(0);
        level.pending_spawns.clear();
        level.entities = EntityArena::default();
        self.player.x = (self.width as i32 / 2) * TILE_SIZE + 8;
        self.player.y = (self.height as i32 / 2) * TILE_SIZE + 8;
        self.player.direction = Direction::Down;
        self.player.walk_distance = 0;
        self.player.step_count = 0;
        self.player.stamina = MAX_STAT;
        self.player.stamina_recharge = 0;
        self.player.stamina_recharge_delay = 0;
        self.player.hunger = MAX_STAT;
        self.player.hunger_ticks = MAX_HUNGER_TICKS;
        self.player.hunger_stamina_count = HUNGER_STAMINA_STEPS[self.difficulty];
        self.tick = 0;
        self.clock.set_day_tick(0);
        self.updater = Updater::default();
    }

    pub(crate) fn set_player_stats_for_parity(
        &mut self,
        stamina: u8,
        stamina_recharge: u8,
        stamina_recharge_delay: u8,
    ) {
        self.player.stamina = stamina.min(MAX_STAT);
        self.player.stamina_recharge = stamina_recharge;
        self.player.stamina_recharge_delay = stamina_recharge_delay;
    }

    pub(crate) fn set_obstacle_for_parity(&mut self, dx: i32, dy: i32, solid: bool) {
        let tile_x = self.player.x.div_euclid(TILE_SIZE) + dx;
        let tile_y = self.player.y.div_euclid(TILE_SIZE) + dy;
        if tile_x < 0 || tile_y < 0 {
            return;
        }
        let index = tile_x as usize + tile_y as usize * self.width;
        if index < self.levels[self.current_level].tiles.len() {
            self.levels[self.current_level].tiles[index] =
                if solid { Tile::Rock } else { Tile::Grass };
            self.levels[self.current_level].data[index] = 0;
        }
    }

    pub(crate) fn player_motion_state_for_parity(
        &self,
    ) -> (i32, i32, &'static str, u32, u8, u8, u8, u8) {
        let direction = match self.player.direction {
            Direction::Down => "down",
            Direction::Up => "up",
            Direction::Left => "left",
            Direction::Right => "right",
        };
        (
            self.player.x,
            self.player.y,
            direction,
            self.player.walk_distance,
            self.player.stamina,
            self.player.stamina_recharge,
            self.player.stamina_recharge_delay,
            self.player.step_count,
        )
    }

    pub(crate) fn prepare_combat_parity(&mut self, seed: i64, tool: Option<ToolKind>) {
        self.prepare_player_motion_parity();
        self.random = JavaRandom::new(seed);
        self.player.direction = Direction::Right;
        self.player.attack_time = 0;
        self.player.attack_item = None;
        self.player.inventory = Inventory::new(27);
        self.player.active_item = tool.map(|kind| {
            let index = self
                .player
                .inventory
                .add_tool(ToolItem::new(kind, ToolTier::Wood))
                .expect("empty parity inventory accepts one tool");
            ActiveItem::Tool(index)
        });
        let tile_x = self.player.x.div_euclid(TILE_SIZE) + 1;
        let tile_y = self.player.y.div_euclid(TILE_SIZE);
        let index = tile_x as usize + tile_y as usize * self.width;
        self.levels[self.current_level].tiles[index] = Tile::Tree;
        self.levels[self.current_level].data[index] = 0;
        self.sound_events.clear();
    }

    /// Runs only the deterministic Player.tick portions surrounding attack input.
    /// Random tile ticks, natural spawning, and unrelated entity AI have their own
    /// parity adapters and would otherwise obscure the combat random streams.
    pub(crate) fn combat_attack_for_parity(&mut self) {
        self.player.attack_time = self.player.attack_time.saturating_sub(1);
        if self.player.attack_time == 0 {
            self.player.attack_item = None;
        }
        self.tick_survival_stats();
        if self.player.stamina > 0 {
            self.player.stamina -= 1;
            self.player.stamina_recharge = 0;
            self.attack();
        }
    }

    pub(crate) fn combat_state_for_parity(
        &self,
    ) -> (String, u16, u8, u8, u32, i32, u32, Vec<String>) {
        let tile_x = self.player.x.div_euclid(TILE_SIZE) + 1;
        let tile_y = self.player.y.div_euclid(TILE_SIZE);
        let index = tile_x as usize + tile_y as usize * self.width;
        let level = &self.levels[self.current_level];
        let tool_durability = self
            .active_tool()
            .map_or(-1, |tool| i32::from(tool.durability));
        let mut smash_count = 0_u32;
        let mut text_values = Vec::new();
        for entity in level.entities.entities() {
            match &entity.kind {
                EntityKind::Particle(entity::ParticleKind::Smash) => smash_count += 1,
                EntityKind::TextParticle(text) => text_values.push(text.clone()),
                _ => {}
            }
        }
        let tile = match level.tiles[index] {
            Tile::Tree => "tree".to_owned(),
            Tile::Grass => "grass".to_owned(),
            tile => tile_progress_name(tile),
        };
        (
            tile,
            level.data[index],
            self.player.stamina,
            self.player.attack_time,
            self.player.walk_distance,
            tool_durability,
            smash_count,
            text_values,
        )
    }

    pub(crate) fn prepare_crafting_parity(
        &mut self,
        workbench: bool,
        wood: u16,
        coal: u16,
        stone: u16,
        string: u16,
    ) {
        self.prepare_player_motion_parity();
        self.player.inventory = Inventory::new(27);
        for (item, count) in [
            (ItemId::Wood, wood),
            (ItemId::Coal, coal),
            (ItemId::Stone, stone),
            (ItemId::String, string),
        ] {
            if count > 0 {
                self.player.inventory.add(item, count);
            }
        }
        self.player.active_item = None;
        self.pause.close();
        self.pause.begin_tick();
        self.inventory_open = true;
        self.personal_crafting = !workbench;
        self.crafting_station = workbench.then_some(FurnitureKind::Workbench);
        self.inventory_pane = 1;
        self.inventory_selection = 0;
        self.refresh_crafting_order();
        self.sound_events.clear();
    }

    pub(crate) fn prepare_inventory_audio_parity(&mut self) {
        self.prepare_player_motion_parity();
        self.player.inventory = Inventory::new(27);
        self.player.inventory.add(ItemId::Wood, 3);
        self.player.inventory.add(ItemId::Coal, 2);
        self.player.active_item = None;
        self.pause.close();
        self.pause.begin_tick();
        self.inventory_open = true;
        self.personal_crafting = false;
        self.crafting_station = None;
        self.inventory_pane = 0;
        self.inventory_item_selection = 0;
        self.sound_events.clear();
    }

    pub(crate) fn prepare_pause_audio_parity(&mut self) {
        self.prepare_player_motion_parity();
        self.inventory_open = false;
        self.personal_crafting = false;
        self.crafting_station = None;
        self.pause.open();
        self.pause.begin_tick();
        self.sound_events.clear();
    }

    /// Settle a static preview fixture at a Display update boundary. This is
    /// never called by the production renderer and does not tick the world.
    pub(crate) fn settle_pause_preview(&mut self) {
        self.pause.begin_tick();
    }

    pub(crate) fn prepare_quests_audio_parity(&mut self) {
        self.prepare_pause_audio_parity();
        self.progress = ProgressState::load().expect("bundled progression data must be valid");
        self.quest_tree_state = quest_tree::State::default();
        self.quests_enabled = true;
        self.player.inventory = Inventory::new(27);
        self.player.inventory.add(ItemId::WheatSeeds, 5);
        self.update_progress(ProgressEvent::InventoryChanged);
        self.pause.view_mut().selection = 3;
        self.pause.push(PausePage::Quests, 0);
        self.pause.begin_tick();
        self.pause.view_mut().quest_tab = 0;
        self.pause.view_mut().quest_group = 0;
        self.pause.view_mut().quest_item = 0;
        self.pause.view_mut().quest_series_selection = 0;
    }

    pub(crate) fn complete_hoe_quest_for_parity(&mut self) {
        self.player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Hoe, ToolTier::Wood));
        self.update_progress(ProgressEvent::InventoryChanged);
    }

    fn quest_group_views_for_current_tab(&self) -> Vec<crate::content::QuestGroupView> {
        self.progress
            .quest_group_views(&self.inventory_progress(), self.pause.view().quest_tab == 1)
    }

    fn quest_views_for_selected_group(&self) -> Vec<crate::content::QuestView> {
        self.progress
            .quest_views(self.pause.view().quest_group, &self.inventory_progress())
    }

    /// Refresh runtime geometry from the same localization used by rendering.
    pub(crate) fn prepare_quest_layout(&mut self, localization: &Localization) {
        if self.pause.is_open()
            && matches!(
                self.pause.view().page,
                PausePage::QuestSeriesInfo | PausePage::QuestSeriesViewer | PausePage::QuestDetail
            )
        {
            self.quest_tree_state.labels = self
                .quest_views_for_selected_group()
                .iter()
                .map(|quest| (quest.id.clone(), localization.text(&quest.id).to_owned()))
                .collect();
        }
    }

    fn quest_tree(&self) -> quest_tree::Tree {
        quest_tree::Tree::new(&self.quest_views_for_selected_group(), |id| {
            self.quest_tree_state
                .labels
                .get(id)
                .map_or_else(|| id.to_owned(), Clone::clone)
        })
    }

    pub(crate) fn audio_menu_state_for_parity(&self) -> (&'static str, usize) {
        if self.pause.is_open() {
            let (_, screen, selection) = self.pause_state_for_parity();
            return (screen, selection);
        }
        if self.inventory_open {
            if self.personal_crafting || self.crafting_station.is_some() {
                return ("crafting", self.inventory_selection);
            }
            return ("inventory", self.inventory_item_selection);
        }
        ("gameplay", 0)
    }

    pub(crate) fn select_crafting_product_for_parity(
        &mut self,
        product: &str,
    ) -> Result<(), String> {
        let recipes = ordered_recipe_views(
            self.crafting_station,
            &self.player.inventory,
            self.mode == GameMode::Creative,
            &self.crafting_order,
        );
        self.inventory_selection = recipes
            .iter()
            .position(|recipe| recipe.product_name() == product)
            .ok_or_else(|| format!("crafting product {product:?} is unavailable"))?;
        Ok(())
    }

    pub(crate) fn crafting_state_for_parity(&self) -> CraftingParityState {
        let recipes = ordered_recipe_views(
            self.crafting_station,
            &self.player.inventory,
            self.mode == GameMode::Creative,
            &self.crafting_order,
        );
        let selected = recipes.get(self.inventory_selection);
        let item_count = |item| u32::from(self.player.inventory.count(item));
        let items = [
            ItemId::Wood,
            ItemId::Coal,
            ItemId::Stone,
            ItemId::String,
            ItemId::Plank,
            ItemId::Workbench,
        ]
        .into_iter()
        .map(|item| (item.display_name().to_owned(), item_count(item)))
        .collect();
        let wood_axes = self
            .player
            .inventory
            .tools()
            .iter()
            .filter(|tool| tool.kind == ToolKind::Axe && tool.tier == ToolTier::Wood)
            .count() as u32;
        CraftingParityState {
            capacity: self.player.inventory.capacity(),
            used_slots: self.player.inventory.used_slots(),
            selection: self.inventory_selection,
            items,
            tools: vec![("Wood Axe".to_owned(), wood_axes)],
            recipe_order: recipes.iter().map(RecipeView::product_name).collect(),
            craftable: recipes
                .iter()
                .map(|recipe| {
                    recipe_can_craft(
                        recipe,
                        &self.player.inventory,
                        self.mode == GameMode::Creative,
                    )
                })
                .collect(),
            selected_have: selected.map_or(0, |recipe| recipe.output_count(&self.player)),
            selected_costs: selected.map_or_else(Vec::new, |recipe| {
                recipe
                    .costs
                    .iter()
                    .map(|cost| {
                        (
                            cost.item.display_name().to_owned(),
                            self.player.inventory.count(cost.item),
                            cost.count,
                        )
                    })
                    .collect()
            }),
        }
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
            clock: Default::default(),
            render_gui: true,
            updater: Updater::default(),
            days: 1,
            difficulty,
            mode: options.mode,
            score: 0,
            score_ticks: (match options.score_minutes {
                10 | 20 | 40 | 60 | 120 => options.score_minutes,
                _ => 20,
            } * 60
                * 60) as i32,
            score_multiplier: 1,
            multiplier_ticks: 300,
            game_over: false,
            end_game: None,
            story_complete: false,
            sleeping: 0,
            bed_level: None,
            respawn_tile: Some((spawn_x, spawn_y)),
            sleep_speed: 1,
            tutorials_enabled: options.tutorials,
            quests_enabled: options.quests,
            show_quests: options.show_quests,
            progress,
            signs: (0..sign_levels).map(|_| HashMap::new()).collect(),
            sign_editor: None,
            book_open: None,
            pause: PauseSession::default(),
            quest_tree_state: quest_tree::State::default(),
            inventory_open: false,
            inventory_selection: 0,
            inventory_item_selection: 0,
            inventory_pane: 0,
            crafting_station: None,
            crafting_order: Vec::new(),
            personal_crafting: false,
            notifications: Notifications::default(),
            notification_context: notifications::Context::default(),
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
        world.notify(&format!("DEPTH {depth} PREVIEW"), 0);
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
        self.refresh_crafting_order();
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
        self.refresh_crafting_order();
        self.inventory_item_selection = 3;
        self.inventory_selection = 2;
        self.notify("FOOD SURVIVAL PREVIEW", 0);
    }

    /// Opens the Java-style player inventory for visual regression.
    pub fn populate_inventory_preview(&mut self) {
        self.player.inventory = Inventory::new(27);
        self.player.active_item = None;
        self.player.inventory.add(ItemId::Wood, 20);
        self.player.inventory.add(ItemId::Stone, 10);
        self.player.inventory.add(ItemId::Coal, 4);
        self.player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood));
        self.inventory_open = true;
        self.personal_crafting = false;
        self.crafting_station = None;
        self.inventory_pane = 0;
        self.inventory_item_selection = 1;
    }

    /// Opens the Java personal crafting display (Z / Shift+E) for visual regression.
    pub fn populate_personal_crafting_preview(&mut self) {
        self.player.inventory = Inventory::new(27);
        self.player.active_item = None;
        self.player.inventory.add(ItemId::Wood, 20);
        self.player.inventory.add(ItemId::Coal, 4);
        self.inventory_open = true;
        self.personal_crafting = true;
        self.crafting_station = None;
        self.inventory_pane = 1;
        self.refresh_crafting_order();
        self.inventory_selection = 1;
    }

    /// Creates a deterministic Java-style HUD fixture without changing terrain.
    pub fn populate_hud_parity_preview(&mut self) {
        self.mode = GameMode::Survival;
        self.player.inventory = Inventory::new(27);
        let mut axe = ToolItem::new(ToolKind::Axe, ToolTier::Wood);
        axe.durability = 17;
        let index = self
            .player
            .inventory
            .add_tool(axe)
            .expect("empty parity inventory accepts the HUD tool");
        self.player.active_item = Some(ActiveItem::Tool(index));
        self.player.health = 7;
        self.player.stamina = 6;
        self.player.hunger = 5;
        self.player.stamina_recharge_delay = 0;
        self.player.armor = 0;
        self.player.armor_kind = None;
        self.player.potion_effects.fill(0);
    }

    /// Opens the Java PauseDisplay main-menu confirmation popup fixture.
    pub fn populate_exit_confirm_parity_preview(&mut self) {
        self.quests_enabled = false;
        self.pause.open();
        self.pause.begin_tick();
        self.pause.view_mut().selection = pause_actions(false).len() - 1;
        self.pause.confirm_exit();
        self.pause.begin_tick();
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
        self.notify("SCORE MODE PREVIEW", 0);
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
        self.notify("DATA-DRIVEN PROGRESS ACTIVE", 0);
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
        self.notifications.clear();
    }

    pub fn populate_pause_preview(&mut self) {
        self.pause.open();
        self.pause.begin_tick();
        self.pause.view_mut().selection = 1;
        self.notifications.clear();
    }

    pub fn tick(&mut self, input: &Input) -> WorldAction {
        self.begin_display_tick();
        self.tick_after_display(true, input)
    }

    pub(crate) fn tick_after_display(&mut self, focused: bool, input: &Input) -> WorldAction {
        self.tick_with_autosave(focused, input, false, |_| Ok(()))
            .expect("disabled autosave has no fallible side effects")
    }

    fn tick_after_prelude(&mut self, focused: bool, input: &Input) -> WorldAction {
        if !self.update_calendar_and_score(focused) {
            return WorldAction::None;
        }
        self.tick_player_display_controls(input);
        // Once activated, the terminal Display owns input instead of the
        // previously open book/sign/inventory compatibility host.
        if self.game_over
            && self.mode == GameMode::Score
            && self.end_game.as_ref().is_some_and(|display| display.active)
        {
            return self.tick_end_game(input);
        }
        if self.tick_sign_editor(input) || self.tick_book(input) {
            return WorldAction::None;
        }
        if self.game_over {
            return self.tick_end_game(input);
        }
        if self.sleeping > 0 {
            self.tick_sleeping_level();
            return WorldAction::None;
        }
        if input.exit {
            if self.inventory_open {
                self.inventory_open = false;
                self.personal_crafting = false;
                self.crafting_station = None;
            } else if self.pause.is_open() {
                self.pause.back();
            } else {
                self.pause.open();
            }
            return WorldAction::None;
        }
        if self.pause.is_open() {
            return self.tick_pause(input);
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
                self.refresh_crafting_order();
            } else if !self.inventory_open {
                self.inventory_open = true;
                self.personal_crafting = true;
                self.player.active_item = None;
                self.crafting_station = None;
                self.inventory_selection = 0;
                self.refresh_crafting_order();
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
        if self.mode == GameMode::Score && self.score_multiplier > 1 {
            self.multiplier_ticks = self.multiplier_ticks.saturating_sub(1);
            if self.multiplier_ticks == 0 {
                self.score_multiplier = 1;
                self.multiplier_ticks = 300;
            }
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
            self.clock.get().day_tick,
            self.days,
            &mut self.random,
        );
        let time_slowed = self.effect_active(PotionKind::Time);
        let outcome = self.tick_level_entities(
            Some((self.player.x, self.player.y)),
            time_slowed,
            self.mode == GameMode::Creative,
        );
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

        let horizontal = i32::from(input.right) - i32::from(input.left);
        let vertical = i32::from(input.down) - i32::from(input.up);

        if horizontal != 0 || vertical != 0 {
            let on_water = self.tile_at_pixel(self.player.x, self.player.y) == Tile::Water;
            let has_swim = self.effect_active(PotionKind::Swim);
            // Player.tick only calls Mob.move on even recovery-delay ticks (or
            // while swimming). Mob.move then skips even swimming ticks before
            // changing direction/walkDist, but reports movement to stepCount.
            if self.player.stamina_recharge_delay.is_multiple_of(2) || on_water {
                let moved = if on_water && !has_swim && self.tick.is_multiple_of(2) {
                    true
                } else {
                    self.player.direction =
                        direction_for_move(horizontal, vertical, self.player.direction);
                    self.player.walk_distance = self.player.walk_distance.wrapping_add(1);
                    // Java casts each component of +/-1 * 1.5 back to int, so
                    // the Speed potion does not turn ordinary movement into 2 px.
                    self.move_player(horizontal, vertical)
                };
                if moved {
                    self.player.step_count = self.player.step_count.saturating_add(1);
                }
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

    fn refresh_crafting_order(&mut self) {
        let recipes = recipe_views(self.crafting_station);
        let creative = self.mode == GameMode::Creative;
        let mut order: Vec<usize> = (0..recipes.len()).collect();
        order.sort_by(|&left, &right| {
            recipe_can_craft(&recipes[right], &self.player.inventory, creative).cmp(
                &recipe_can_craft(&recipes[left], &self.player.inventory, creative),
            )
        });
        self.crafting_order = order;
    }

    fn craft_selected_recipe(&mut self) {
        let recipes = ordered_recipe_views(
            self.crafting_station,
            &self.player.inventory,
            self.mode == GameMode::Creative,
            &self.crafting_order,
        );
        let Some(recipe) = recipes.get(self.inventory_selection).copied() else {
            return;
        };
        // craft.wav is dispatched by immediate_menu_sounds on the input frame;
        // queueing it here would play it again on the next fixed world tick.
        match recipe.output {
            RecipeOutput::Stack(output) => self.craft_stack(crate::item::Recipe {
                output,
                costs: recipe.costs,
            }),
            RecipeOutput::Tool(output) => self.craft_tool(crate::item::ToolRecipe {
                output,
                costs: recipe.costs,
                tool_cost: recipe.tool_cost,
            }),
        };
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
                self.add_notification("minicraft.notification.wrong_level_sky");
            } else if self.levels[self.current_level]
                .entities
                .has_mob(spawn::NaturalMob::AirWizard)
            {
                self.add_notification("minicraft.notification.boss_limit");
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
                self.add_notification(if self.levels[self.current_level].depth != -4 {
                    "minicraft.notification.wrong_level_dungeon"
                } else {
                    "minicraft.notification.spawn_on_boss_tile"
                });
            } else if self.levels[self.current_level]
                .entities
                .has_mob(spawn::NaturalMob::ObsidianKnight)
            {
                self.add_notification("minicraft.notification.boss_limit");
            } else if self.levels[self.current_level]
                .entities
                .has_furniture(FurnitureKind::KnightStatue)
            {
                self.add_notification("minicraft.notification.knight_statue_exists");
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
                self.notify("Health increase is at max!", 0);
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
                self.notify("minicraft.notification.air_wizard_defeated", 0);
                if !self.air_wizard_defeated {
                    self.unlock_achievement("minicraft.achievement.airwizard");
                    self.notify("minicraft.notification.dungeon_opened", -400);
                }
                self.air_wizard_defeated = true;
            }
            spawn::NaturalMob::ObsidianKnight => {
                self.sound(SoundEffect::BossDeath);
                self.obsidian_knight_defeated = true;
                self.story_complete = true;
                self.notify("minicraft.notification.obsidian_knight_defeated", 0);
                self.unlock_achievement("minicraft.achievement.obsidianknight");
            }
            _ => {}
        }
    }

    fn unlock_achievement(&mut self, id: &str) {
        if self.progress.unlock_achievement(id) {
            let label = self.notification_context.locale.text(id).to_owned();
            self.add_notification_args("minicraft.notification.achievement_unlocked", &[&label]);
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
                    self.add_notification("You can't escape from here!");
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
        if self.sleeping > 0 {
            return false;
        }
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
        self.apply_bumped_tile_damage(next_x, self.player.y);
        if self.can_stand(next_x, self.player.y) {
            self.player.x = next_x;
        }
        let next_y = self.player.y + vertical;
        self.apply_bumped_tile_damage(self.player.x, next_y);
        if self.can_stand(self.player.x, next_y) {
            self.player.y = next_y;
        }
        self.player.x != old_x || self.player.y != old_y
    }

    fn apply_bumped_tile_damage(&mut self, x: i32, y: i32) {
        let damage = [(-4, -3), (4, -3), (-4, 4), (4, 4)]
            .into_iter()
            .map(|(offset_x, offset_y)| self.tile_at_pixel(x + offset_x, y + offset_y))
            .map(|tile| match tile {
                Tile::Cactus => {
                    if self.difficulty.min(2) == 2 {
                        2
                    } else {
                        1
                    }
                }
                Tile::LavaBrick => 3,
                _ => 0,
            })
            .max()
            .unwrap_or(0);
        if damage > 0 {
            self.hurt_player(damage, false);
        }
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

        if self.levels[self.current_level]
            .entities
            .mob_near(self.player.x + offset_x, self.player.y + offset_y)
        {
            let mut damage = (self.random.next_int(2) + 1) as u8;
            damage = damage.saturating_add(self.pay_tool_melee_bonus());
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
                let damage = tool_damage.unwrap_or_else(|| (self.random.next_int(3) + 1) as u16);
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
                    self.add_notification("minicraft.notification.defeat_obsidian_knight_first");
                }
            }
            Tile::Cactus => {
                let bare_damage = (self.random.next_int(3) + 1) as u16;
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
                let damage = tool_damage.unwrap_or_else(|| (self.random.next_int(3) + 1) as u16);
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
                        // Java HardRockTile adds this key literally, without localization.
                        self.notifications
                            .add("minicraft.notification.gem_pickaxe_required");
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
                    self.add_notification("minicraft.notification.defeat_air_wizard_first");
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
                    self.add_notification("minicraft.notification.defeat_obsidian_knight_first");
                    return;
                }
                let Some(damage) = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 4) else {
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
        let (x, y) = self.respawn_position();
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
                self.refresh_crafting_order();
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
                if let Some(touches) = touches {
                    match touches {
                        1 => self.add_notification("minicraft.notifications.statue_tapped"),
                        2 => self.add_notification("minicraft.notifications.statue_touched"),
                        _ => self.notify("minicraft.notification.obsidian_knight_awoken", 0),
                    }
                }
            } else if kind == FurnitureKind::Bed {
                if !minicraft_client::core::sleep::can_sleep(
                    self.clock.get().day_tick,
                    self.days > 1,
                ) {
                    let seconds =
                        (40_500_u32.saturating_sub(self.clock.get().day_tick)).div_ceil(60);
                    self.add_notification_args(
                        "minicraft.notification.cannot_sleep",
                        &[&(seconds / 60).to_string(), &(seconds % 60).to_string()],
                    );
                } else {
                    self.enter_bed();
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
}

fn render_world_book(screen: &mut Screen, assets: &Assets, book: Book, page: usize) {
    let pages = book.pages();
    let page = page.min(pages.len().saturating_sub(1));
    screen.rect(18, 17, WIDTH as i32 - 36, HEIGHT as i32 - 34, 0xD8C68F);
    screen.rect(22, 21, WIDTH as i32 - 44, HEIGHT as i32 - 42, 0x33291C);
    screen.centered_text(assets.font(), book.title(), 27);
    for (row, line) in pages[page].iter().enumerate() {
        screen.text(assets.font(), line, 29, 42 + row as i32 * 8);
    }
    screen.centered_text(
        assets.font(),
        &format!("< PAGE {}/{} >  SELECT CLOSE", page + 1, pages.len()),
        HEIGHT as i32 - 25,
    );
}

fn render_sign_editor(screen: &mut Screen, assets: &Assets, text: &str) {
    screen.rect(36, 72, WIDTH as i32 - 72, 69, 0x101018);
    screen.centered_text(assets.font(), "EDIT SIGN", 81);
    let visible = text.chars().take(28).collect::<String>();
    screen.rect(47, 99, WIDTH as i32 - 94, 13, 0x493A28);
    screen.centered_text(assets.font(), &format!("{visible}_"), 102);
    screen.centered_text(assets.font(), "TYPE  BACKSPACE  SELECT SAVE", 124);
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
    try_queue_natural_spawn_for_player(
        level,
        width,
        height,
        Some((player_x, player_y)),
        day_tick,
        days,
        random,
    );
}

fn try_queue_natural_spawn_for_player(
    level: &mut Level,
    width: usize,
    height: usize,
    player: Option<(i32, i32)>,
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
    // Java Level.trySpawn performs the skip roll before iterating players.
    // An empty player set consumes no species or coordinate random draws.
    let Some((player_x, player_y)) = player else {
        return;
    };
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
    ticks: i32,
    score: u32,
    multiplier: u8,
) {
    let total_seconds = (f64::from(ticks) / 60.0).ceil() as i32;
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
    screen.text(assets.font(), &time, WIDTH as i32 / 2 - 72, 2);
    let score = format!("CURRENT SCORE: {score}");
    screen.text(
        assets.font(),
        &score,
        WIDTH as i32 - minicraft_client::gfx::font::text_width(&score) - 2,
        11,
    );
    if multiplier > 1 {
        let multiplier = format!("X{multiplier}");
        screen.text(
            assets.font(),
            &multiplier,
            WIDTH as i32 - minicraft_client::gfx::font::text_width(&multiplier) - 2,
            20,
        );
    }
}

fn render_quest_overlay(screen: &mut Screen, assets: &Assets, progress: &str) {
    let label = progress.chars().take(30).collect::<String>();
    let width =
        (((label.chars().count().max(6) as i32 * 8 + 32) / 8) * 8).clamp(72, WIDTH as i32 - 18);
    let x = WIDTH as i32 - 9 - width;
    render_menu_frame(screen, assets.hud(), x, 9, width, 24);
    screen.text(assets.font(), "QUESTS", x + (width - 48) / 2, 9);
    screen.text(assets.font(), &label, x + 16, 17);
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
            minicraft_client::gfx::font::text_width(&label) + 4,
            10,
            0x101018,
        );
        screen.text(assets.font(), &label, x, y);
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
    screen.blit_region_transformed(assets.hud(), 0, 32, 8, 8, x + 200, y, true, false);
    for column in 0..100 {
        screen.blit_region(assets.hud(), x + column * 2, y, 24, 32, 8, 8, false);
    }
    screen.blit_region(assets.hud(), x - 5, y, 0, 40, 8, 8, false);
    for column in 0..length {
        screen.blit_region(assets.hud(), x + column as i32 * 2, y, 24, 40, 8, 8, false);
    }
    screen.centered_text(assets.font(), title, y + 8);
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
            screen.blit_region(assets.hud(), x * 8, bottom - 8, 40, 16, 8, 8, false);
        }
    }

    if let Some(active) = player.active_item {
        let (image, label) = match active {
            ActiveItem::Stack(item) => (
                Some(assets.item(item)),
                format!(
                    " {} {}",
                    player.inventory.count(item).clamp(1, 999),
                    item.display_name()
                ),
            ),
            ActiveItem::Tool(index) => player
                .inventory
                .tools()
                .get(index)
                .map(|tool| (assets.tool(*tool), format!(" {}", tool.display_name())))
                .map_or((None, String::new()), |(image, label)| (Some(image), label)),
        };
        if let Some(image) = image {
            screen.blit(image, 80, bottom - 8);
            draw_hud_text_background(screen, assets, &label, 88, bottom - 8, 0xFFFFFF);
        }
    }

    if let Some(ActiveItem::Tool(index)) = player.active_item
        && let Some(tool) = player.inventory.tools().get(index)
    {
        let durability = u32::from(tool.durability) * 100 / u32::from(tool.max_durability.max(1));
        draw_hud_text_background(
            screen,
            assets,
            &format!("{durability}%"),
            164,
            bottom - 16,
            hud_percentage_color(durability),
        );
        if tool.kind == ToolKind::Bow {
            draw_hud_text_background(
                screen,
                assets,
                &format!(" x{}", player.inventory.count(ItemId::Arrow)),
                84,
                bottom - 16,
                0xFFFFFF,
            );
            screen.blit_region(assets.hud(), 84, bottom - 16, 32, 8, 8, 8, false);
        }
    }
    if player.active_item == Some(ActiveItem::Stack(ItemId::WateringCan)) {
        let content = u32::from(player.watering_content) * 100 / 1_800;
        draw_hud_text_background(
            screen,
            assets,
            &format!("{content}%"),
            164,
            bottom - 16,
            hud_percentage_color(content),
        );
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
        let width = minicraft_client::gfx::font::text_width(&effects) + 4;
        screen.rect(WIDTH as i32 - width - 2, 8, width, 10, 0x101018);
        screen.text(assets.font(), &effects, WIDTH as i32 - width, 9);
    }

    for index in 0..10 {
        let health_source_y = if player.health > 20 {
            24
        } else if player.health > 10 {
            16
        } else {
            0
        };
        screen.blit_region(assets.hud(), index * 8, bottom - 16, 0, 8, 8, 8, false);
        let tier_start = if player.health > 20 {
            20
        } else if player.health > 10 {
            10
        } else {
            0
        };
        if usize::from(player.health.saturating_sub(tier_start)) > index as usize {
            screen.blit_region(
                assets.hud(),
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
            assets.hud(),
            index * 8,
            bottom - 8,
            8,
            stamina_source_y,
            8,
            8,
            false,
        );
        screen.blit_region(
            assets.hud(),
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

fn draw_hud_text_background(
    screen: &mut Screen,
    assets: &Assets,
    text: &str,
    x: i32,
    y: i32,
    color: u32,
) {
    for (index, _) in text.chars().enumerate() {
        screen.blit_region(assets.hud(), x + index as i32 * 8, y, 40, 16, 8, 8, false);
    }
    screen.text_colored(assets.font(), text, x, y, color);
}

fn hud_percentage_color(percent: u32) -> u32 {
    let green = (percent.min(100) as f32 * 2.55) as u32;
    (255 - green) << 16 | green << 8
}

/// Renders only the live gameplay HUD for deterministic pixel comparison.
pub(crate) fn render_hud_parity(screen: &mut Screen, assets: &Assets, world: &World) {
    render_hud(
        screen,
        assets,
        &world.player,
        world.mode,
        world.seed,
        world.levels[world.current_level].depth,
        world.clock.get().day_tick,
        world.days,
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
    crafting_order: &[usize],
    personal_crafting: bool,
    mode: GameMode,
) {
    if personal_crafting || station.is_some() {
        render_crafting_menu(
            screen,
            assets,
            player,
            station,
            craft_selection,
            mode == GameMode::Creative,
            crafting_order,
        );
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

    let player_width = inventory_panel_width(&player_entries, "Inventory");
    if mode != GameMode::Creative || pane == 0 {
        render_inventory_panel(
            screen,
            assets,
            &player_entries,
            "Inventory",
            9,
            Some(item_selection),
        );
        render_inventory_counter(
            screen,
            assets.inventory_counter(),
            9 + player_width,
            9,
            player_entries.len(),
            player.inventory.capacity(),
        );
        if mode != GameMode::Creative
            && let Some(description) = inventory_description(player, item_selection)
        {
            render_inventory_description(screen, assets, &description, 10, 105);
            screen.text(assets.font(), "(SHIFT-F) to search.", 12, 137);
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
    let creative_width = inventory_panel_width(&creative_entries, "Items");
    let creative_x = WIDTH as i32 - 10 - creative_width;
    let player_x = creative_x - 10 - player_width;
    render_inventory_panel(screen, assets, &player_entries, "Inventory", player_x, None);
    render_inventory_panel(
        screen,
        assets,
        &creative_entries,
        "Items",
        creative_x,
        Some(craft_selection),
    );
    render_inventory_counter_minimized(
        screen,
        assets.inventory_counter(),
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

fn inventory_description(player: &Player, selection: usize) -> Option<String> {
    if let Some(stack) = player.inventory.slots().get(selection) {
        return Some(stack.item.display_name().to_owned());
    }
    player
        .inventory
        .tools()
        .get(selection.saturating_sub(player.inventory.slots().len()))
        .map(|tool| tool.display_name())
}

fn render_inventory_description(
    screen: &mut Screen,
    assets: &Assets,
    description: &str,
    x: i32,
    y: i32,
) {
    let width = ((minicraft_client::gfx::font::text_width(&description) + 16).max(32) + 7) / 8 * 8;
    render_menu_frame(screen, assets.hud(), x, y, width, 24);
    screen.text(assets.font(), description, x + 8, y + 8);
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
    let (y, height) = (9, 88);
    render_menu_frame(screen, assets.hud(), x, y, width, height);
    render_menu_title(screen, assets, title, x, y, width, 0xFFFF00);

    let selected = selection.unwrap_or(0);
    let first = selected
        .saturating_sub(4)
        .min(entries.len().saturating_sub(9));
    for (row, (image, label)) in entries.iter().skip(first).take(9).enumerate() {
        let index = first + row;
        let entry_x = x + 24;
        let entry_y = y + 8 + row as i32 * 8;
        if selection == Some(index) {
            screen.text(assets.font(), "> ", entry_x - 16, entry_y);
            screen.text(
                assets.font(),
                " <",
                entry_x + minicraft_client::gfx::font::text_width(&label),
                entry_y,
            );
        }
        screen.text(assets.font(), label, entry_x, entry_y);
        screen.blit(image, entry_x, entry_y);
    }
}

/// Renders the live inventory overlay without a world background for pixel parity.
pub(crate) fn render_inventory_parity(screen: &mut Screen, assets: &Assets, world: &World) {
    render_inventory(
        screen,
        assets,
        &world.player,
        world.inventory_item_selection,
        world.inventory_selection,
        world.inventory_pane,
        world.crafting_station,
        &world.crafting_order,
        world.personal_crafting,
        world.mode,
    );
}

fn render_crafting_menu(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    station: Option<FurnitureKind>,
    selection: usize,
    creative: bool,
    crafting_order: &[usize],
) {
    let recipes = ordered_recipe_views(station, &player.inventory, creative, crafting_order);
    let title = station.map_or("Crafting", FurnitureKind::display_name);
    let content_width = recipes
        .iter()
        .map(|recipe| recipe.label().chars().count())
        .chain([title.len()])
        .max()
        .unwrap_or(title.len()) as i32
        * 8;
    let width = (content_width + 48).clamp(96, 176);
    let (x, y, height) = (9, 9, 88);
    render_menu_frame(screen, assets.hud(), x, y, width, height);
    render_menu_title(screen, assets, title, x, y, width, 0xFFFF00);
    let first = selection
        .saturating_sub(4)
        .min(recipes.len().saturating_sub(9));
    for (row, recipe) in recipes.iter().skip(first).take(9).enumerate() {
        let index = first + row;
        let entry_x = x + 24;
        let entry_y = y + 8 + row as i32 * 8;
        let output = recipe.label();
        if index == selection {
            screen.text(assets.font(), "> ", entry_x - 16, entry_y);
            screen.text(
                assets.font(),
                " <",
                entry_x + minicraft_client::gfx::font::text_width(&output),
                entry_y,
            );
        }
        let craftable = creative
            || (recipe
                .costs
                .iter()
                .all(|cost| player.inventory.count(cost.item) >= cost.count)
                && recipe.tool_cost.is_none_or(|(kind, tier)| {
                    player
                        .inventory
                        .tools()
                        .iter()
                        .any(|tool| tool.kind == kind && tool.tier == tier)
                }));
        screen.text_colored(
            assets.font(),
            &output,
            entry_x,
            entry_y,
            if craftable { 0xFFFFFF } else { 0x999999 },
        );
        screen.blit(recipe.image(assets), entry_x, entry_y);
    }

    if let Some(recipe) = recipes.get(selection) {
        let costs = recipe.costs;
        let panel_x = x + width + 8;
        let available_width = ((WIDTH as i32 - panel_x - 9) / 8) * 8;
        let have_label = format!(" {}", recipe.output_count(player));
        let have_width =
            (("Have:".len().max(have_label.len()) as i32 * 8) + 16).min(available_width);
        if have_width >= 56 {
            render_menu_frame(screen, assets.hud(), panel_x, y, have_width, 24);
            render_menu_title_left(screen, assets, "Have:", panel_x, y, 0xFFFF00);
            screen.blit(recipe.image(assets), panel_x + 8, y + 8);
            screen.text(assets.font(), &have_label, panel_x + 8, y + 8);
            let cost_width = costs
                .iter()
                .map(|cost| format!(" {}/{}", player.inventory.count(cost.item), cost.count).len())
                .chain(["Cost:".len()])
                .max()
                .unwrap_or("Cost:".len()) as i32
                * 8
                + 16;
            let cost_width = cost_width.min(available_width);
            let cost_height = costs.len().max(1) as i32 * 8 + 16;
            let cost_y = y + height - cost_height;
            render_menu_frame(
                screen,
                assets.hud(),
                panel_x,
                cost_y,
                cost_width,
                cost_height,
            );
            render_menu_title_left(screen, assets, "Cost:", panel_x, cost_y, 0xFFFF00);
            for (row, cost) in costs.iter().enumerate() {
                let entry_y = cost_y + 8 + row as i32 * 8;
                screen.blit(assets.item(cost.item), panel_x + 8, entry_y);
                screen.text(
                    assets.font(),
                    &format!(" {}/{}", player.inventory.count(cost.item), cost.count),
                    panel_x + 8,
                    entry_y,
                );
            }
        }
    }
}

pub(crate) use minicraft_client::gfx::gui::render_menu_frame;

fn render_menu_title(
    screen: &mut Screen,
    assets: &Assets,
    title: &str,
    x: i32,
    y: i32,
    width: i32,
    color: u32,
) {
    let title_width = minicraft_client::gfx::font::text_width(&title);
    let title_x = x + (width - title_width) / 2;
    minicraft_client::gfx::gui::render_title(
        screen,
        assets.hud(),
        assets.font(),
        title,
        title_x,
        y,
        color,
    );
}

fn render_menu_title_left(
    screen: &mut Screen,
    assets: &Assets,
    title: &str,
    x: i32,
    y: i32,
    color: u32,
) {
    let title_x = x + 8;
    minicraft_client::gfx::gui::render_title(
        screen,
        assets.hud(),
        assets.font(),
        title,
        title_x,
        y,
        color,
    );
}

fn render_inventory_counter(
    screen: &mut Screen,
    counter: &crate::gfx::Image,
    right: i32,
    top: i32,
    size: usize,
    capacity: usize,
) {
    if size < 10 {
        screen.blit_region(counter, right - 16, top - 3, 12, 12, 3, 13, false);
        screen.blit_region(counter, right - 13, top - 3, 20, 12, 15, 13, false);
    } else {
        screen.blit_region(counter, right - 21, top - 3, 12, 12, 23, 13, false);
    }
    let current_color = inventory_counter_color(size, capacity, true);
    render_counter_digits(
        screen,
        counter,
        right - if size < 10 { 14 } else { 19 },
        top - 1,
        5,
        5,
        7,
        size,
        current_color,
    );
    render_counter_digits(
        screen,
        counter,
        right - 8,
        top + 3,
        0,
        4,
        5,
        capacity,
        0x999999,
    );
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
        render_counter_digits(
            screen,
            counter,
            right - 10,
            top + 1,
            0,
            4,
            5,
            size,
            inventory_counter_faded_color(size, 27),
        );
    } else {
        screen.blit_region(counter, right - 16, top - 1, 0, 12, 12, 9, false);
        render_counter_digits(
            screen,
            counter,
            right - 14,
            top + 1,
            0,
            4,
            5,
            size,
            inventory_counter_faded_color(size, 27),
        );
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
    color: u32,
) {
    for (index, digit) in number.to_string().bytes().enumerate() {
        screen.render_sprite(
            counter,
            x + index as i32 * width as i32,
            y,
            minicraft_client::gfx::sprite::SpriteRender {
                source_x: usize::from(digit - b'0') * width,
                source_y,
                width,
                height,
                white_tint: Some(color),
                ..Default::default()
            },
        );
    }
}

fn inventory_counter_color(size: usize, capacity: usize, main: bool) -> u32 {
    let capacity = capacity.max(1) as f32;
    let inner = (size as f32 / capacity).clamp(0.0, 1.0) - 1.0;
    let heaviness = (-(inner * inner) + 1.0).clamp(0.0, 1.0);
    if main {
        if heaviness < 0.5 {
            ((heaviness / 0.5 * 255.0) as u32) << 16 | 0x00FF00
        } else {
            (((1.0 - heaviness) / 0.5 * 255.0) as u32) << 8 | 0xFF0000
        }
    } else if heaviness < 0.3 {
        (heaviness / 0.3 * 255.0) as u32 | 0xFFFF00
    } else {
        (((1.0 - heaviness) / 0.7 * 255.0) as u32) << 8 | 0xFF0000
    }
}

fn inventory_counter_faded_color(size: usize, capacity: usize) -> u32 {
    let color = inventory_counter_color(size, capacity, false);
    let blend = |background: u32, foreground: u32| {
        (background as i32 + ((foreground as i32 - background as i32) as f32 * 0.35) as i32) as u32
    };
    let red = blend(0x2A, (color >> 16) & 0xFF);
    let green = blend(0x32, (color >> 8) & 0xFF);
    let blue = blend(0x99, color & 0xFF);
    red << 16 | green << 8 | blue
}

#[derive(Clone, Copy)]
enum RecipeOutput {
    Stack(ItemStack),
    Tool(ToolItem),
}

#[derive(Clone, Copy)]
struct RecipeView {
    output: RecipeOutput,
    costs: &'static [ItemStack],
    tool_cost: Option<(crate::item::ToolKind, crate::item::ToolTier)>,
}

impl RecipeView {
    fn product_name(&self) -> String {
        match self.output {
            RecipeOutput::Stack(stack) => stack.item.display_name().to_owned(),
            RecipeOutput::Tool(tool) => tool.display_name(),
        }
    }

    fn label(&self) -> String {
        match self.output {
            RecipeOutput::Stack(stack) => {
                let suffix = if stack.count > 1 {
                    format!(" x{}", stack.count)
                } else {
                    String::new()
                };
                format!(" {}{}", stack.item.display_name(), suffix)
            }
            RecipeOutput::Tool(tool) => format!(" {}", tool.display_name()),
        }
    }

    fn image<'a>(&self, assets: &'a Assets) -> &'a crate::gfx::Image {
        match self.output {
            RecipeOutput::Stack(stack) => assets.item(stack.item),
            RecipeOutput::Tool(tool) => assets.tool(tool),
        }
    }

    fn output_count(&self, player: &Player) -> usize {
        match self.output {
            RecipeOutput::Stack(stack) => usize::from(player.inventory.count(stack.item)),
            RecipeOutput::Tool(expected) => player
                .inventory
                .tools()
                .iter()
                .filter(|tool| tool.kind == expected.kind && tool.tier == expected.tier)
                .count(),
        }
    }
}

fn recipe_views(station: Option<FurnitureKind>) -> Vec<RecipeView> {
    let stack = |recipe: &crate::item::Recipe| RecipeView {
        output: RecipeOutput::Stack(recipe.output),
        costs: recipe.costs,
        tool_cost: None,
    };
    let tool = |recipe: &crate::item::ToolRecipe| RecipeView {
        output: RecipeOutput::Tool(recipe.output),
        costs: recipe.costs,
        tool_cost: recipe.tool_cost,
    };
    match station {
        None => HAND_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Workbench) => WORKBENCH_STATION_RECIPES[..28]
            .iter()
            .map(stack)
            .chain(WORKBENCH_TOOL_RECIPES.iter().map(tool))
            .chain(WORKBENCH_STATION_RECIPES[28..].iter().map(stack))
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

fn sorted_recipe_views(
    station: Option<FurnitureKind>,
    inventory: &crate::item::Inventory,
    creative: bool,
) -> Vec<RecipeView> {
    let mut recipes = recipe_views(station);
    recipes.sort_by(|left, right| {
        recipe_can_craft(right, inventory, creative)
            .cmp(&recipe_can_craft(left, inventory, creative))
    });
    recipes
}

fn ordered_recipe_views(
    station: Option<FurnitureKind>,
    inventory: &crate::item::Inventory,
    creative: bool,
    order: &[usize],
) -> Vec<RecipeView> {
    let recipes = recipe_views(station);
    if order.len() == recipes.len() && order.iter().all(|&index| index < recipes.len()) {
        order.iter().map(|&index| recipes[index]).collect()
    } else {
        sorted_recipe_views(station, inventory, creative)
    }
}

fn recipe_can_craft(
    recipe: &RecipeView,
    inventory: &crate::item::Inventory,
    creative: bool,
) -> bool {
    creative
        || (recipe
            .costs
            .iter()
            .all(|cost| inventory.count(cost.item) >= cost.count)
            && recipe.tool_cost.is_none_or(|(kind, tier)| {
                inventory
                    .tools()
                    .iter()
                    .any(|tool| tool.kind == kind && tool.tier == tier)
            }))
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
                    assets.font(),
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
                screen.blit(assets.smash(), entity.x - camera_x, entity.y - camera_y);
            }
            entity::ParticleKind::Fire => {
                screen.rect(entity.x - camera_x, entity.y - camera_y, 2, 2, 0xFF8A30);
            }
        },
        EntityKind::TextParticle(text) => {
            let (offset_x, offset_y, z) = text_particle_motion(entity.id, entity.age);
            let x = entity.x - camera_x - text.chars().count() as i32 * 4 + offset_x;
            let y = entity.y - camera_y + offset_y - z;
            screen.text_colored(assets.font(), text, x + 1, y + 1, 0x000000);
            screen.text_colored(assets.font(), text, x, y, 0xFF3030);
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

pub(crate) fn render_pause(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
) {
    if matches!(
        world.pause.render_view().page,
        PausePage::Quests
            | PausePage::QuestSeriesInfo
            | PausePage::QuestSeriesViewer
            | PausePage::QuestDetail
    ) {
        render_quest_display(screen, assets, localization, world);
        return;
    }
    // Java Menu.Builder derives the frame from nine 8px entries with 4px
    // spacing. The non-selectable help line is the widest entry, but the
    // selectable cursor inset removes 32px from its width contribution.
    let entry_count = if world.quests_enabled { 10 } else { 9 };
    let width = 184;
    let height = entry_count * 12 + 12;
    let x = (WIDTH as i32 - width) / 2;
    let y = (HEIGHT as i32 - height) / 2;
    render_menu_frame(screen, assets.hud(), x, y, width, height);
    let (title, labels): (&str, Vec<String>) = match world.pause.render_view().page {
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
        PausePage::Quests
        | PausePage::QuestSeriesInfo
        | PausePage::QuestSeriesViewer
        | PausePage::QuestDetail => unreachable!(),
    };
    render_menu_title(screen, assets, title, x, y, width, 0xFFFF00);
    for (index, label) in labels.iter().enumerate() {
        // The first Java entry is BlankEntry; selectable entries begin on row 1.
        let draw_y = y + 20 + index as i32 * 12;
        let draw_x = x + (width - minicraft_client::gfx::font::text_width(&label)) / 2;
        let selected = if world.pause.render_view().confirm
            && world.pause.render_view().page == PausePage::Main
        {
            labels.len().saturating_sub(1)
        } else {
            world.pause.render_view().selection
        };
        if index == selected
            && matches!(
                world.pause.render_view().page,
                PausePage::Main | PausePage::Options
            )
        {
            screen.text(assets.font(), "> ", draw_x - 16, draw_y);
            screen.text(
                assets.font(),
                " <",
                draw_x + minicraft_client::gfx::font::text_width(&label),
                draw_y,
            );
        }
        let selectable = matches!(
            world.pause.render_view().page,
            PausePage::Main | PausePage::Options
        );
        screen.text_colored(
            assets.font(),
            label,
            draw_x,
            draw_y,
            if selectable && index != selected {
                0x999999
            } else {
                0xFFFFFF
            },
        );
    }
    if world.pause.render_view().page == PausePage::Main {
        let help_y = y + 20 + labels.len() as i32 * 12 + 12;
        screen.text_colored(
            assets.font(),
            "UP and DOWN to Scroll",
            (WIDTH as i32 - 21 * 8) / 2,
            help_y,
            0x999999,
        );
        screen.text_colored(
            assets.font(),
            "ENTER: Choose",
            (WIDTH as i32 - 13 * 8) / 2,
            help_y + 12,
            0x999999,
        );
    }
    if world.pause.render_view().confirm {
        // PopupDisplay ignores PopupConfig.entrySpacing and uses Menu's default
        // zero spacing. Six entries therefore produce this centered 280x64 box.
        let (popup_x, popup_y, popup_width, popup_height) = (4, 64, 280, 64);
        render_menu_frame(
            screen,
            assets.hud(),
            popup_x,
            popup_y,
            popup_width,
            popup_height,
        );
        screen.centered_text(
            assets.font(),
            "Are you sure you want to exit the",
            popup_y + 8,
        );
        screen.centered_text(assets.font(), "game?", popup_y + 16);
        let warning = "All unsaved progress will be lost";
        screen.text_colored(
            assets.font(),
            warning,
            (WIDTH as i32 - minicraft_client::gfx::font::text_width(&warning)) / 2,
            popup_y + 24,
            0xC62C2C,
        );
        for (index, label) in ["Cancel", "Quit without saving"].iter().enumerate() {
            let draw_y = popup_y + 40 + index as i32 * 8;
            let draw_x = (WIDTH as i32 - label.len() as i32 * 8) / 2;
            if index == world.pause.render_view().selection {
                screen.text(assets.font(), "> ", draw_x - 16, draw_y);
                screen.text(assets.font(), " <", draw_x + label.len() as i32 * 8, draw_y);
            }
            screen.text_colored(
                assets.font(),
                label,
                draw_x,
                draw_y,
                if index == world.pause.render_view().selection {
                    0xFFFFFF
                } else {
                    0x999999
                },
            );
        }
    }
}

fn quest_description_lines(text: &str) -> Vec<String> {
    // Java StringEntry.useLines uses pixel width and keeps the final remainder.
    minicraft_client::gfx::font::get_lines(text, WIDTH as i32 - 20, HEIGHT as i32 * 2, 0, false)
}

fn render_quest_root(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
    selection: usize,
) {
    // QuestsDisplay(clearScreen=true), including when drawn as a parent.
    screen.clear(0);
    screen.centered_text(
        assets.font(),
        &format!(
            "{}: {}",
            localization.text("minicraft.settings.quests"),
            if world.quests_enabled { "On" } else { "Off" }
        ),
        6,
    );
    let unlocked = localization.text("minicraft.displays.quests.display.header.unlocked");
    let completed = localization.text("minicraft.displays.quests.display.header.completed");
    screen.text_colored(
        assets.font(),
        unlocked,
        WIDTH as i32 / 2 - minicraft_client::gfx::font::text_width(&unlocked) - 16,
        26,
        if world.pause.render_view().quest_tab == 0 {
            0xFFFFFF
        } else {
            0x999999
        },
    );
    screen.text_colored(
        assets.font(),
        completed,
        WIDTH as i32 / 2 + 16,
        26,
        if world.pause.render_view().quest_tab == 1 {
            0xFFFFFF
        } else {
            0x999999
        },
    );
    let groups = world.progress.quest_group_views(
        &world.inventory_progress(),
        world.pause.render_view().quest_tab == 1,
    );
    let first = selection
        .saturating_sub(2)
        .min(groups.len().saturating_sub(5));
    // The menu is built empty with displayLength=5. setEntries does not
    // recalculate its fixed 44px-high entry bounds, centered at (144,76).
    for (row, group) in groups.iter().skip(first).take(5).enumerate() {
        let label = localization.text(&group.id);
        let width = minicraft_client::gfx::font::text_width(&label);
        let x = (WIDTH as i32 - width) / 2;
        let y = 54 + row as i32 * 9;
        if first + row == selection {
            screen.text(assets.font(), "> ", x - 16, y);
            screen.text(assets.font(), " <", x + width, y);
        }
        screen.text_colored(
            assets.font(),
            label,
            x,
            y,
            if group.completed {
                quest_tree::GREEN
            } else {
                0xFFFFFF
            },
        );
    }
    let description = groups
        .get(selection)
        .map(|group| localization.text(&group.description))
        .unwrap_or_else(|| localization.text("minicraft.displays.quests.display.no_quest_desc"));
    let lines = quest_description_lines(description);
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    minicraft_client::gfx::font_style::FontStyle {
        color: 0xFFFFFF,
        shadow_color: 0,
        shadow_mask: 0,
    }
    .draw_paragraph(
        screen,
        assets.font(),
        &lines,
        &minicraft_client::gfx::font_style::ParagraphLayout {
            anchor: (WIDTH as i32 / 2, 127),
            block_position: minicraft_client::screen::rel_pos::RelPos::BOTTOM,
            line_position: minicraft_client::screen::rel_pos::RelPos::TOP,
            spacing: 0,
        },
    );
}

/// Menu.Builder(true, spacing, CENTER) anchored at (Screen.w/2, top), BOTTOM.
/// Titles here are StringEntry rows inside separate panels, not yellow frame titles.
fn render_quest_info_panel(
    screen: &mut Screen,
    assets: &Assets,
    top: i32,
    spacing: i32,
    entries: &[(String, u32, bool)],
) {
    let selectable = entries.iter().any(|entry| entry.2);
    let cursor_inset = if selectable { 32 } else { 0 };
    let content_width = entries
        .iter()
        .map(|(text, _, select)| {
            (minicraft_client::gfx::font::text_width(&text)
                - if selectable && !select {
                    cursor_inset
                } else {
                    0
                })
            .max(0)
        })
        .max()
        .unwrap_or(0);
    let width = content_width + cursor_inset + 16;
    let step = 8 + spacing;
    let visible = ((HEIGHT as i32 - top - 16 + spacing) / step).max(0) as usize;
    let count = entries.len().min(visible);
    let height = count as i32 * step - spacing + 16;
    render_menu_frame(
        screen,
        assets.hud(),
        (WIDTH as i32 - width) / 2,
        top,
        width,
        height,
    );
    for (row, (text, color, select)) in entries.iter().take(count).enumerate() {
        let x = (WIDTH as i32 - minicraft_client::gfx::font::text_width(&text)) / 2;
        let y = top + 8 + row as i32 * step;
        screen.text_colored(assets.font(), text, x, y, *color);
        if *select {
            screen.text(assets.font(), "> ", x - 16, y);
            screen.text(
                assets.font(),
                " <",
                x + minicraft_client::gfx::font::text_width(&text),
                y,
            );
        }
    }
}

fn render_quest_display(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
) {
    render_quest_page(
        screen,
        assets,
        localization,
        world,
        world.pause.render_view().page,
    );
}

fn render_quest_page(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
    page: PausePage,
) {
    match page {
        PausePage::Quests => {
            render_quest_root(
                screen,
                assets,
                localization,
                world,
                world.pause.render_view().selection,
            );
        }
        PausePage::QuestSeriesInfo => {
            render_quest_root(
                screen,
                assets,
                localization,
                world,
                world.pause.render_view().quest_series_selection,
            );
            let Some(group) = world
                .progress
                .quest_group_views(&world.inventory_progress(), false)
                .into_iter()
                .find(|group| group.index == world.pause.render_view().quest_group)
            else {
                return;
            };
            render_quest_info_panel(
                screen,
                assets,
                5,
                0,
                &[(localization.text(&group.id).to_owned(), 0xFFFFFF, false)],
            );
            let status_key = if group.completed {
                "minicraft.displays.quests.quest_info.display.status.completed"
            } else {
                "minicraft.displays.quests.quest_info.display.status.unlocked"
            };
            let mut entries = vec![
                (
                    localization.format(
                        "minicraft.displays.quests.quest_info.display.status",
                        &[localization.text(status_key)],
                    ),
                    if group.completed {
                        quest_tree::GREEN
                    } else {
                        0xFFFFFF
                    },
                    false,
                ),
                (
                    localization.format(
                        "minicraft.displays.quests.quest_info.display.quests_completed_count",
                        &[&group.completed_quests.to_string()],
                    ),
                    0xFFFFFF,
                    false,
                ),
            ];
            let description = localization.format(
                "minicraft.displays.quests.quest_info.display.description",
                &[localization.text(&group.description)],
            );
            entries.extend(
                quest_description_lines(&description)
                    .into_iter()
                    .map(|line| (line, 0xFFFFFF, false)),
            );
            entries.push((
                localization.format(
                    "minicraft.displays.quests.quest_info.display.ongoing_quests",
                    &[&group.ongoing_quests.to_string()],
                ),
                0xFFFFFF,
                false,
            ));
            entries.push((String::new(), 0xFFFFFF, false));
            entries.push((
                localization
                    .text("minicraft.displays.quests.quest_info.view_quests")
                    .to_owned(),
                0xFFFFFF,
                true,
            ));
            render_quest_info_panel(screen, assets, 30, 2, &entries);
        }
        PausePage::QuestSeriesViewer | PausePage::QuestDetail => {
            let quests = world.progress.quest_views(
                world.pause.render_view().quest_group,
                &world.inventory_progress(),
            );
            let Some(group) = world
                .progress
                .quest_group_views(&world.inventory_progress(), false)
                .into_iter()
                .find(|group| group.index == world.pause.render_view().quest_group)
            else {
                return;
            };
            if page == PausePage::QuestSeriesViewer {
                render_quest_page(
                    screen,
                    assets,
                    localization,
                    world,
                    PausePage::QuestSeriesInfo,
                );
                render_quest_info_panel(
                    screen,
                    assets,
                    6,
                    0,
                    &[
                        (
                            localization.text("minicraft.displays.quests").to_owned(),
                            0xFFFFFF,
                            false,
                        ),
                        (localization.text(&group.id).to_owned(), 0xFFFFFF, false),
                    ],
                );
                render_menu_frame(
                    screen,
                    assets.hud(),
                    8,
                    40,
                    WIDTH as i32 - 16,
                    HEIGHT as i32 - 60,
                );
                let tree = quest_tree::Tree::new(&quests, |id| localization.text(id).to_owned());
                tree.render(
                    screen,
                    assets.font(),
                    &quests,
                    world.pause.render_view().selection,
                    &world.quest_tree_state,
                    localization.text("minicraft.displays.quests.display.no_quest"),
                );
            } else {
                render_quest_page(
                    screen,
                    assets,
                    localization,
                    world,
                    PausePage::QuestSeriesViewer,
                );
                if let Some(quest) = quests.get(world.pause.render_view().quest_item) {
                    let status_key = if quest.completed {
                        "minicraft.displays.quests.quest_info.display.status.completed"
                    } else {
                        "minicraft.displays.quests.quest_info.display.status.unlocked"
                    };
                    let progress = if quest.has_requirements {
                        localization
                            .text(
                                "minicraft.displays.quests.quest_info.display.progress_uncompleted",
                            )
                            .to_owned()
                    } else {
                        localization.format(
                            "minicraft.displays.quests.quest_info.display.progress",
                            &[
                                &quest.criteria_completed.to_string(),
                                &quest.criteria_total.to_string(),
                            ],
                        )
                    };
                    render_quest_info_panel(
                        screen,
                        assets,
                        5,
                        1,
                        &[
                            (localization.text(&group.id).to_owned(), 0xFFFFFF, false),
                            (
                                format!(
                                    "{}: {}",
                                    localization.text(&quest.id),
                                    localization.text(status_key)
                                ),
                                if quest.completed {
                                    quest_tree::GREEN
                                } else {
                                    0xFFFFFF
                                },
                                false,
                            ),
                            (progress, 0xFFFFFF, false),
                        ],
                    );
                    let entries: Vec<_> =
                        quest_description_lines(localization.text(&quest.description))
                            .into_iter()
                            .map(|line| (line, 0xFFFFFF, false))
                            .collect();
                    render_quest_info_panel(screen, assets, 52, 2, &entries);
                }
            }
        }
        PausePage::Main | PausePage::Options | PausePage::Achievements => unreachable!(),
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
pub(crate) mod registry_tests {
    use super::{
        ActiveItem, DAY_LENGTH, Direction, FurnitureKind, GameMode, HUNGER_STAMINA_STEPS, Level,
        MAX_HUNGER_TICKS, PausePage, PauseSession, Player, RecipeOutput, RecipeView,
        STARVATION_HEALTH_FLOORS, TILE_SIZE, Tile, World, creative_entries,
        entity::{EntityArena, EntityKind},
        inventory_counter_color, pause_actions, player_sprite_frame,
        random::JavaRandom,
        recipe_can_craft, recipe_views, sorted_recipe_views, spawn, surface_darkness, time_name,
        try_queue_natural_spawn,
    };
    use crate::{
        audio::SoundEffect,
        input::Input,
        item::{ArmorKind, Inventory, ItemId, PotionKind, ToolItem, ToolKind, ToolTier},
    };
    use minicraft_client::core::updater::Updater;

    pub(crate) fn tiny_world() -> World {
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
            clock: Default::default(),
            render_gui: true,
            updater: Default::default(),
            days: 1,
            difficulty: 1,
            mode: super::GameMode::Survival,
            score: 0,
            score_ticks: 20 * 60 * 60,
            score_multiplier: 1,
            multiplier_ticks: 300,
            game_over: false,
            end_game: None,
            story_complete: false,
            sleeping: 0,
            bed_level: None,
            respawn_tile: None,
            sleep_speed: 1,
            tutorials_enabled: true,
            quests_enabled: true,
            show_quests: true,
            progress: crate::content::ProgressState::load().unwrap(),
            signs: vec![std::collections::HashMap::new()],
            sign_editor: None,
            book_open: None,
            pause: PauseSession::default(),
            inventory_open: false,
            quest_tree_state: super::quest_tree::State::default(),
            inventory_selection: 0,
            inventory_item_selection: 0,
            inventory_pane: 0,
            crafting_station: None,
            crafting_order: Vec::new(),
            personal_crafting: false,
            notifications: super::Notifications::default(),
            notification_context: super::notifications::Context::default(),
            air_wizard_defeated: false,
            obsidian_knight_defeated: false,
            random: JavaRandom::new(1),
            sound_events: Vec::new(),
        }
    }

    #[test]
    fn inventory_counter_uses_java_heaviness_colors() {
        assert_eq!(inventory_counter_color(0, 27, true), 0x00FF00);
        assert_eq!(inventory_counter_color(27, 27, true), 0xFF0000);
    }

    #[test]
    fn cactus_bump_hurts_and_blocks_the_player() {
        let mut world = tiny_world();
        let start_x = world.player.x;
        world.levels[0].tiles[9 + 8 * 16] = Tile::Cactus;

        assert!(!world.move_player(5, 0));
        assert_eq!(world.player.x, start_x);
        assert_eq!(world.player.health, 9);
        assert_eq!(world.player.hurt_time, 30);

        assert!(!world.move_player(5, 0));
        assert_eq!(world.player.health, 9);
    }

    #[test]
    fn hard_cactus_and_lava_brick_use_java_bump_damage() {
        let mut hard = tiny_world();
        hard.difficulty = 2;
        hard.levels[0].tiles[9 + 8 * 16] = Tile::Cactus;
        assert!(!hard.move_player(5, 0));
        assert_eq!(hard.player.health, 8);

        let mut lava_brick = tiny_world();
        lava_brick.levels[0].tiles[9 + 8 * 16] = Tile::LavaBrick;
        assert!(lava_brick.move_player(5, 0));
        assert_eq!(lava_brick.player.health, 7);
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
    fn player_motion_matches_java_direction_collision_and_exhaustion_rules() {
        let mut opposing = tiny_world();
        let start_x = opposing.player.x;
        opposing.tick(&Input {
            left: true,
            right: true,
            ..Input::default()
        });
        assert_eq!(opposing.player.x, start_x);
        assert!(matches!(opposing.player.direction, Direction::Right));
        assert_eq!(opposing.player.walk_distance, 0);

        let mut blocked = tiny_world();
        blocked.player.x = 9 * TILE_SIZE - 5;
        blocked.levels[0].tiles[9 + 8 * 16] = Tile::Rock;
        let blocked_x = blocked.player.x;
        blocked.tick(&Input {
            right: true,
            ..Input::default()
        });
        assert_eq!(blocked.player.x, blocked_x);
        assert_eq!(blocked.player.walk_distance, 1);
        assert_eq!(blocked.player.step_count, 0);

        let mut exhausted = tiny_world();
        exhausted.player.stamina = 0;
        let exhausted_x = exhausted.player.x;
        exhausted.tick(&Input {
            right: true,
            ..Input::default()
        });
        assert_eq!(exhausted.player.stamina_recharge_delay, 39);
        assert_eq!(exhausted.player.x, exhausted_x);
        exhausted.tick(&Input {
            right: true,
            ..Input::default()
        });
        assert_eq!(exhausted.player.stamina_recharge_delay, 38);
        assert_eq!(exhausted.player.x, exhausted_x + 1);
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
        world.inventory_selection = sorted_recipe_views(
            None,
            &world.player.inventory,
            false,
        )
        .iter()
        .position(|recipe| {
            matches!(recipe.output, RecipeOutput::Stack(stack) if stack.item == ItemId::Workbench)
        })
        .unwrap();
        let craft = Input {
            select: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sounds(&craft),
            vec![SoundEffect::Craft]
        );
        world.tick(&craft);
        assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
        assert!(world.take_sound_events().is_empty());
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
        world.inventory_selection = sorted_recipe_views(
            None,
            &world.player.inventory,
            false,
        )
        .iter()
        .position(|recipe| {
            matches!(recipe.output, RecipeOutput::Stack(stack) if stack.item == ItemId::Workbench)
        })
        .unwrap();
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.count(ItemId::Wood), 0);
        assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
        assert!(
            world
                .notifications
                .messages()
                .iter()
                .all(|message| !message.starts_with("CRAFTED"))
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
        world.notifications.clear();
        world.tick(&Input {
            pickup: true,
            ..Input::default()
        });
        assert_eq!(world.player.attack_time, 5);
        assert!(world.notifications.messages().is_empty());

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
        assert!(world.notifications.messages().is_empty());
        assert!(world.take_sound_events().contains(&SoundEffect::Pickup));
    }

    #[test]
    fn pause_audio_is_available_to_the_input_frame_without_a_tick_delay() {
        let mut world = tiny_world();
        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        let down = Input {
            down_pressed: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sounds(&down),
            vec![SoundEffect::Select]
        );
        world.tick(&down);
        let confirm = Input {
            select: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sounds(&confirm),
            vec![SoundEffect::Confirm]
        );
        world.tick(&confirm);
        assert!(world.take_sound_events().is_empty());
    }

    #[test]
    fn closing_pause_uses_the_java_display_exit_sound() {
        let mut world = tiny_world();
        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        assert_eq!(
            world.immediate_menu_sounds(&Input {
                exit: true,
                ..Input::default()
            }),
            vec![SoundEffect::Craft]
        );
    }

    #[test]
    fn canceling_pause_quit_plays_confirm_then_display_exit() {
        let mut world = tiny_world();
        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        world.pause.view_mut().selection = pause_actions(world.quests_enabled).len() - 1;
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert_eq!(
            world.immediate_menu_sounds(&Input {
                select: true,
                ..Input::default()
            }),
            vec![SoundEffect::Confirm, SoundEffect::Craft]
        );
    }

    #[test]
    fn pause_achievements_use_null_entries_and_java_on_exit_sound_order() {
        let mut world = tiny_world();
        world.prepare_pause_audio_parity();
        world.pause.view_mut().selection = 2;
        world.pause.push(PausePage::Achievements, 0);
        world.pause.begin_tick();
        assert!(
            world
                .immediate_menu_sounds(&Input {
                    select: true,
                    ..Input::default()
                })
                .is_empty()
        );
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert_eq!(world.pause.view().page, PausePage::Achievements);
        assert_eq!(
            world.immediate_menu_sounds(&Input {
                exit: true,
                ..Input::default()
            }),
            vec![SoundEffect::Craft, SoundEffect::Confirm]
        );
    }

    #[test]
    fn quest_root_tabs_and_nested_displays_follow_java_stack() {
        let mut world = tiny_world();
        world.prepare_quests_audio_parity();
        assert_eq!(world.audio_menu_state_for_parity(), ("quests", 0));

        let select = Input {
            select: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sounds(&select),
            vec![SoundEffect::Confirm]
        );
        world.tick(&select);
        assert_eq!(
            world.audio_menu_state_for_parity(),
            ("quest_series_info", 0)
        );
        world.tick(&select);
        assert_eq!(
            world.audio_menu_state_for_parity(),
            ("quest_series_viewer", 0)
        );
        world.tick(&select);
        assert_eq!(world.audio_menu_state_for_parity(), ("quest_detail", 0));

        let exit = Input {
            exit: true,
            ..Input::default()
        };
        for expected in ["quest_series_viewer", "quest_series_info", "quests"] {
            assert_eq!(world.immediate_menu_sounds(&exit), vec![SoundEffect::Craft]);
            world.tick(&exit);
            assert_eq!(world.audio_menu_state_for_parity().0, expected);
        }

        let right = Input {
            right_pressed: true,
            ..Input::default()
        };
        assert!(world.immediate_menu_sounds(&right).is_empty());
        world.tick(&right);
        assert_eq!(world.audio_menu_state_for_parity(), ("quests", 1));
        assert!(world.immediate_menu_sounds(&select).is_empty());
    }

    #[test]
    fn quest_tree_navigation_and_detail_return_preserve_cursor_and_viewport() {
        let mut world = tiny_world();
        world.prepare_quests_audio_parity();
        world.complete_hoe_quest_for_parity();
        let select = Input {
            select: true,
            ..Input::default()
        };
        world.tick(&select);
        world.prepare_quest_layout(&crate::localization::Localization::load("en-us", &[]));
        world.tick(&select);
        let right = Input {
            right_pressed: true,
            ..Input::default()
        };
        assert!(world.immediate_menu_sounds(&right).is_empty());
        world.tick(&right);
        assert_eq!(world.pause.view().selection, 0); // Children are below, not to the right.
        let down = Input {
            down_pressed: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sounds(&down),
            vec![SoundEffect::Select]
        );
        world.tick(&down);
        assert_eq!(world.pause.view().selection, 1);
        assert!(world.immediate_menu_sounds(&down).is_empty());

        let browsing = Input {
            shift: true,
            down_pressed: true,
            select: true,
            ..Input::default()
        };
        assert!(world.immediate_menu_sounds(&browsing).is_empty());
        world.tick(&browsing);
        assert_eq!(world.pause.view().page, PausePage::QuestSeriesViewer);
        assert_eq!(world.quest_tree_state.y_scroll, 3);
        assert_eq!(world.pause.view().selection, 1);
        world.tick(&select); // Release Shift; current node is already visible.
        assert_eq!(world.pause.view().page, PausePage::QuestDetail);
        assert_eq!(world.pause.view().quest_item, 1);
        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        assert_eq!(world.pause.view().page, PausePage::QuestSeriesViewer);
        assert_eq!(
            (
                world.pause.view().selection,
                world.quest_tree_state.y_scroll
            ),
            (1, 3)
        );
        assert!(world.take_sound_events().is_empty());

        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        world.tick(&select); // A newly constructed viewer resets its scroll/cursor.
        assert_eq!(
            (
                world.pause.view().selection,
                world.quest_tree_state.y_scroll
            ),
            (0, 0)
        );
    }

    #[test]
    fn inventory_equip_and_exit_use_the_java_display_exit_sound_immediately() {
        let mut world = tiny_world();
        world.prepare_inventory_audio_parity();
        let select = Input {
            select: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sounds(&select),
            vec![SoundEffect::Craft]
        );
        world.tick(&select);
        assert!(world.take_sound_events().is_empty());
        assert_eq!(world.audio_menu_state_for_parity().0, "gameplay");

        world.prepare_inventory_audio_parity();
        assert_eq!(
            world.immediate_menu_sounds(&Input {
                exit: true,
                ..Input::default()
            }),
            vec![SoundEffect::Craft]
        );
        assert_eq!(
            world.immediate_menu_sounds(&Input {
                menu: true,
                ..Input::default()
            }),
            vec![SoundEffect::Craft]
        );
    }

    #[test]
    fn successful_crafting_sound_is_not_deferred_or_duplicated() {
        let mut world = tiny_world();
        world.prepare_crafting_parity(false, 10, 0, 0, 0);
        let select = Input {
            select: true,
            ..Input::default()
        };
        assert_eq!(
            world.immediate_menu_sounds(&select),
            vec![SoundEffect::Craft]
        );
        world.tick(&select);
        assert!(world.take_sound_events().is_empty());
        assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);

        assert_eq!(
            world.immediate_menu_sounds(&Input {
                craft: true,
                ..Input::default()
            }),
            vec![SoundEffect::Craft]
        );
    }

    #[test]
    fn crafting_list_stably_partitions_craftable_recipes_like_java() {
        let mut inventory = Inventory::new(27);
        inventory.add(ItemId::Wood, 10);
        let recipes = sorted_recipe_views(None, &inventory, false);
        let craftable: Vec<bool> = recipes
            .iter()
            .map(|recipe| recipe_can_craft(recipe, &inventory, false))
            .collect();
        assert!(craftable.windows(2).all(|pair| pair[0] || !pair[1]));

        let original = recipe_views(None);
        let mut expected = Vec::new();
        for wanted in [true, false] {
            expected.extend(
                original
                    .iter()
                    .filter(|recipe| recipe_can_craft(recipe, &inventory, false) == wanted)
                    .map(RecipeView::product_name),
            );
        }
        assert_eq!(
            recipes
                .iter()
                .map(RecipeView::product_name)
                .collect::<Vec<_>>(),
            expected
        );
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
        assert!(world.pause.is_open());
        world.pause.view_mut().selection = 4;
        assert!(matches!(
            world.tick(&Input {
                select: true,
                ..Input::default()
            }),
            super::WorldAction::SaveGame
        ));
        assert!(!world.pause.is_open());

        world.tick(&Input {
            exit: true,
            ..Input::default()
        });
        world.pause.view_mut().selection = 5;
        world.tick(&Input {
            select: true,
            ..Input::default()
        });
        assert!(world.pause.view().confirm);
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
    fn pause_parent_selection_and_world_clock_follow_update_boundaries() {
        let mut world = tiny_world();
        let exit = Input {
            exit: true,
            ..Default::default()
        };
        let select = Input {
            select: true,
            ..Default::default()
        };
        world.tick(&exit);
        assert!(!world.pause.is_rendering());
        world.tick(&Input {
            down_pressed: true,
            ..Default::default()
        });
        assert_eq!(world.pause.render_view().selection, 1);
        world.tick(&select);
        assert_eq!(world.pause.view().page, PausePage::Options);
        assert_eq!(world.pause.render_view().page, PausePage::Main);
        world.tick(&exit);
        assert_eq!(world.pause.view().selection, 1);
        assert_eq!(world.pause.render_view().page, PausePage::Options);
        world.tick(&Input::default());
        assert_eq!(world.tick, 0);
        assert_eq!(world.pause.render_view().selection, 1);
        world.tick(&exit);
        assert!(!world.pause.is_open());
        assert!(world.pause.is_rendering());
        world.tick(&Input::default());
        assert_eq!(world.tick, 1);
        assert!(!world.pause.is_rendering());
        assert!(world.take_sound_events().is_empty());
    }

    #[test]
    fn focus_loss_advances_calendar_but_not_player_and_reconciles_pause() {
        let mut world = tiny_world();
        world.tick(&Input::default());
        assert_eq!((world.tick, world.clock.get().day_tick), (1, 0));
        let position = (world.player.x, world.player.y, world.player.stamina);
        for _ in 0..10 {
            world.begin_display_tick();
            world.tick_after_display(
                false,
                &Input {
                    exit: true,
                    attack: true,
                    right: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!((world.tick, world.clock.get().day_tick), (1, 10));
        assert_eq!(
            (world.player.x, world.player.y, world.player.stamina),
            position
        );
        assert!(!world.pause.is_open());
        world.tick(&Input {
            exit: true,
            ..Default::default()
        });
        assert!(!world.pause.is_rendering());
        world.begin_display_tick();
        world.tick_after_display(false, &Input::default());
        assert!(world.pause.is_rendering());
        assert_eq!(world.clock.get().day_tick, 12); // Paused is not overwritten without focus.
        world.tick(&Input::default());
        assert_eq!(world.clock.get().day_tick, 13);
        for _ in 0..10 {
            world.begin_display_tick();
            world.tick_after_display(false, &Input::default());
        }
        assert_eq!(world.clock.get().day_tick, 13);
        world.tick(&Input {
            exit: true,
            ..Default::default()
        });
        world.tick(&Input::default());
        assert_eq!(world.clock.get().day_tick, 13); // Resume uses previous paused=true.
        world.tick(&Input::default());
        assert_eq!(world.clock.get().day_tick, 14);
        assert!(world.take_sound_events().is_empty());
    }

    #[test]
    fn game_time_advances_in_inventory_but_not_without_focus() {
        let mut world = tiny_world();
        world.inventory_open = true;
        for _ in 0..5 {
            world.tick(&Input::default());
        }
        assert_eq!(
            (
                world.clock.get().game_time,
                world.tick,
                world.clock.get().day_tick
            ),
            (5, 0, 0)
        );
        world.tick_after_display(false, &Input::default());
        assert_eq!(world.clock.get().game_time, 5);
        let restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        assert_eq!((restored.clock.get().game_time, restored.tick), (5, 0));
    }

    #[test]
    fn missing_legacy_game_time_starts_a_new_counter() {
        let mut world = tiny_world();
        world.tick = 123;
        let mut saved: serde_json::Value =
            serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
        saved["world"].as_object_mut().unwrap().remove("game_time");
        let restored = World::from_save_string(&saved.to_string()).unwrap();
        assert_eq!((restored.clock.get().game_time, restored.tick), (0, 123));
    }

    #[test]
    fn focused_and_unfocused_frames_produce_identical_live_worlds_at_four_fps() {
        use minicraft_client::core::game_loop::GameLoopClock;
        use std::time::{Duration, Instant};
        let mut expected = None;
        for fps in [30_u64, 60, 120, 300] {
            let mut world = tiny_world();
            let start = Instant::now();
            let mut clock = GameLoopClock::starting_at(start);
            let input = Input {
                right: true,
                ..Default::default()
            };
            for frame in 1..=fps * 3 {
                // One second playing, one unfocused, one playing. Timestamps
                // are exact fixtures; this does not test OS event sampling.
                let focused = frame <= fps || frame > fps * 2;
                for _ in
                    0..clock.updates_at(start + Duration::from_nanos(frame * 1_000_000_000 / fps))
                {
                    world.begin_display_tick();
                    world.tick_after_display(focused, &input);
                }
            }
            assert_eq!((world.tick, world.clock.get().day_tick), (120, 179));
            let state: serde_json::Value =
                serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
            if let Some(expected) = &expected {
                assert_eq!(&state, expected);
            } else {
                expected = Some(state);
            }
        }
    }

    #[test]
    fn save_resume_preserves_previous_pause_gate_and_calendar_rollover() {
        let mut world = tiny_world();
        world.tick(&Input::default());
        world.clock.set_day_tick(DAY_LENGTH - 1);
        let saved = world.to_save_string().unwrap();
        let mut restored = World::from_save_string(&saved).unwrap();
        for target in [&mut world, &mut restored] {
            target.begin_display_tick();
            target.tick_after_display(false, &Input::default());
            assert_eq!(
                (target.days, target.clock.get().day_tick, target.tick),
                (2, 0, 1)
            );
        }
        let as_value = |world: &World| {
            serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap()).unwrap()
        };
        assert_eq!(as_value(&world), as_value(&restored));
        let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
        legacy["world"].as_object_mut().unwrap().remove("updater");
        assert!(World::from_save_string(&legacy.to_string()).is_ok());
    }

    #[test]
    fn core_prelude_autosave_captures_before_calendar_score_and_input() {
        let mut world = tiny_world();
        world.updater = Updater::from_state(false, 7_200);
        world.mode = GameMode::Score;
        world.score_ticks = 0;
        world.clock.set_day_tick(42);
        let position = (world.player.x, world.player.y);
        let input = Input {
            right: true,
            ..Default::default()
        };
        let failure = world.tick_with_autosave(false, &input, true, |_| Err("write failed".into()));
        assert!(matches!(failure, Err(error) if error == "write failed"));
        assert_eq!(
            (
                world.updater.autosave_ticks(),
                world.clock.get().day_tick,
                world.score_ticks
            ),
            (7_200, 42, 0)
        );
        let mut saved = None;
        world
            .tick_with_autosave(false, &input, true, |snapshot| {
                saved = Some(snapshot.to_save_string()?);
                Ok(())
            })
            .unwrap();
        let snapshot = World::from_save_string(&saved.unwrap()).unwrap();
        assert_eq!(
            (
                snapshot.clock.get().day_tick,
                snapshot.score_ticks,
                snapshot.game_over
            ),
            (42, 0, false)
        );
        assert_eq!(
            (
                world.clock.get().day_tick,
                world.score_ticks,
                world.game_over
            ),
            (43, -1, true)
        );
        assert_eq!(world.updater.autosave_ticks(), 0);
        assert_eq!(world.tick, 0);
        assert_eq!((world.player.x, world.player.y), position);
    }

    #[test]
    fn core_prelude_autosave_gates_dead_disabled_and_game_over_and_resets_manual_save() {
        for (enabled, alive, over) in [
            (true, true, false),
            (false, true, false),
            (true, false, false),
            (true, true, true),
        ] {
            let mut world = tiny_world();
            world.updater = Updater::from_state(false, 7_199);
            world.player.health = if alive { 10 } else { 0 };
            world.game_over = over;
            let mut saves = 0;
            world
                .tick_with_autosave(false, &Input::default(), enabled, |_| {
                    saves += 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(saves, 0);
            assert_eq!(world.updater.autosave_ticks(), 7_200);
            world
                .tick_with_autosave(false, &Input::default(), enabled, |_| {
                    saves += 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(saves, usize::from(enabled && alive && !over));
            assert_eq!(world.updater.autosave_ticks(), 0);
            world.tick_after_display(false, &Input::default());
            world.reset_autosave();
            assert_eq!(world.updater.autosave_ticks(), 0);
        }
    }

    #[test]
    fn core_prelude_score_uses_previous_pause_even_when_focus_is_lost() {
        let mut world = tiny_world();
        world.mode = GameMode::Score;
        world.score_ticks = 2;
        world.tick(&Input::default()); // Java initially paused=true.
        assert_eq!(world.score_ticks, 2);
        world.pause.open();
        world.tick(&Input::default()); // Pause boundary still uses old false.
        assert_eq!(world.score_ticks, 1);
        world.tick_after_display(false, &Input::default());
        assert_eq!(world.score_ticks, 1);
        world.pause.close();
        world.tick(&Input::default()); // Resume uses old true.
        assert_eq!(world.score_ticks, 1);
        world.tick_after_display(false, &Input::default());
        assert_eq!((world.score_ticks, world.game_over), (0, false));
        world.tick_after_display(false, &Input::default());
        assert_eq!((world.score_ticks, world.game_over), (-1, true));
        world.tick_after_display(false, &Input::default());
        assert_eq!(world.score_ticks, -1);
    }

    #[test]
    fn core_prelude_save_continuation_and_legacy_boolean_updater_are_supported() {
        let mut world = tiny_world();
        world.updater = Updater::from_state(false, 7_200);
        world.mode = GameMode::Score;
        world.score_ticks = 0;
        let saved = world.to_save_string().unwrap();
        let mut restored = World::from_save_string(&saved).unwrap();
        for target in [&mut world, &mut restored] {
            let mut saves = 0;
            target
                .tick_with_autosave(false, &Input::default(), true, |_| {
                    saves += 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(saves, 1);
        }
        let value = |world: &World| {
            serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap()).unwrap()
        };
        assert_eq!(value(&world), value(&restored));
        assert_eq!(
            World::from_save_string(&world.to_save_string().unwrap())
                .unwrap()
                .score_ticks,
            -1
        );
        for paused in [false, true] {
            let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
            legacy["world"]["updater"] = paused.into();
            let old = World::from_save_string(&legacy.to_string()).unwrap();
            assert_eq!(old.updater, Updater::from_paused(paused));
        }
    }

    #[test]
    fn core_prelude_autosave_and_score_are_identical_at_four_render_rates() {
        use minicraft_client::core::game_loop::GameLoopClock;
        use std::time::{Duration, Instant};
        let mut expected = None;
        for fps in [30_u64, 60, 120, 300] {
            let mut world = tiny_world();
            world.updater = Updater::from_state(false, 7_190);
            world.mode = GameMode::Score;
            world.score_ticks = 120;
            let start = Instant::now();
            let mut clock = GameLoopClock::starting_at(start);
            let mut saves = Vec::new();
            for frame in 1..=fps {
                for _ in
                    0..clock.updates_at(start + Duration::from_nanos(frame * 1_000_000_000 / fps))
                {
                    world.begin_display_tick();
                    world
                        .tick_with_autosave(false, &Input::default(), true, |snapshot| {
                            saves.push(
                                serde_json::from_str::<serde_json::Value>(
                                    &snapshot.to_save_string().unwrap(),
                                )
                                .unwrap(),
                            );
                            Ok(())
                        })
                        .unwrap();
                }
            }
            assert_eq!(
                (world.score_ticks, world.updater.autosave_ticks()),
                (60, 49)
            );
            assert_eq!(saves.len(), 1);
            let state = (
                saves,
                serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap())
                    .unwrap(),
            );
            if let Some(expected) = &expected {
                assert_eq!(&state, expected);
            } else {
                expected = Some(state);
            }
        }
    }

    #[test]
    fn score_end_game_is_deferred_focus_gated_and_save_resumable() {
        let mut world = tiny_world();
        world.mode = GameMode::Score;
        world.updater = Updater::from_paused(false);
        world.score_ticks = 0;
        assert!(matches!(
            world.tick(&Input {
                select: true,
                ..Default::default()
            }),
            super::WorldAction::None
        ));
        assert!(!world.end_game.unwrap().active);
        assert!(!world.end_game_visible());
        world.begin_display_tick();
        world.tick_after_display(
            false,
            &Input {
                select: true,
                ..Default::default()
            },
        );
        assert_eq!(world.end_game.unwrap().display_timer, 60);
        for _ in 0..59 {
            world.tick(&Input::default());
        }
        assert!(!world.end_game_visible());
        let mut loaded = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        loaded.tick(&Input::default());
        assert!(loaded.end_game_visible());
        assert_eq!(loaded.end_game.unwrap().input_delay, 30);
        assert!(matches!(
            loaded.tick(&Input {
                exit: true,
                ..Default::default()
            }),
            super::WorldAction::None
        ));
        assert!(matches!(
            loaded.tick(&Input {
                select: true,
                ..Default::default()
            }),
            super::WorldAction::ReturnToTitle
        ));
    }

    #[test]
    fn score_end_game_accepts_select_while_hidden_after_activation() {
        let mut world = tiny_world();
        world.mode = GameMode::Score;
        world.updater = Updater::from_paused(false);
        world.score_ticks = 0;
        world.tick(&Input::default());
        world.book_open = Some((crate::content::Book::ALL[0], 0));
        assert!(matches!(
            world.tick(&Input {
                select: true,
                ..Default::default()
            }),
            super::WorldAction::ReturnToTitle
        ));
        assert!(!world.end_game_visible());
    }

    #[test]
    fn player_display_gate_freezes_player_and_ignores_hud_on_menu_close_tick() {
        let mut world = tiny_world();
        world.inventory_open = true;
        world.player.attack_time = 7;
        world.player.hurt_time = 9;
        world.player.stamina = 3;
        world.player.potion_effects[0] = 100;
        let before = serde_json::to_value(&world.player).unwrap();
        world.tick(&Input {
            toggle_hud: true,
            attack: true,
            right: true,
            ..Default::default()
        });
        assert_eq!(serde_json::to_value(&world.player).unwrap(), before);
        assert!(world.render_gui);
        world.tick(&Input {
            exit: true,
            toggle_hud: true,
            ..Default::default()
        });
        assert!(!world.inventory_open);
        assert!(world.render_gui); // Controls run before this tick closes Display.
        world.tick(&Input {
            toggle_hud: true,
            ..Default::default()
        });
        assert!(!world.render_gui);
    }

    #[test]
    fn renderer_hud_toggle_keeps_menu_and_notification_feedback() {
        let mut world = tiny_world();
        let assets = crate::assets::Assets::load(&[]).unwrap();
        let locale = crate::localization::Localization::load("en-us", &[]);
        let mut visible = crate::gfx::Screen::new();
        let mut hidden = crate::gfx::Screen::new();
        world.notify("HUD notice", 181);
        let mut input = Input::default();
        input.latch(Input {
            toggle_hud: true,
            ..Default::default()
        });
        world.tick(&input.take_tick());
        assert!(!world.render_gui);
        world.tick(&input.take_tick());
        assert!(!world.render_gui); // A latched press is consumed once.
        world.pause.open();
        world.begin_display_tick();
        world.render(&mut hidden, &assets, &locale);
        assert_eq!(world.notifications.messages(), ["HUD notice"]);
        assert_eq!(world.notifications.ticks(), 181); // GUI did not run.
        world.tick_after_display(
            false,
            &Input {
                toggle_hud: true,
                ..Default::default()
            },
        );
        assert!(!world.render_gui); // Unfocused keys cannot toggle.
        world.tick(&Input {
            toggle_hud: true,
            ..Default::default()
        });
        assert!(!world.render_gui); // Player.tick returns before HUD controls in menus.
        world.render_gui = true; // Independently test Renderer gate combinations.
        world.render(&mut visible, &assets, &locale);
        assert!(world.notifications.messages().is_empty());
        // The same opaque Pause frame must be present with either HUD setting.
        let mut menu = crate::gfx::Screen::new();
        super::render_pause(&mut menu, &assets, &locale, &world);
        for (index, &pixel) in menu.pixels().iter().enumerate() {
            if pixel != 0 {
                assert_eq!(hidden.pixels()[index], pixel);
                assert_eq!(visible.pixels()[index], pixel);
            }
        }
        world.render_gui = false;
        assert!(
            World::from_save_string(&world.to_save_string().unwrap())
                .unwrap()
                .render_gui
        );
    }

    #[test]
    fn notifications_follow_gui_feedback_even_while_paused_and_unfocused() {
        let mut world = tiny_world();
        let assets = crate::assets::Assets::load(&[]).unwrap();
        let locale = crate::localization::Localization::load("en-us", &[]);
        let mut screen = crate::gfx::Screen::new();
        world.add_notification("First notice");
        world.tick_after_display(false, &Input::default());
        assert_eq!(world.notifications.ticks(), 0);
        world.pause.open();
        world.begin_display_tick();
        world.render(&mut screen, &assets, &locale);
        for _ in 0..180 {
            world.tick_after_display(false, &Input::default());
        }
        assert_eq!(world.notifications.ticks(), 180);
        world.render(&mut screen, &assets, &locale);
        assert_eq!(world.notifications.messages(), ["First notice"]);
        world.tick_after_display(false, &Input::default());
        world.render(&mut screen, &assets, &locale);
        assert!(world.notifications.messages().is_empty());
        assert_eq!(world.tick, 0);
    }

    #[test]
    fn notifications_are_hidden_and_frozen_by_sleep_status_on_render() {
        let mut world = tiny_world();
        let assets = crate::assets::Assets::load(&[]).unwrap();
        let locale = crate::localization::Localization::load("en-us", &[]);
        let mut screen = crate::gfx::Screen::new();
        world.notify("Keep this notice", 170);
        world.render(&mut screen, &assets, &locale);
        world.sleeping = 120;
        world.render(&mut screen, &assets, &locale);
        assert!(!world.notifications.ticking());
        for _ in 0..100 {
            world.tick_after_display(false, &Input::default());
        }
        assert_eq!(world.notifications.ticks(), 170);
        world.sleeping = 0;
        world.render(&mut screen, &assets, &locale);
        assert_eq!(world.notifications.messages(), ["Keep this notice"]);
        assert!(world.notifications.ticking());
        for (day_tick, days, allowed) in [
            (0, 1, false),
            (40_499, 1, false),
            (40_500, 1, true),
            (8_099, 2, true),
            (8_100, 2, false),
            (64_799, 1, true),
        ] {
            let mut world = tiny_world();
            world.clock.set_day_tick(day_tick);
            world.days = days;
            world.player.direction = super::Direction::Down;
            world.levels[0].entities.spawn_furniture(
                FurnitureKind::Bed,
                world.player.x,
                world.player.y + 12,
            );
            world.use_target();
            assert_eq!(world.sleeping > 0, allowed, "day {days}, tick {day_tick}");
            assert_eq!(world.notifications.messages().is_empty(), allowed);
            if day_tick == 40_499 {
                assert_eq!(
                    world.notifications.messages(),
                    ["Can't sleep! 0Min 1 Sec left!"]
                );
            }
        }
    }

    #[test]
    fn notifications_save_success_is_once_only_and_failure_is_silent() {
        let mut world = tiny_world();
        world.updater = Updater::from_state(false, 7_200);
        assert!(
            world
                .tick_with_autosave(false, &Input::default(), true, |_| Err("disk".into()))
                .is_err()
        );
        assert!(world.notifications.messages().is_empty());
        let mut snapshot = String::new();
        world
            .tick_with_autosave(false, &Input::default(), true, |saved| {
                snapshot = saved.to_save_string()?;
                Ok(())
            })
            .unwrap();
        assert!(
            World::from_save_string(&snapshot)
                .unwrap()
                .notifications
                .messages()
                .is_empty()
        );
        assert_eq!(world.notifications.messages(), ["World Saved!"]);
        world
            .tick_with_autosave(false, &Input::default(), true, |_| panic!("duplicate save"))
            .unwrap();
        assert_eq!(world.notifications.messages().len(), 1);
        world.record_save_success();
        assert_eq!(
            world.notifications.messages(),
            ["World Saved!", "World Saved!"]
        );
        assert_eq!(
            (world.notifications.ticks(), world.updater.autosave_ticks()),
            (0, 0)
        );
    }

    #[test]
    fn notifications_localize_at_enqueue_and_preserve_java_boss_order() {
        use std::sync::Arc;
        let mut world = tiny_world();
        let french = Arc::new(crate::localization::Localization::load("fr-fr", &[]));
        let saved_text = french.text("minicraft.notification.world_saved").to_owned();
        world.set_notification_context(french, "Q");
        world.record_save_success();
        world.set_notification_context(
            Arc::new(crate::localization::Localization::load("en-us", &[])),
            "ESCAPE",
        );
        assert_eq!(world.notifications.messages(), [saved_text]);
        world.notifications.clear();
        world.handle_defeated_mob(super::spawn::NaturalMob::AirWizard);
        let messages = world.notifications.messages();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], "Air Wizard Defeated!");
        assert!(messages[1].starts_with("Achivement unlocked: ")); // Java locale typo retained.
        assert_eq!(messages[2], "The Dungeon is now open!");
        assert_eq!(world.notifications.ticks(), -400);
        world.notifications.clear();
        world.handle_defeated_mob(super::spawn::NaturalMob::AirWizard);
        assert_eq!(world.notifications.messages(), ["Air Wizard Defeated!"]);
        assert_eq!(world.notifications.ticks(), 0);
    }

    #[test]
    fn notifications_save_restores_queue_feedback_and_reads_legacy_slot() {
        let mut world = tiny_world();
        world.notifications =
            super::Notifications::from_state(vec!["One".into(), "Two".into()], -123, true);
        let saved = world.to_save_string().unwrap();
        let mut restored = World::from_save_string(&saved).unwrap();
        for target in [&mut world, &mut restored] {
            target.tick_after_display(false, &Input::default());
        }
        assert_eq!(world.notifications, restored.notifications);
        assert_eq!(restored.notifications.ticks(), -122);
        let mut old: serde_json::Value = serde_json::from_str(&saved).unwrap();
        old["world"]["notification"] = serde_json::json!(["Legacy notice", 60]);
        let restored = World::from_save_string(&old.to_string()).unwrap();
        assert_eq!(restored.notifications.messages(), ["Legacy notice"]);
        assert_eq!(restored.notifications.ticks(), 121);
        old["world"]["notification"] = serde_json::Value::Null;
        assert!(
            World::from_save_string(&old.to_string())
                .unwrap()
                .notifications
                .messages()
                .is_empty()
        );
        old["world"].as_object_mut().unwrap().remove("notification");
        assert!(
            World::from_save_string(&old.to_string())
                .unwrap()
                .notifications
                .messages()
                .is_empty()
        );
    }

    #[test]
    fn pause_render_keeps_old_page_until_lifecycle_update() {
        let mut world = tiny_world();
        world.populate_pause_preview();
        let assets = crate::assets::Assets::load(&[]).unwrap();
        let localization = crate::localization::Localization::load("en-us", &[]);
        let draw = |world: &World| {
            let mut screen = crate::gfx::Screen::new();
            super::render_pause(&mut screen, &assets, &localization, world);
            screen.pixels().to_vec()
        };
        let parent = draw(&world);
        world.tick(&Input {
            select: true,
            ..Default::default()
        });
        assert_eq!(draw(&world), parent);
        world.tick(&Input::default());
        let options = draw(&world);
        assert_ne!(options, parent);
        world.tick(&Input {
            exit: true,
            ..Default::default()
        });
        assert_eq!(draw(&world), options);
        world.tick(&Input::default());
        assert_eq!(draw(&world), parent);
    }

    #[test]
    fn pause_pending_child_save_restores_parent_and_reads_legacy_keys() {
        let mut world = tiny_world();
        world.populate_pause_preview();
        world.tick(&Input {
            select: true,
            ..Default::default()
        });
        let saved = world.to_save_string().unwrap();
        let mut restored = World::from_save_string(&saved).unwrap();
        assert_eq!(restored.pause.view().page, PausePage::Options);
        restored.tick(&Input {
            exit: true,
            ..Default::default()
        });
        assert_eq!(restored.pause.view().selection, 1);
        let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
        legacy["world"]
            .as_object_mut()
            .unwrap()
            .remove("pause_frames");
        let mut restored = World::from_save_string(&legacy.to_string()).unwrap();
        restored.tick(&Input {
            exit: true,
            ..Default::default()
        });
        assert_eq!(restored.pause.view().selection, 1);
        // Data added in this batch is not allowed to create arbitrary cycles or roots.
        let mut malformed: serde_json::Value = serde_json::from_str(&saved).unwrap();
        malformed["world"]["pause_frames"][0]["page"] = "QuestDetail".into();
        assert!(World::from_save_string(&malformed.to_string()).is_err());
    }

    #[test]
    fn legacy_pause_confirm_cancel_uses_actual_main_menu_length() {
        for quests in [false, true] {
            let mut world = tiny_world();
            world.quests_enabled = quests;
            world.prepare_pause_audio_parity();
            let last = pause_actions(quests).len() - 1;
            world.pause.view_mut().selection = last;
            world.tick(&Input {
                select: true,
                ..Default::default()
            });
            let mut legacy: serde_json::Value =
                serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
            legacy["world"]
                .as_object_mut()
                .unwrap()
                .remove("pause_frames");
            let mut restored = World::from_save_string(&legacy.to_string()).unwrap();
            restored.tick(&Input {
                exit: true,
                ..Default::default()
            });
            assert_eq!(restored.pause.view().selection, last);
            assert!(!restored.pause.view().confirm);
        }
    }

    #[test]
    fn quest_nested_return_restores_pause_parent_without_world_progress() {
        let mut world = tiny_world();
        world.prepare_quests_audio_parity();
        let select = Input {
            select: true,
            ..Default::default()
        };
        for _ in 0..3 {
            world.tick(&select);
        }
        assert_eq!(world.pause.frames().len(), 5);
        let mut restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        for _ in 0..4 {
            restored.tick(&Input {
                exit: true,
                ..Default::default()
            });
        }
        assert_eq!(restored.pause.view().page, PausePage::Main);
        assert_eq!(restored.pause.view().selection, 3);
        assert_eq!(restored.tick, 0);
    }

    #[test]
    fn ordinary_tile_hit_uses_java_damage_timing_and_tool_wear() {
        let mut world = tiny_world();
        let seed = 47_227;
        world.random = JavaRandom::new(seed);
        let expected_damage = (JavaRandom::new(seed).next_int(3) + 1) as u16;
        let target = 9 + 8 * world.width;
        world.levels[0].tiles[target] = Tile::Tree;
        world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Sword, ToolTier::Wood));
        world.player.active_item = Some(ActiveItem::Tool(0));

        world.attack();

        assert_eq!(world.levels[0].data[target], expected_damage);
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
        let seed = 47_227;
        world.random = JavaRandom::new(seed);
        let expected_damage = (JavaRandom::new(seed).next_int(5) + 10) as u16;
        let target = 9 + 8 * world.width;
        world.levels[0].tiles[target] = Tile::Tree;
        world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood));
        world.player.active_item = Some(ActiveItem::Tool(0));

        world.attack();

        assert_eq!(world.levels[0].data[target], expected_damage);
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

        world.player.inventory.add(ItemId::Wood, 5);
        world.use_target();
        assert!(world.inventory_open);
        assert_eq!(world.crafting_station, Some(FurnitureKind::Workbench));

        world.inventory_selection = sorted_recipe_views(
            Some(FurnitureKind::Workbench),
            &world.player.inventory,
            false,
        )
        .iter()
        .position(|recipe| {
            matches!(recipe.output, RecipeOutput::Tool(tool) if tool.kind == ToolKind::Axe)
        })
        .unwrap();
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
        assert!(world.notifications.messages().is_empty());
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
        world.clock.set_day_tick(48_600);
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
        world.clock.set_day_tick(49_000);
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
        assert_eq!(loaded.clock.get().day_tick, 49_000);
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
        // Updater starts paused: the first gameplay dispatch does not advance
        // the calendar prelude. Two calendar days require one additional tick.
        assert_eq!(continuous.days, 2);
        assert_eq!(continuous.clock.get().day_tick, DAY_LENGTH - 1);
        continuous.tick(&Input::default());
        assert_eq!(continuous.days, 3);
        assert_eq!(continuous.clock.get().day_tick, 0);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(15),
            "headless two-day soak exceeded the generous regression budget"
        );
        assert!(continuous.levels[0].entities.entities().len() < 32);
    }
}
