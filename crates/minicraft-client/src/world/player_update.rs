//! Player entry controls that must run before compatibility menu dispatch.
//! Display-requested gating mirrors Java Player.tick's early return: with a
//! requested Display (pause/inventory/book/sign/death) neither HUD toggles nor
//! survival timers run; see core/player_update.rs and tick_after_prelude.
use super::World;
use crate::io::Input;

impl World {
    pub(super) fn tick_player_display_controls(&mut self, input: &Input) {
        let requested = self.pause.is_open()
            || self.inventory_open
            || self.sign_editor.is_some()
            || self.book_open.is_some()
            || self.game_over;
        // Bed membership temporarily models removal; general removed/entity
        // lifecycle remains pending. Do not substitute health==0 for isRemoved.
        if !crate::core::player_update::can_tick(
            self.current_level < self.levels.len(),
            self.sleeping > 0,
            requested,
        ) {
            return;
        }
        if input.toggle_hud {
            self.render_gui = !self.render_gui;
        }
    }
}
