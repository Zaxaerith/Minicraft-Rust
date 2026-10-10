//! Java Player.tick gameplay body and item-use helpers (food, armor, potion, fishing).
use super::{
    ActiveItem, Direction, FurnitureKind, GameMode, HUNGER_MOVE_STEPS, HUNGER_STAMINA_STEPS,
    HUNGER_TICK_INTERVALS, ItemId, MAX_HEALTH, MAX_HUNGER_TICKS, MAX_STAT, PotionKind,
    STARVATION_HEALTH_FLOORS, SoundEffect, TILE_SIZE, Tile, World, direction_for_move, find_spawn,
    item_for_furniture, spawn,
};
use std::collections::HashMap;

use crate::io::Input;
use crate::item::{
    ItemStack, RecipeOutput, ToolKind, ToolTier, ordered_recipe_views, recipe_can_craft,
    recipe_views,
};
use crate::progression::{Book, ProgressEvent};

impl World {
    pub(super) fn tick_gameplay_player(&mut self, input: &Input) {
        self.player.hurt_time = self.player.hurt_time.saturating_sub(1);
        self.player.attack_time = self.player.attack_time.saturating_sub(1);
        if self.player.attack_time == 0 {
            self.player.attack_item = None;
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

        // Java Player.tick returns immediately when removed.
        if !self.player_removed {
            // Stairs / infinite fall run before movement (Player.java after potions).
            self.tick_stair_and_fall();
            if self.pending_level_change != 0 {
                return;
            }

            let horizontal = i32::from(input.right) - i32::from(input.left);
            let vertical = i32::from(input.down) - i32::from(input.up);

            if horizontal != 0 || vertical != 0 {
                let swimming = self.player_is_swimming();
                let has_swim = self.effect_active(PotionKind::Swim);
                // Player.tick only calls Mob.move on even recovery-delay ticks (or
                // while swimming). Mob.move then skips even swimming ticks before
                // changing direction/walkDist, but reports movement to stepCount.
                if self.player.stamina_recharge_delay.is_multiple_of(2) || swimming {
                    let moved = if swimming && !has_swim && self.tick.is_multiple_of(2) {
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
            // Java Player.tick: drowning drains stamina every 60 ticks while swimming.
            if self.player_is_swimming()
                && self.tick.is_multiple_of(60)
                && !self.effect_active(PotionKind::Swim)
            {
                if self.player.stamina > 0 {
                    self.pay_stamina(1);
                } else if self.player.hurt_time == 0 {
                    // directHurt bypasses armor; still respects hurt cooldown.
                    self.hurt_player(1, true);
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
            if input.attack {
                if self.player.stamina > 0
                    || self.effect_active(PotionKind::Energy)
                    || self.mode == GameMode::Creative
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
            } else if input.select {
                // Java world maps SELECT and ATTACK to the same pad button (A);
                // only run the non-attack interact path when attack did not fire.
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
                self.die_player();
            }
        }
    }

    pub(super) fn crafting_recipe_count(&self) -> usize {
        recipe_views(self.crafting_station)
            .iter()
            .filter(|recipe| self.progress.recipe_is_unlocked(recipe))
            .count()
    }

    pub(super) fn refresh_crafting_order(&mut self) {
        self.update_progress(ProgressEvent::InventoryChanged);
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

    pub(super) fn craft_selected_recipe(&mut self) {
        let recipes = ordered_recipe_views(
            self.crafting_station,
            &self.player.inventory,
            self.mode == GameMode::Creative,
            &self.crafting_order,
            &self.progress,
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

    pub(super) fn craft_stack(&mut self, recipe: crate::item::Recipe) -> bool {
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

    pub(super) fn craft_tool(&mut self, recipe: crate::item::ToolRecipe) -> bool {
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

    pub(super) fn use_active_self_item(&mut self) -> bool {
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
                self.player.imported_shirt_color = None;
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

    pub(super) fn tick_book(&mut self, input: &Input) -> bool {
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

    pub(super) fn tick_sign_editor(&mut self, input: &Input) -> bool {
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

    pub(super) fn add_score(&mut self, points: u32, multiplier_gain: u8) {
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

    pub(super) fn unlock_achievement(&mut self, id: &str) {
        if self.progress.unlock_achievement(id) {
            let label = self.notification_context.locale.text(id).to_owned();
            self.add_notification_args("minicraft.notification.achievement_unlocked", &[&label]);
        }
    }

    pub(super) fn crafting_achievement(&mut self, item: ItemId) {
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

    pub(super) fn inventory_progress(&self) -> HashMap<String, u16> {
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

    pub(super) fn update_progress(&mut self, event: ProgressEvent) {
        // Java recipe advancements run even when tutorials/quests are disabled.
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

    pub(super) fn grant_reward(&mut self, reward: &str) {
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

    pub(super) fn pickup_target(&mut self) {
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

    pub(super) fn consume_active_stack(&mut self, item: ItemId) {
        if self.mode == GameMode::Creative {
            return;
        }
        let removed = self.player.inventory.remove(item, 1);
        debug_assert!(removed);
        if self.player.inventory.count(item) == 0 {
            self.player.active_item = None;
        }
    }

    pub(super) fn pay_stamina(&mut self, cost: u8) -> bool {
        if self.mode == GameMode::Creative || self.effect_active(PotionKind::Energy) {
            return true;
        }
        if self.player.stamina == 0 {
            return false;
        }
        self.player.stamina = self.player.stamina.saturating_sub(cost);
        true
    }

    pub(super) fn apply_potion(&mut self, potion: PotionKind) -> bool {
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

    pub(super) fn effect_active(&self, potion: PotionKind) -> bool {
        self.player.potion_effects[potion.id()] > 0
    }

    pub(super) fn tick_potion_effects(&mut self) {
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

    pub(super) fn tick_fishing(&mut self) {
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

    pub(super) fn hurt_player(&mut self, mut damage: u8, direct: bool) -> bool {
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

    pub(super) fn eat_active_food(&mut self) -> bool {
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

    pub(super) fn tick_survival_stats(&mut self) {
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
            // Java: do not recharge stamina while swimming (unless Swim potion).
            if self.player_is_swimming() && !self.effect_active(PotionKind::Swim) {
                self.player.stamina_recharge = 0;
            }
            while self.player.stamina_recharge > 10 {
                self.player.stamina_recharge -= 10;
                self.player.stamina = self.player.stamina.saturating_add(1).min(MAX_STAT);
            }
        }

        let difficulty = self.difficulty.min(2);
        if self.player.stamina < MAX_STAT {
            self.player.hunger_ticks = self.player.hunger_ticks.saturating_sub(difficulty as i16);
            if self.player.stamina == 0 {
                self.player.hunger_ticks =
                    self.player.hunger_ticks.saturating_sub(difficulty as i16);
            }
        }
        if self.tick.is_multiple_of(HUNGER_TICK_INTERVALS[difficulty]) {
            self.player.hunger_ticks = self.player.hunger_ticks.saturating_sub(1);
        }
        if self.player.step_count >= HUNGER_MOVE_STEPS[difficulty] {
            self.player.hunger_ticks = self.player.hunger_ticks.saturating_sub(1);
            self.player.step_count = 0;
        }
        if self.player.hunger_charge_delay > 0 {
            self.player.hunger_ticks = self
                .player
                .hunger_ticks
                .saturating_sub((2 + difficulty) as i16);
            if self.player.hunger == 0 {
                self.player.hunger_ticks =
                    self.player.hunger_ticks.saturating_sub(difficulty as i16);
            }
        }
        while self.player.hunger_ticks <= 0 {
            self.player.hunger_ticks += MAX_HUNGER_TICKS;
            self.player.hunger_stamina_count = self.player.hunger_stamina_count.saturating_sub(1);
        }
        while self.player.hunger_stamina_count <= 0 {
            self.player.hunger = self.player.hunger.saturating_sub(1);
            self.player.hunger_stamina_count += HUNGER_STAMINA_STEPS[difficulty];
        }

        if self.player.health < self.player.max_health && self.player.hunger > MAX_STAT / 2 {
            self.player.hunger_charge_delay = self.player.hunger_charge_delay.saturating_add(1);
            let distance = u16::from(MAX_STAT.saturating_sub(self.player.hunger) + 2);
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
}
