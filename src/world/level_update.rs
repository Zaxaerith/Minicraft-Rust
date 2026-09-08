//! Shared Level dispatch boundary for awake and sleeping compatibility hosts.
//! Species AI remains in entity; this adapter owns pending-add ordering only.
use super::{World, entity::TickOutcome};

impl World {
    pub(super) fn tick_level_entities(
        &mut self,
        player: Option<(i32, i32)>,
        time_slowed: bool,
        player_passive: bool,
    ) -> TickOutcome {
        let level = &mut self.levels[self.current_level];
        for intent in std::mem::take(&mut level.pending_spawns) {
            level.entities.spawn_mob(intent.kind, intent.x, intent.y);
        }
        level.entities.tick_with_player(
            &mut level.tiles,
            &level.data,
            self.width,
            self.height,
            player,
            time_slowed,
            player_passive,
            &mut self.random,
        )
    }
}
