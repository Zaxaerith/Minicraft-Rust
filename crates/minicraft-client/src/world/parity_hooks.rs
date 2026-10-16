//! Deterministic World adapters used by the parity snapshot runners.
use super::*;
use crate::core::updater::Updater;

use crate::progression::{ProgressEvent, ProgressState};

impl World {
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

    pub(super) fn quest_group_views_for_current_tab(
        &self,
    ) -> Vec<crate::progression::QuestGroupView> {
        self.progress
            .quest_group_views(&self.inventory_progress(), self.pause.view().quest_tab == 1)
    }

    pub(super) fn quest_views_for_selected_group(&self) -> Vec<crate::progression::QuestView> {
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

    pub(super) fn quest_tree(&self) -> quest_tree::Tree {
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
            &self.progress,
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
            &self.progress,
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
            selected_have: selected
                .map_or(0, |recipe| super::recipe_output_count(recipe, &self.player)),
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
}

impl World {
    pub(crate) fn java230_render_preview(path: &std::path::Path) -> Result<(), String> {
        use crate::entity::furniture::{FurnitureKind, bed};
        use crate::item::dye::DyeColor;
        use crate::level::tile::{Tile, flower::FlowerVariant};
        let assets = crate::assets::Assets::load(&[])?;
        let localization = crate::resource::localization::Localization::load("en-us", &[]);
        let mut world = crate::world::World::new_with_play_options(
            42,
            crate::level::WorldSpec::new(128, 0, 0),
            crate::world::PlayOptions {
                difficulty: 1,
                mode: GameMode::Survival,
                score_minutes: 20,
                tutorials: false,
                quests: false,
                show_quests: false,
                custom_skin: false,
            },
        );
        world.player.x = 8 * 16 + 8;
        world.player.y = 9 * 16 + 8;
        world.player_removed = true;
        world.render_gui = false;
        world.clock.set_day_tick(18_000);
        let level = &mut world.levels[world.current_level];
        level.tiles.fill(Tile::Grass);
        level.data.fill(0);
        level.entities = Default::default();
        for color in DyeColor::ALL {
            let index = 1 + color as usize + 5 * world.width;
            level.tiles[index] =
                crate::item::wool::wool_tile(color.wool_item()).ok_or("missing wool tile")?;
            level.entities.spawn_furniture_with_state(
                FurnitureKind::Bed,
                (1 + (color as i32 % 8) * 2) * 16 + 8,
                (10 + (color as i32 / 8) * 2) * 16 + 8,
                bed::state_for_color(color),
            );
        }
        for (index, variant) in FlowerVariant::ALL.into_iter().enumerate() {
            let offset = 1 + index % 16 + (7 + index / 16) * world.width;
            level.tiles[offset] = Tile::Flower230;
            level.data[offset] = variant as u16;
        }
        let mut screen = crate::gfx::Screen::new();
        world.render(&mut screen, &assets, &localization);
        for (text, y) in [
            ("16 WOOL", 8),
            ("19 FLOWERS", 42),
            ("16 BEDS", 86),
            ("16 DYES", 166),
        ] {
            screen.text(assets.font(), text, 8, y);
        }
        for color in DyeColor::ALL {
            screen.blit(
                assets.item(color.dye_item()),
                8 + i32::from(color as u8) * 16,
                178,
            );
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        screen.save_png(path)
    }
}
