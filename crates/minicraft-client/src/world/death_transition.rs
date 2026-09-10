//! Java player-death delay menu and LevelTransitionDisplay timing.
use super::{ActiveItem, Assets, GameMode, Localization, World, WorldAction};
use crate::io::Input;
use serde::{Deserialize, Serialize};

/// Java LevelTransitionDisplay DURATION.
pub const LEVEL_TRANSITION_TICKS: u32 = 30;
/// Java Updater opens PlayerDeathDisplay after playerDeadTime > 60.
pub const DEATH_DELAY_TICKS: u32 = 60;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LevelTransition {
    /// Java visual direction: positive squares rise, negative fall.
    pub dir: i8,
    pub time: u32,
}

impl World {
    /// Java World.scheduleLevelChange: store pending direction only.
    /// Rust levels are ordered sky→surface→caves, so the index delta is the
    /// opposite of Java's Stairs Up (+1 toward sky index).
    pub(super) fn schedule_level_change(&mut self, visual_dir: i8) {
        if self.pending_level_change != 0 || self.level_transition.is_some() {
            return;
        }
        self.pending_level_change = visual_dir.saturating_neg();
    }

    /// Java World.changeLevel: wrap indices, recenter on the stairs tile.
    pub(super) fn apply_level_change(&mut self, rust_delta: i8) {
        let len = self.levels.len() as i32;
        let mut next = self.current_level as i32 + i32::from(rust_delta);
        if next < 0 {
            next = len - 1;
        }
        if next >= len {
            next = 0;
        }
        self.current_level = next as usize;
        self.player.x = (self.player.x >> 4) * 16 + 8;
        self.player.y = (self.player.y >> 4) * 16 + 8;
        let depth = self.levels[self.current_level].depth;
        if depth == -3 {
            self.unlock_achievement("minicraft.achievement.lowest_caves");
        }
        if depth == -4 {
            self.unlock_achievement("minicraft.achievement.obsidian_dungeon");
        }
    }

    /// Java Player.die: score penalty, DeathChest with gear, death sound, remove.
    /// Score subtraction is unconditional in Java (not Score-mode only).
    pub(super) fn die_player(&mut self) {
        self.score -= self.score / 3;
        self.score_multiplier = 1;
        self.multiplier_ticks = 300;
        let drop_x = self.player.x;
        let drop_y = self.player.y;
        let stacks = self.player.inventory.take_all();
        let tools = self.player.inventory.tools().to_vec();
        self.player
            .inventory
            .set_capacity(self.player.inventory.capacity());
        // Java DeathChest copies inventory + activeItem + armor; tools stay in
        // the chest storage as well so death does not delete them.
        let mut chest_stacks = stacks;
        if let Some(ActiveItem::Stack(item)) = self.player.active_item.take() {
            chest_stacks.push(crate::item::ItemStack::new(item, 1));
        }
        if let Some(armor) = self.player.armor_kind.take() {
            chest_stacks.push(crate::item::ItemStack::new(armor.item(), 1));
        }
        self.levels[self.current_level].entities.spawn_death_chest(
            drop_x,
            drop_y,
            chest_stacks,
            tools,
        );
        self.sound(super::SoundEffect::Death);
        self.player_removed = true;
        self.player_dead_time = 0;
        self.death_open = false;
        self.death_selection = 0;
    }

    /// Returns `Some` when a terminal menu action fires or a Display-like
    /// state (death menu / level transition) owns the tick (skip player+level).
    /// Returns `None` during the death delay so player+level still run, matching
    /// Java Updater which only queues the death Display and keeps ticking.
    pub(super) fn tick_death_and_transition(&mut self, input: &Input) -> Option<WorldAction> {
        if let Some(transition) = self.level_transition {
            return Some(self.tick_level_transition(transition));
        }
        if self.pending_level_change != 0 && !self.player_removed {
            let delta = self.pending_level_change;
            self.pending_level_change = 0;
            let mut transition = LevelTransition {
                dir: delta.saturating_neg(),
                time: 0,
            };
            // Java queues LevelTransitionDisplay then still runs player+level
            // this tick; the Display becomes current on the next tick.
            transition.time = 1;
            if transition.time == LEVEL_TRANSITION_TICKS / 2 {
                self.apply_level_change(-transition.dir);
            }
            self.level_transition = Some(transition);
            return None;
        }
        if self.death_open {
            return Some(self.tick_death_menu(input));
        }
        if self.player_removed && self.sleeping == 0 {
            self.player_dead_time = self.player_dead_time.saturating_add(1);
            if self.player_dead_time > DEATH_DELAY_TICKS {
                self.death_open = true;
                self.death_selection = 0;
            }
            // Delay ticks and the open tick still run player (isRemoved) + level.
            return None;
        }
        None
    }

    fn tick_level_transition(&mut self, mut transition: LevelTransition) -> WorldAction {
        transition.time += 1;
        if transition.time == LEVEL_TRANSITION_TICKS / 2 {
            let delta = -transition.dir;
            self.apply_level_change(delta);
        }
        if transition.time >= LEVEL_TRANSITION_TICKS {
            self.level_transition = None;
        } else {
            self.level_transition = Some(transition);
        }
        WorldAction::None
    }

