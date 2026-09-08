//! The first migrated Updater phases: Display lifecycle, previous-paused time
//! gating, autosave and score countdown, then focus-gated input/gameplay dispatch.
//! Notification timer/GUI feedback lives in core::notifications. Full
//! player-before-menu updates remain separate migration work.

use super::display_stack::DisplayStack;
use crate::screen::display::Display;

pub const DAY_LENGTH: u32 = 64_800;

/// Normal Updater.setTime(tickCount + 1) rollover. Sleep/time-of-day jumps
/// are separate phases. Return whether compatibility day bookkeeping advances.
pub fn advance_calendar(tick: &mut u32, advance: bool) -> bool {
    if !advance {
        return false;
    }
    if *tick >= DAY_LENGTH - 1 {
        *tick = 0;
        true
    } else {
        *tick += 1;
        false
    }
}

/// Java's signed gameTime advances on every focused update, even in menus.
/// It is independent of the calendar and world simulation counters.
pub fn advance_game_time(game_time: &mut i32, focused: bool) {
    if focused {
        *game_time = game_time.wrapping_add(1);
    }
}

/// Compatibility hosts may still contain nested displays (World's PauseSession).
pub trait DisplayHost {
    fn update_child_displays(&mut self);
}

/// Run BEFORE checking focus, like Java Updater.tick. Both the top-level queue
/// and its active host's child queue cross the same logical update boundary.
pub fn begin_display_tick<D: Display + DisplayHost>(displays: &mut DisplayStack<D>) {
    begin_display_tick_observed(displays, |_, _| {});
}

