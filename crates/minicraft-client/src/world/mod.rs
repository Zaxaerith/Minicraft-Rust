use crate::core::notifications::Notifications;
use crate::core::updater::Updater;
use crate::screen::pause::{PauseAction, PausePage, PauseSession, pause_actions};

mod clock_save;
mod core_update;
mod death_transition;
mod end_game;
mod level_update;
pub(crate) mod notifications;
mod parity_hooks;
mod pause;
mod pause_save;
mod pause_ui;
mod player_update;
mod render;
mod sleep;

mod boss;
mod entity;
mod entity_render;
mod furniture_action;
mod generation;
mod hud;
mod inventory_ui;
mod level_map;
mod overlay_ui;
mod player_actions;
mod player_render;
mod quest_tree;
mod quest_ui;
pub(crate) mod random;
#[cfg(test)]
pub(crate) mod registry_tests;
pub mod spawn;
mod structure;
mod tile;
mod tile_attack;
mod tile_behavior;
mod tile_interact;

pub use crate::gfx::gui::render_menu_frame;
pub use hud::render_hud_parity;
pub(crate) use hud::{
    render_boss_bar, render_hud, render_potion_overlay, render_quest_overlay, render_score_overlay,
};
pub use inventory_ui::render_inventory_parity;
pub(crate) use inventory_ui::{creative_entries, render_inventory, render_menu_title};
pub use level_map::Level;
pub(crate) use pause_ui::render_pause;
pub(crate) use player_render::player_sprite_frame;
pub use tile::{
    Tile, connection_mask, damage_tile, find_stair_site, fluid_animation_phase,
    increment_tile_tick, tile_placement, tile_tick, underlay_layers,
};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::assets::Assets;
use crate::gfx::{HEIGHT, Screen, WIDTH};
use crate::io::Input;
use crate::io::audio::SoundEffect;
use crate::item::{
    ArmorKind, Inventory, ItemId, ItemStack, PotionKind, ToolItem, ToolKind, ToolTier,
    WORKBENCH_STATION_RECIPES, WORKBENCH_TOOL_RECIPES,
};
use crate::progression::{Book, ProgressEvent, ProgressState};
use crate::resource::localization::Localization;

pub use crate::entity::furniture::FurnitureKind;
pub use crate::saveload::{import_java_save, probe_java_save};
use entity::{EntityArena, EntityKind};
use random::JavaRandom;

const TILE_SIZE: i32 = 16;
pub(crate) const DAY_LENGTH: u32 = crate::core::updater::DAY_LENGTH;
pub(crate) const MAX_STAT: u8 = 10;
pub(crate) fn default_score_minutes() -> usize {
    20
}

fn default_render_gui() -> bool {
    true
}
pub(crate) const MAX_HEALTH: u8 = 20;
const MAX_HUNGER_TICKS: i16 = 400;
const HUNGER_STAMINA_STEPS: [i16; 3] = [10, 7, 5];
const HUNGER_TICK_INTERVALS: [u64; 3] = [120, 30, 10];
const HUNGER_MOVE_STEPS: [u8; 3] = [8, 3, 1];
const STARVATION_HEALTH_FLOORS: [u8; 3] = [5, 3, 0];

