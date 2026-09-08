//! World adapter for the migrated Updater phases. Serialization stays here,
//! keeping client/core free of save-format or device dependencies.

use minicraft_client::core::updater::{Updater, advance_score_time};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{GameMode, World, WorldAction};
use crate::input::Input;

#[derive(Serialize, Deserialize)]
struct SavedUpdater {
    paused: bool,
    #[serde(default)]
    autosave_ticks: u32,
}

pub(super) fn serialize<S: Serializer>(
    updater: &Updater,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    SavedUpdater {
        paused: updater.paused(),
        autosave_ticks: updater.autosave_ticks(),
    }
    .serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Updater, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Saved {
        Legacy(bool),
        Current(SavedUpdater),
    }
    Ok(match Saved::deserialize(deserializer)? {
        Saved::Legacy(paused) => Updater::from_paused(paused),
        Saved::Current(state) => Updater::from_state(state.paused, state.autosave_ticks),
    })
}

impl World {
    /// Actual saves run at Java's prelude boundary, before calendar/score/input.
    /// Pure World tests use the same path with saving disabled and a no-op writer.
    pub(crate) fn tick_with_autosave(
        &mut self,
        focused: bool,
        input: &Input,
        enabled: bool,
        mut save: impl FnMut(&World) -> Result<(), String>,
    ) -> Result<WorldAction, String> {
        let previous_updater = (self.updater.paused(), self.updater.autosave_ticks());
        self.tick_sleep_prelude();
        if self
            .updater
            .advance_autosave(enabled, self.game_over, self.player.health > 0)
        {
            match save(self) {
                Ok(()) => self.record_save_success(),
                Err(error) => {
                    // Keep a failed write retryable; no later tick phase has run.
                    self.updater.restore(previous_updater.0, previous_updater.1);
                    return Err(error);
                }
            }
        }
        Ok(self.tick_after_prelude(focused, input))
    }

    pub(crate) fn reset_autosave(&mut self) {
        self.updater.reset_autosave();
    }

    pub(crate) fn begin_display_tick(&mut self) {
        self.pause.begin_tick();
        self.begin_end_game_tick();
    }

    pub(super) fn update_calendar_and_score(&mut self, focused: bool) -> bool {
        let has_display = self.pause.is_rendering()
            || self.inventory_open
            || self.sign_editor.is_some()
            || self.book_open.is_some()
            || self.game_over;
        let plan = self.updater.plan_tick(focused, has_display);
        if self.clock.advance_day(plan.advance_calendar) {
            self.days += 1;
        }
        if advance_score_time(
            plan.advance_calendar,
            self.mode == GameMode::Score,
            self.game_over,
            &mut self.score_ticks,
        ) {
            self.game_over = true;
            self.end_game = Some(Default::default());
            self.prepare_end_game();
        }
        self.notifications.tick(); // Java runs this before the focus/input branch.
        self.clock.advance_focused(focused);
        plan.dispatch_input
    }

    pub(crate) fn calendar_tick(&self) -> u64 {
        u64::from(self.clock.get().day_tick)
    }

    pub(crate) fn shared_clock(&self) -> minicraft_client::core::clock::SharedClock {
        self.clock.clone()
    }

    pub(crate) fn shared_updater(&self) -> Updater {
        self.updater.clone()
    }

    #[cfg(test)]
    pub(crate) fn clock_paused(&self) -> bool {
        self.updater.paused()
    }
}
