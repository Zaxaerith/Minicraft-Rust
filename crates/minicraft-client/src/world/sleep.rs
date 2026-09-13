//! Compatibility bed membership; entity removal/restoration remains separate.
use super::World;
use crate::core::sleep;

impl World {
    /// Run world entities without a target player. Full Java Level spawn and
    /// cross-level dispatch are still being migrated independently.
    /// Bed.use calls player.remove(); Java Player.tick returns on isRemoved,
    /// so no player timers run while sleeping.
    pub(super) fn tick_sleeping_level(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let level = &mut self.levels[self.current_level];
        super::tile_behavior::tick_random_tiles(level, self.width, self.height, &mut self.random);
        let outcome = self.tick_level_entities(None, false, false);
        for (x, y, radius, player_tnt) in outcome.explosions {
            self.apply_explosion(x, y, radius, player_tnt);
        }
        for mob in outcome.defeated_mobs {
            self.handle_defeated_mob(mob);
        }
        // Level.trySpawn follows entity updates/removals, even with no players.
        super::try_queue_natural_spawn_for_player(
            &mut self.levels[self.current_level],
            self.width,
            self.height,
            None,
            self.clock.get().day_tick,
            self.days,
            &mut self.random,
        );
        // Java still runs level.tick + Tile.tickCount++ while sleeping in bed
        // (no Display is open; only gamespeed changes).
        super::increment_tile_tick();
        self.tile_ticks = self.tile_ticks.wrapping_add(1);
    }

    pub(super) fn enter_bed(&mut self) {
        if self.sleeping > 0 {
            return;
        }
        // Bed.use stores the player's tile, not the furniture's tile.
        self.respawn_tile = Some((self.player.x >> 4, self.player.y >> 4));
        self.bed_level = Some(self.current_level);
        self.sleeping = 120; // Legacy save representation of membership.
        // Java Bed.use calls player.remove(); Player.tick returns on isRemoved.
        self.player_removed = true;
    }

    pub(super) fn respawn_position(&mut self) -> (i32, i32) {
        use super::Tile;
        let valid = self.respawn_tile.filter(|&(x, y)| {
            if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
                return false;
            }
            // Java Tile.maySpawn, plus host-only boss/ornate floors kept valid
            // for respawn in this port.
            let tile = self.levels[self.current_level].tiles[x as usize + y as usize * self.width];
            tile.may_spawn()
                || matches!(
                    tile,
                    Tile::BossFloor | Tile::OrnateStone | Tile::OrnateObsidian
                )
        });
        let position = valid.unwrap_or_else(|| {
            super::find_spawn(
                &self.levels[self.current_level].tiles,
                self.width,
                self.height,
            )
        });
        self.respawn_tile = Some(position);
        position
    }

    pub(super) fn tick_sleep_prelude(&mut self) {
        if self.sleeping == 0 {
            return;
        }
        self.sleep_speed = 20;
        let step = sleep::advance(self.clock.get().day_tick);
        self.clock.set_day_tick(step.day_tick);
        if step.passed_midnight {
            self.days = self.days.max(2);
        }
        if step.wake {
            // Compatibility ownership uses a level index instead of a Java
            // Bed reference. Do not teleport the player onto the furniture.
            if let Some(level) = self
                .bed_level
                .take()
                .filter(|&index| index < self.levels.len())
            {
                self.current_level = level;
            }
            self.sleeping = 0;
            self.sleep_speed = 1;
            // Java Bed.restorePlayers re-adds the player to the level entity list.
            self.player_removed = false;
        }
    }

    pub(crate) fn simulation_speed(&self) -> f64 {
        // Initializer ignores gamespeed while a Display is active.
        if self.sleeping > 0
            && !self.pause.is_rendering()
            && !self.inventory_open
            && self.sign_editor.is_none()
            && self.book_open.is_none()
            && !self.game_over
        {
            f64::from(self.sleep_speed.max(1))
        } else {
            1.0
        }
    }
}