#[allow(unused_imports)]
pub use crate::level::{GameMode, TerrainType, Theme, WorldSpec};

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) enum Direction {
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
pub(crate) enum ActiveItem {
    Stack(ItemId),
    Tool(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreativeEntry {
    Stack(ItemId),
    Tool(ToolItem),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Player {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) direction: Direction,
    pub(crate) walk_distance: u32,
    pub(crate) attack_time: u8,
    #[serde(default = "default_direction")]
    pub(crate) attack_direction: Direction,
    #[serde(default)]
    pub(crate) attack_item: Option<ActiveItem>,
    pub(crate) health: u8,
    pub(crate) max_health: u8,
    pub(crate) stamina: u8,
    pub(crate) hunger: u8,
    pub(crate) armor: u8,
    pub(crate) armor_kind: Option<ArmorKind>,
    pub(crate) armor_damage_buffer: u8,
    pub(crate) hurt_time: u8,
    pub(crate) stamina_recharge: u8,
    pub(crate) stamina_recharge_delay: u8,
    pub(crate) hunger_stamina_count: i16,
    pub(crate) hunger_ticks: i16,
    pub(crate) step_count: u8,
    pub(crate) hunger_charge_delay: u16,
    pub(crate) hunger_starve_delay: u8,
    pub(crate) potion_effects: [u16; PotionKind::ALL.len()],
    pub(crate) regen_tick: u8,
    pub(crate) fishing_level: Option<u8>,
    pub(crate) fishing_ticks: u16,
    pub(crate) watering_content: u16,
    pub(crate) clothing: ItemId,
    #[serde(default)]
    pub(crate) imported_shirt_color: Option<i32>,
    pub(crate) inventory: Inventory,
    pub(crate) active_item: Option<ActiveItem>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SignEditor {
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
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) levels: Vec<Level>,
    pub(crate) current_level: usize,
    pub(crate) player: Player,
    pub(crate) seed: i64,
    pub(crate) tick: u64,
    #[serde(skip, default = "crate::assets::animation_millis")]
    animation_enter_millis: i64,
    #[serde(flatten, with = "clock_save")]
    pub(crate) clock: crate::core::clock::SharedClock,
    #[serde(skip, default = "default_render_gui")]
    render_gui: bool,
    #[serde(default, with = "core_update")]
    updater: Updater,
    pub(crate) days: u32,
    pub(crate) difficulty: usize,
    pub(crate) mode: GameMode,
    pub(crate) score: u32,
    pub(crate) score_ticks: i32,
    #[serde(default = "default_score_minutes")]
    pub(crate) score_minutes: usize,
    score_multiplier: u8,
    multiplier_ticks: u16,
    game_over: bool,
    #[serde(default, with = "end_game")]
    end_game: Option<crate::screen::end_game::EndGameDisplay>,
    story_complete: bool,
    sleeping: u16,
    #[serde(default)]
    bed_level: Option<usize>,
    #[serde(default)]
    pub(crate) respawn_tile: Option<(i32, i32)>,
    #[serde(skip)]
    sleep_speed: u8,
    #[serde(default)]
    player_removed: bool,
    #[serde(default)]
    player_dead_time: u32,
    #[serde(default)]
    pending_level_change: i8,
    #[serde(default)]
    on_stair_delay: u8,
    #[serde(default)]
    on_fall_delay: u8,
    #[serde(default)]
    level_transition: Option<death_transition::LevelTransition>,
    #[serde(default)]
    death_open: bool,
    #[serde(default)]
    death_selection: usize,
    pub(crate) tutorials_enabled: bool,
    pub(crate) quests_enabled: bool,
    pub(crate) show_quests: bool,
    pub(crate) progress: ProgressState,
    pub(crate) signs: Vec<HashMap<usize, String>>,
    pub(crate) sign_editor: Option<SignEditor>,
    pub(crate) book_open: Option<(Book, usize)>,
    #[serde(flatten, with = "pause_save")]
    pub(crate) pause: PauseSession,
    #[serde(skip)]
    quest_tree_state: quest_tree::State,
    pub(crate) inventory_open: bool,
    inventory_selection: usize,
    inventory_item_selection: usize,
    inventory_pane: usize,
    pub(crate) crafting_station: Option<FurnitureKind>,
    /// Open ContainerDisplay chest entity (Java Chest.use → ContainerDisplay).
    #[serde(default)]
    container_id: Option<u64>,
    #[serde(default)]
    crafting_order: Vec<usize>,
    #[serde(default)]
    personal_crafting: bool,
    #[serde(default, rename = "notification", with = "notifications")]
    pub(crate) notifications: Notifications,
    #[serde(skip)]
    notification_context: notifications::Context,
    pub(crate) air_wizard_defeated: bool,
    pub(crate) obsidian_knight_defeated: bool,
    random: random::JavaRandom,
    #[serde(skip)]
    sound_events: Vec<SoundEffect>,
    /// Per-host mirror of Java `Tile.tickCount` increments (race-free tests).
    #[serde(skip)]
    tile_ticks: u32,
    /// Per-host mirror of Java `Sound.tick()` calls (race-free tests).
    #[serde(skip)]
    sound_ticks: u64,
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

    /// Per-host Java `Tile.tickCount` increments (gameplay + sleeping level ticks).
    pub fn tile_ticks(&self) -> u32 {
        self.tile_ticks
    }

    /// Per-host Java `Sound.tick()` pumps (every Updater tick, including menus).
    pub fn sound_ticks(&self) -> u64 {
        self.sound_ticks
    }

    /// World pause/inventory/crafting sounds are emitted on the logical tick
    /// via take_sound_events. Frame-level prediction is retired for this host.
    pub fn immediate_menu_sounds(&self, _input: &Input) -> Vec<SoundEffect> {
        Vec::new()
    }

    fn sound(&mut self, effect: SoundEffect) {
        self.sound_events.push(effect);
    }

    pub(crate) fn validate_save(&self) -> Result<(), String> {
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
        progress.begin_new_world();
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
                imported_shirt_color: None,
                inventory,
                active_item: None,
            },
            seed,
            tick: 0,
            clock: Default::default(),
            animation_enter_millis: crate::assets::animation_millis(),
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
            score_minutes: match options.score_minutes {
                10 | 20 | 40 | 60 | 120 => options.score_minutes,
                _ => 20,
            },
            score_multiplier: 1,
            multiplier_ticks: 300,
            game_over: false,
            end_game: None,
            story_complete: false,
            sleeping: 0,
            bed_level: None,
            respawn_tile: Some((spawn_x, spawn_y)),
            sleep_speed: 1,
            player_removed: false,
            player_dead_time: 0,
            pending_level_change: 0,
            on_stair_delay: 0,
            on_fall_delay: 0,
            level_transition: None,
            death_open: false,
            death_selection: 0,
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
            container_id: None,
            crafting_order: Vec::new(),
            personal_crafting: false,
            notifications: Notifications::default(),
            notification_context: notifications::Context::default(),
            air_wizard_defeated: false,
            obsidian_knight_defeated: false,
            random: random::JavaRandom::new(seed ^ 0x05EE_D224),
            sound_events: Vec::new(),
            tile_ticks: 0,
            sound_ticks: 0,
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
        // Java Updater no-menu branch. `Some` from death/transition means the
        // death menu or LevelTransitionDisplay owns this tick (player early-out,
        // no level tick). Terminal actions already returned via that path.
        if let Some(action) = self.tick_death_and_transition(input) {
            return action;
        }
        if input.exit {
            if self.inventory_open {
                self.inventory_open = false;
                self.personal_crafting = false;
                self.crafting_station = None;
                self.container_id = None;
                self.sound(SoundEffect::Craft);
            } else if self.pause.is_open() {
                // Java Game.exitDisplay plays craft; AchievementsDisplay.onExit also plays confirm.
                let was_achievements = self.pause.view().page == PausePage::Achievements;
                self.pause.back();
                self.sound(SoundEffect::Craft);
                if was_achievements {
                    self.sound(SoundEffect::Confirm);
                }
            } else {
                self.pause.open();
                self.sound(SoundEffect::Craft);
            }
            return WorldAction::None;
        }
        // Java Updater still *calls* Game.player.tick() while currentDisplay
        // != null, but Player.tick itself returns on an already-requested
        // Display (core/player_update.rs::can_tick) before hurt/attack timers,
        // potion/fishing, hunger and stamina recharge. Returning here — and on
        // the inventory/book/sign/death branches below — is that no-op; do not
        // resume tick_gameplay_player under menus.
        if self.pause.is_open() {
            return self.tick_pause(input);
        }
        if input.craft {
            if self.use_furniture_front() {
                return WorldAction::None;
            }
            if self.inventory_open && self.personal_crafting {
                self.inventory_open = false;
                self.personal_crafting = false;
                self.sound(SoundEffect::Craft);
            } else if self.inventory_open
                && self.crafting_station.is_none()
                && self.container_id.is_none()
            {
                // Make the Java personal-crafting display reachable directly from
                // the already-open player inventory instead of closing the menu.
                self.personal_crafting = true;
                self.inventory_selection = 0;
                self.refresh_crafting_order();
                self.sound(SoundEffect::Craft);
            } else if !self.inventory_open {
                self.inventory_open = true;
                self.personal_crafting = true;
                self.player.active_item = None;
                self.crafting_station = None;
                self.container_id = None;
                self.inventory_selection = 0;
                self.refresh_crafting_order();
                self.sound(SoundEffect::Craft);
            }
        } else if input.menu {
            if self.use_furniture_front() {
                return WorldAction::None;
            }
            if self.inventory_open && self.container_id.is_some() {
                self.close_container();
                self.sound(SoundEffect::Craft);
            } else if self.inventory_open {
                self.inventory_open = false;
                self.personal_crafting = false;
                self.crafting_station = None;
                self.container_id = None;
                self.sound(SoundEffect::Craft);
            } else {
                self.inventory_open = true;
                self.personal_crafting = false;
                self.sound(SoundEffect::Craft);
                self.player.active_item = None;
                self.crafting_station = None;
                self.container_id = None;
                self.inventory_pane = usize::from(
                    self.mode == GameMode::Creative && self.player.inventory.used_slots() == 0,
                );
            }
        }
        if self.inventory_open {
            if self.container_id.is_some() {
                return self.tick_container_ui(input);
            }
            if self.personal_crafting || self.crafting_station.is_some() {
                self.inventory_pane = 1;
                let count = self.crafting_recipe_count();
                if input.up_pressed {
                    self.inventory_selection = self
                        .inventory_selection
                        .checked_sub(1)
                        .unwrap_or(count.saturating_sub(1));
                    self.sound(SoundEffect::Select);
                }
                if input.down_pressed {
                    self.inventory_selection = (self.inventory_selection + 1) % count.max(1);
                    self.sound(SoundEffect::Select);
                }
                if (input.select || input.attack) && self.crafting_recipe_count() > 0 {
                    self.craft_selected_recipe();
                    self.sound(SoundEffect::Craft);
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
                    self.sound(SoundEffect::Select);
                }
                if input.down_pressed {
                    self.inventory_selection =
                        (self.inventory_selection + 1) % entries.len().max(1);
                    self.sound(SoundEffect::Select);
                }
                if input.select
                    && let Some(entry) = entries.get(self.inventory_selection).copied()
                {
                    let inserted = match entry {
                        CreativeEntry::Stack(item) => self.player.inventory.add(item, 1) == 0,
                        CreativeEntry::Tool(tool) => self.player.inventory.add_tool(tool).is_some(),
                    };
                    if inserted {
                        self.sound(SoundEffect::Confirm);
                    }
                }
                return WorldAction::None;
            }

            if input.up_pressed {
                let count = self.player.inventory.used_slots();
                self.inventory_item_selection = self
                    .inventory_item_selection
                    .checked_sub(1)
                    .unwrap_or(count.saturating_sub(1));
                self.sound(SoundEffect::Select);
            }
            if input.down_pressed {
                let count = self.player.inventory.used_slots();
                self.inventory_item_selection = (self.inventory_item_selection + 1) % count.max(1);
                self.sound(SoundEffect::Select);
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
                    self.sound(SoundEffect::Craft);
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

        // Java Updater order with no menu: player.tick() first, then level.tick().
        // Level.tickEntity skips Game.player; only the separate Player.tick runs.
        self.tick_gameplay_player(input);

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
        if outcome.player_damage > 0 {
            self.hurt_player(outcome.player_damage, false);
        }
        for (x, y, radius, player_tnt) in outcome.explosions {
            self.apply_explosion(x, y, radius, player_tnt);
        }
        for mob in outcome.defeated_mobs {
            self.handle_defeated_mob(mob);
        }
        // Java Updater: level.tick(true); Tile.tickCount++;
        increment_tile_tick();
        self.tile_ticks = self.tile_ticks.wrapping_add(1);
        self.update_progress(ProgressEvent::InventoryChanged);
        WorldAction::None
    }

    /// Java Player.tick gameplay body, after the no-menu early returns.
    fn move_player(&mut self, horizontal: i32, vertical: i32) -> bool {
        let old_x = self.player.x;
        let old_y = self.player.y;
        // Java Entity: try one axis, then the other; furniture push on collision.
        if horizontal != 0 {
            let next_x = self.player.x + horizontal;
            self.apply_bumped_tile_damage(next_x, self.player.y);
            if self.can_stand(next_x, self.player.y) {
                self.player.x = next_x;
            } else {
                // Java moveByEntityHitBoxChecks: touchedBy uses the *new* hitbox.
                self.levels[self.current_level]
                    .entities
                    .try_push_furniture_near(next_x, self.player.y, horizontal.signum(), 0);
            }
        }
        if vertical != 0 {
            let next_y = self.player.y + vertical;
            self.apply_bumped_tile_damage(self.player.x, next_y);
            if self.can_stand(self.player.x, next_y) {
                self.player.y = next_y;
            } else {
                self.levels[self.current_level]
                    .entities
                    .try_push_furniture_near(self.player.x, next_y, 0, vertical.signum());
            }
        }
        self.player.x != old_x || self.player.y != old_y
    }

    fn attack(&mut self) {
        self.attack_inner();
        self.remove_depleted_active_tool();
    }

    fn remove_depleted_active_tool(&mut self) {
        let Some(ActiveItem::Tool(index)) = self.player.active_item else {
            return;
        };
        if self
            .player
            .inventory
            .tools()
            .get(index)
            .is_some_and(|tool| tool.is_depleted())
        {
            self.player.inventory.tools.remove(index);
            self.player.active_item = None;
            self.player.attack_item = None;
            self.inventory_item_selection = self
                .inventory_item_selection
                .min(self.player.inventory.used_slots().saturating_sub(1));
        }
    }

    fn attack_inner(&mut self) {
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
        self.attack_target_tile(tile_x, tile_y, index);
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
        let Some((tile, bucket)) = crate::item::bucket_action(item, target) else {
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
        // Java ExplosionTileTicker settles explode tiles into holes after 18 ticks.
        let depth = self.levels[self.current_level].depth;
        self.levels[self.current_level]
            .entities
            .spawn_explosion_ticker(
                center_x * TILE_SIZE + 8,
                center_y * TILE_SIZE + 8,
                radius,
                depth,
            );
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

    /// Java Player.respawn(Level): restore stats/position only. Score penalty
    /// and item drops already ran in die_player when health hit zero.
    fn respawn(&mut self) {
        self.current_level = self
            .levels
            .iter()
            .position(|level| level.depth == 0)
            .unwrap_or(self.current_level);
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
        // Java World.resetGame clears notifications when reopening after death.
        self.notifications.clear();
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
                if !crate::core::sleep::can_sleep(self.clock.get().day_tick, self.days > 1) {
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
        }

        // Stairs/infinite fall are scheduled from Player.tick while standing on
        // the tile (death_transition::tick_stair_and_fall), not from interact.
    }
}

fn short_progress_id(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

use furniture_action::furniture_for_item;

impl World {
    /// Java Player.getInteractionTile: origin (x, y-2) plus dir * INTERACT_DIST(12).
    pub(super) fn interaction_tile(&self) -> (i32, i32) {
        let mut x = self.player.x;
        let mut y = self.player.y - 2;
        match self.player.direction {
            Direction::Down => y += 12,
            Direction::Up => y -= 12,
            Direction::Left => x -= 12,
            Direction::Right => x += 12,
        }
        (x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE))
    }

    /// Java Furniture.use via Player.use on menu/craft. Returns true when handled.
    pub(super) fn use_furniture_front(&mut self) -> bool {
        let (offset_x, offset_y) = match self.player.direction {
            Direction::Down => (0, 12),
            Direction::Up => (0, -12),
            Direction::Left => (-12, 0),
            Direction::Right => (12, 0),
        };
        let target_x = self.player.x + offset_x;
        let target_y = self.player.y + offset_y;
        let Some((id, kind, state)) = self.levels[self.current_level]
            .entities
            .furniture_entry_near(target_x, target_y, 14)
        else {
            return false;
        };
        match kind {
            FurnitureKind::Chest | FurnitureKind::DungeonChest => {
                if state == 2 {
                    return false;
                }
                if kind == FurnitureKind::DungeonChest && state == 0 {
                    // Java DungeonChest.use still opens the display; unlock is interact.
                    // Keep open even when locked so the player can see the key requirement.
                }
                self.open_container(id);
                self.sound(SoundEffect::Confirm);
                true
            }
            FurnitureKind::Workbench
            | FurnitureKind::Oven
            | FurnitureKind::Furnace
            | FurnitureKind::Anvil
            | FurnitureKind::Enchanter
            | FurnitureKind::Loom => {
                self.inventory_open = true;
                self.container_id = None;
                self.crafting_station = Some(kind);
                self.personal_crafting = false;
                self.inventory_pane = 1;
                self.inventory_selection = 0;
                self.refresh_crafting_order();
                self.sound(SoundEffect::Craft);
                true
            }
            FurnitureKind::Bed => {
                if !crate::core::sleep::can_sleep(self.clock.get().day_tick, self.days > 1) {
                    let seconds =
                        (40_500_u32.saturating_sub(self.clock.get().day_tick)).div_ceil(60);
                    self.add_notification_args(
                        "minicraft.notification.cannot_sleep",
                        &[&(seconds / 60).to_string(), &(seconds % 60).to_string()],
                    );
                } else {
                    self.enter_bed();
                }
                true
            }
            _ => false,
        }
    }

    pub(super) fn open_container(&mut self, id: u64) {
        self.inventory_open = true;
        self.container_id = Some(id);
        self.personal_crafting = false;
        self.crafting_station = None;
        self.inventory_item_selection = 0;
        self.inventory_selection = 0;
        self.inventory_pane = 0;
    }

    pub(super) fn close_container(&mut self) {
        self.inventory_open = false;
        self.container_id = None;
        self.personal_crafting = false;
        self.crafting_station = None;
    }

    pub(super) fn tick_container_ui(&mut self, input: &Input) -> WorldAction {
        let Some(container_id) = self.container_id else {
            self.close_container();
            return WorldAction::None;
        };
        let chest_exists = self.levels[self.current_level]
            .entities
            .entities()
            .iter()
            .any(|entity| entity.id == container_id && !entity.removed);
        if !chest_exists {
            self.close_container();
            return WorldAction::None;
        }
        if input.menu {
            self.close_container();
            self.sound(SoundEffect::Craft);
            return WorldAction::None;
        }
        // Java Display switches sibling menus with CURSOR-LEFT/RIGHT. Keep
        // MOVE-LEFT/RIGHT independent so remapping movement cannot strand the
        // keyboard user in the player pane.
        if input.cursor_left_pressed {
            self.inventory_pane = 0;
            self.sound(SoundEffect::Select);
        }
        if input.cursor_right_pressed {
            self.inventory_pane = 1;
            self.sound(SoundEffect::Select);
        }
        let (from_count, _to_count) = if self.inventory_pane == 0 {
            (
                self.player.inventory.slots().len(),
                self.chest_used_slots(container_id),
            )
        } else {
            (
                self.chest_used_slots(container_id),
                self.player.inventory.slots().len(),
            )
        };
        if self.inventory_pane == 0 {
            if input.cursor_up_pressed {
                self.inventory_item_selection = self
                    .inventory_item_selection
                    .checked_sub(1)
                    .unwrap_or(from_count.saturating_sub(1));
                self.sound(SoundEffect::Select);
            }
            if input.cursor_down_pressed {
                self.inventory_item_selection =
                    (self.inventory_item_selection + 1) % from_count.max(1);
                self.sound(SoundEffect::Select);
            }
        } else if input.cursor_up_pressed {
            self.inventory_selection = self
                .inventory_selection
                .checked_sub(1)
                .unwrap_or(from_count.saturating_sub(1));
            self.sound(SoundEffect::Select);
        } else if input.cursor_down_pressed {
            self.inventory_selection = (self.inventory_selection + 1) % from_count.max(1);
            self.sound(SoundEffect::Select);
        }
        // Attack/select transfers one; shift attack transfers the stack (Java).
        if input.attack || input.select {
            let transfer_all = input.shift;
            let moved = if self.inventory_pane == 0 {
                let from_sel = self.inventory_item_selection;
                self.transfer_player_to_chest(container_id, from_sel, transfer_all)
            } else {
                let from_sel = self.inventory_selection;
                self.transfer_chest_to_player(container_id, from_sel, transfer_all)
            };
            if moved {
                self.sound(SoundEffect::Confirm);
            } else {
                self.sound(SoundEffect::Select);
            }
        }
        WorldAction::None
    }

    fn chest_used_slots(&self, container_id: u64) -> usize {
        self.levels[self.current_level]
            .entities
            .death_chest_storage(container_id)
            .map(|storage| storage.used_slots())
            .unwrap_or(0)
    }

    fn transfer_player_to_chest(
        &mut self,
        container_id: u64,
        from_sel: usize,
        transfer_all: bool,
    ) -> bool {
        let Some(stack) = self.player.inventory.slots().get(from_sel).copied() else {
            return false;
        };
        if stack.count == 0 {
            return false;
        }
        let amount = if transfer_all { stack.count } else { 1 };
        let Some(storage) = self.levels[self.current_level]
            .entities
            .entities_mut_for_storage(container_id)
        else {
            return false;
        };
        let accepted = storage.add(stack.item, amount);
        let moved = amount - accepted;
        if moved == 0 {
            return false;
        }
        self.player.inventory.remove(stack.item, moved);
        true
    }

    fn transfer_chest_to_player(
        &mut self,
        container_id: u64,
        from_sel: usize,
        transfer_all: bool,
    ) -> bool {
        let Some(stack) = self.levels[self.current_level]
            .entities
            .death_chest_storage(container_id)
            .and_then(|storage| storage.slots().get(from_sel).copied())
        else {
            return false;
        };
        if stack.count == 0 {
            return false;
        }
        let amount = if transfer_all { stack.count } else { 1 };
        let accepted = self.player.inventory.add(stack.item, amount);
        if accepted == amount {
            return false;
        }
        let moved = amount - accepted;
        if let Some(storage) = self.levels[self.current_level]
            .entities
            .entities_mut_for_storage(container_id)
        {
            storage.remove(stack.item, moved);
            true
        } else {
            false
        }
    }
}

fn item_for_furniture(kind: FurnitureKind) -> Option<ItemId> {
    crate::item::item_for_furniture(kind)
}

fn tile_progress_name(tile: Tile) -> String {
    match tile {
        Tile::Wheat => "wheat".to_owned(),
        Tile::Potato => "potato".to_owned(),
        _ => tile.asset_name().replace('_', " ").to_ascii_lowercase(),
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

#[cfg(test)]
fn time_name(tick: u32) -> &'static str {
    match tick / (DAY_LENGTH / 4) {
        0 => "MORN",
        1 => "DAY",
        2 => "EVE",
        _ => "NIGHT",
    }
}

use crate::item::{RecipeOutput, RecipeView, ordered_recipe_views, recipe_can_craft};

fn recipe_image<'a>(recipe: &RecipeView, assets: &'a Assets) -> &'a crate::gfx::Image {
    match recipe.output {
        RecipeOutput::Stack(stack) => assets.item(stack.item),
        RecipeOutput::Tool(tool) => assets.tool(tool),
    }
}

fn recipe_output_count(recipe: &RecipeView, player: &Player) -> usize {
    match recipe.output {
        RecipeOutput::Stack(stack) => usize::from(player.inventory.count(stack.item)),
        RecipeOutput::Tool(expected) => player
            .inventory
            .tools()
            .iter()
            .filter(|tool| tool.kind == expected.kind && tool.tier == expected.tier)
            .count(),
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