pub fn begin_display_tick_observed<D: Display + DisplayHost>(
    displays: &mut DisplayStack<D>,
    observer: impl FnMut(super::display_stack::LifecycleEvent, &D),
) {
    displays.update_observed(observer);
    if let Some(display) = displays.current_mut() {
        display.update_child_displays();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickPlan {
    pub advance_calendar: bool,
    pub dispatch_input: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UpdaterState {
    paused: bool,
    autosave_ticks: u32,
}

/// Game and its World host share one runtime allocation; saves copy values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Updater(std::rc::Rc<std::cell::Cell<UpdaterState>>);

impl Default for Updater {
    fn default() -> Self {
        // Java Updater.paused starts true, before the first gameplay tick.
        Self::from_state(true, 0)
    }
}

impl Updater {
    pub fn from_paused(paused: bool) -> Self {
        Self::from_state(paused, 0)
    }
    pub fn from_state(paused: bool, autosave_ticks: u32) -> Self {
        Self(std::rc::Rc::new(std::cell::Cell::new(UpdaterState {
            paused,
            autosave_ticks,
        })))
    }

    pub fn restore(&self, paused: bool, autosave_ticks: u32) {
        self.0.set(UpdaterState {
            paused,
            autosave_ticks,
        });
    }

    pub fn set_paused(&self, paused: bool) {
        let mut state = self.0.get();
        state.paused = paused;
        self.0.set(state);
    }

    pub fn autosave_ticks(&self) -> u32 {
        self.0.get().autosave_ticks
    }

    pub fn reset_autosave(&mut self) {
        self.restore(self.paused(), 0);
    }

    /// Java Updater's autosave prelude. The interval expires even if saving is
    /// disabled, the player is dead, or game over. Run before calendar/score/input.
    pub fn advance_autosave(&mut self, enabled: bool, game_over: bool, alive: bool) -> bool {
        let mut state = self.0.get();
        if !state.paused {
            state.autosave_ticks = state.autosave_ticks.saturating_add(1);
        }
        self.0.set(state);
        if state.autosave_ticks > 7_200 {
            self.reset_autosave();
            enabled && !game_over && alive
        } else {
            false
        }
    }
    pub fn paused(&self) -> bool {
        self.0.get().paused
    }

    pub fn plan_tick(&mut self, focused: bool, has_display: bool) -> TickPlan {
        // Java advances tickCount using the PREVIOUS paused value. Losing
        // focus releases input but does not overwrite that paused value.
        let advance_calendar = !self.paused();
        if focused {
            self.set_paused(has_display);
        }
        TickPlan {
            advance_calendar,
            dispatch_input: focused,
        }
    }
}

/// Updater checks <= 0 BEFORE decrementing, including the terminal 0 -> -1 tick.
/// `advance_calendar` comes from the previous-paused gate, not current focus.
pub fn advance_score_time(
    advance_calendar: bool,
    score_mode: bool,
    game_over: bool,
    remaining: &mut i32,
) -> bool {
    if !advance_calendar || !score_mode || game_over {
        return false;
    }
    let ended = *remaining <= 0;
    *remaining = remaining.wrapping_sub(1);
    ended
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_boundary_and_pause_share_one_rollover_rule() {
        let mut tick = DAY_LENGTH - 1;
        assert!(!advance_calendar(&mut tick, false));
        assert_eq!(tick, DAY_LENGTH - 1);
        assert!(advance_calendar(&mut tick, true));
        assert_eq!(tick, 0);
        assert!(!advance_calendar(&mut tick, true));
        assert_eq!(tick, 1);
    }

    #[test]
    fn game_time_is_focus_gated_and_wraps_like_java_int() {
        let mut time = i32::MAX;
        advance_game_time(&mut time, false);
        assert_eq!(time, i32::MAX);
        advance_game_time(&mut time, true);
        assert_eq!(time, i32::MIN);
        advance_game_time(&mut time, true);
        assert_eq!(time, i32::MIN + 1);
    }

    #[test]
    fn autosave_expires_strictly_after_7200_and_resets_when_ineligible() {
        for (enabled, over, alive) in [
            (true, false, true),
            (false, false, true),
            (true, true, true),
            (true, false, false),
        ] {
            let mut updater = Updater::from_state(false, 7_199);
            assert!(!updater.advance_autosave(enabled, over, alive));
            assert_eq!(updater.autosave_ticks(), 7_200);
            assert_eq!(
                updater.advance_autosave(enabled, over, alive),
                enabled && !over && alive
            );
            assert_eq!(updater.autosave_ticks(), 0);
        }
    }

    #[test]
    fn autosave_uses_previous_pause_and_manual_save_resets_interval() {
        let mut updater = Updater::from_state(true, 7_200);
        assert!(!updater.advance_autosave(true, false, true));
        updater.plan_tick(true, false);
        assert!(updater.advance_autosave(true, false, true));
        updater.advance_autosave(true, false, true);
        updater.reset_autosave();
        assert_eq!(updater.autosave_ticks(), 0);
    }

    #[test]
    fn score_zero_is_observed_on_the_following_unpaused_tick() {
        let mut remaining = 1;
        assert!(!advance_score_time(true, true, false, &mut remaining));
        assert_eq!(remaining, 0);
        assert!(!advance_score_time(false, true, false, &mut remaining));
        assert!(advance_score_time(true, true, false, &mut remaining));
        assert_eq!(remaining, -1);
        assert!(!advance_score_time(true, true, true, &mut remaining));
        assert_eq!(remaining, -1);
        assert!(!advance_score_time(true, false, false, &mut remaining));
    }

    #[test]
    fn pause_and_resume_calendar_gate_uses_previous_tick() {
        let mut updater = Updater::default();
        assert!(!updater.plan_tick(true, false).advance_calendar);
        assert!(updater.plan_tick(true, true).advance_calendar);
        assert!(!updater.plan_tick(true, true).advance_calendar);
        assert!(!updater.plan_tick(true, false).advance_calendar);
        assert!(updater.plan_tick(true, false).advance_calendar);
    }

    #[test]
    fn focus_loss_does_not_change_the_previous_pause_state() {
        for paused in [false, true] {
            let mut updater = Updater::from_paused(paused);
            for display in [false, true, false] {
                let plan = updater.plan_tick(false, display);
                assert_eq!(plan.advance_calendar, !paused);
                assert!(!plan.dispatch_input);
                assert_eq!(updater.paused(), paused);
            }
        }
    }

    #[test]
    fn nested_lifecycle_runs_on_the_new_current_host_even_without_input() {
        #[derive(Default)]
        struct Host {
            boundaries: usize,
        }
        impl Display for Host {}
        impl DisplayHost for Host {
            fn update_child_displays(&mut self) {
                self.boundaries += 1;
            }
        }
        let mut stack = DisplayStack::default();
        stack.open(Host::default());
        begin_display_tick(&mut stack);
        stack.open(Host::default());
        begin_display_tick(&mut stack);
        assert_eq!(stack.current().unwrap().boundaries, 1);
        stack.exit(1);
        begin_display_tick(&mut stack);
        assert_eq!(stack.current().unwrap().boundaries, 2);
    }
}