    fn death_option_count(&self) -> usize {
        if self.mode == GameMode::Hardcore {
            1
        } else {
            3
        }
    }

    fn tick_death_menu(&mut self, input: &Input) -> WorldAction {
        let count = self.death_option_count();
        self.death_selection = self.death_selection.min(count - 1);
        if input.up_pressed {
            self.death_selection = self.death_selection.checked_sub(1).unwrap_or(count - 1);
            self.sound(super::SoundEffect::Select);
        }
        if input.down_pressed {
            self.death_selection = (self.death_selection + 1) % count;
            self.sound(super::SoundEffect::Select);
        }
        if !input.select {
            return WorldAction::None;
        }
        self.sound(super::SoundEffect::Confirm);
        if self.mode == GameMode::Hardcore {
            // Only Save and Quit.
            self.death_open = false;
            return WorldAction::ReturnToTitle;
        }
        match self.death_selection {
            0 => {
                self.death_open = false;
                self.player_removed = false;
                self.player_dead_time = 0;
                self.respawn();
                WorldAction::None
            }
            1 => {
                self.death_open = false;
                WorldAction::ReturnToTitle
            }
            _ => {
                self.death_open = false;
                WorldAction::QuitWithoutSaving
            }
        }
    }

    /// Stairs / infinite fall cooldown matching Player.onStairDelay/onFallDelay.
    pub(super) fn tick_stair_and_fall(&mut self) {
        let tile = self.tile_at_pixel(self.player.x, self.player.y);
        if matches!(tile, super::Tile::StairsUp | super::Tile::StairsDown) {
            if self.on_stair_delay == 0 {
                let visual = if tile == super::Tile::StairsUp { 1 } else { -1 };
                self.schedule_level_change(visual);
                self.on_stair_delay = 10;
                return;
            }
            self.on_stair_delay = 10;
        } else if self.on_stair_delay > 0 {
            self.on_stair_delay -= 1;
        }
        if tile == super::Tile::InfiniteFall && self.mode != GameMode::Creative {
            if self.on_fall_delay == 0 {
                self.schedule_level_change(-1);
                self.on_fall_delay = 40;
                return;
            }
        } else if self.on_fall_delay > 0 {
            self.on_fall_delay -= 1;
        }
    }

    pub(super) fn render_level_transition(&self, screen: &mut crate::gfx::Screen, assets: &Assets) {
        let Some(transition) = self.level_transition else {
            return;
        };
        // Java LevelTransitionDisplay: 200×150 cells of 8px, dd pattern.
        for x in 0..200 {
            for y in 0..150 {
                let dd = (y + x % 2 * 2 + x / 3) as i32 - (transition.time as i32) * 2;
                if (dd < 0) && (dd > -30) {
                    let draw_x = x * 8;
                    let draw_y = if transition.dir > 0 {
                        y * 8
                    } else {
                        crate::gfx::HEIGHT as i32 - y * 8 - 8
                    };
                    if (0..crate::gfx::WIDTH as i32).contains(&draw_x)
                        && (0..crate::gfx::HEIGHT as i32).contains(&draw_y)
                    {
                        screen.blit_region(assets.hud(), draw_x, draw_y, 40, 16, 8, 8, false);
                    }
                }
            }
        }
    }

    pub(super) fn render_death_menu(
        &self,
        screen: &mut crate::gfx::Screen,
        assets: &Assets,
        localization: &Localization,
    ) {
        use crate::gfx::gui::render_menu_frame;
        let width = 160;
        let height = 88;
        let x = 8;
        let y = 24;
        render_menu_frame(screen, assets.hud(), x, y, width, height);
        let title = localization.text("minicraft.displays.player_death.title");
        screen.centered_text(assets.font(), title, y + 8);
        let score = localization.format(
            "minicraft.displays.player_death.display.score",
            &[&self.score.to_string()],
        );
        screen.text(assets.font(), &score, x + 12, y + 24);
        let mut row = y + 40;
        if self.mode != GameMode::Hardcore {
            let selected = self.death_selection == 0;
            let label = localization.text("minicraft.displays.player_death.respawn");
            let mark = if selected { "> " } else { "  " };
            screen.text(assets.font(), &format!("{mark}{label}"), x + 12, row);
            row += 10;
        }
        let save_quit = self.death_selection == usize::from(self.mode != GameMode::Hardcore);
        let label = localization.text("minicraft.displays.player_death.save_quit");
        let mark = if save_quit { "> " } else { "  " };
        screen.text(assets.font(), &format!("{mark}{label}"), x + 12, row);
        if self.mode != GameMode::Hardcore {
            row += 10;
            let selected = self.death_selection == 2;
            let label = localization.text("minicraft.displays.player_death.quit");
            let mark = if selected { "> " } else { "  " };
            screen.text(assets.font(), &format!("{mark}{label}"), x + 12, row);
        }
    }
}
