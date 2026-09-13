//! Session state surviving replacement of the compatibility World host.
use super::{
    clock::{Clock, SharedClock},
    updater::Updater,
};

/// Atomic compatibility handoff; all three values describe the same tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSnapshot {
    pub game_time: i32,
    pub calendar_tick: u32,
    pub paused: bool,
}

impl ClockSnapshot {
    pub const fn new(game_time: i32, calendar_tick: u32, paused: bool) -> Self {
        Self {
            game_time,
            calendar_tick,
            paused,
        }
    }
}

#[derive(Debug, Default)]
pub struct Game {
    ready: bool,
    clock: SharedClock,
    updater: Updater,
    /// Per-host mirror of Java Sound.tick() calls (race-free tests).
    sound_ticks: u64,
}

impl Game {
    pub fn observe_clock(&mut self, clock: ClockSnapshot) {
        self.clock = SharedClock::new(Clock {
            game_time: clock.game_time,
            day_tick: clock.calendar_tick,
        });
        self.updater = Updater::from_paused(clock.paused);
    }
    pub fn calendar_tick(&self) -> u32 {
        self.clock.get().day_tick
    }

    pub fn paused(&self) -> bool {
        self.updater.paused()
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn game_time(&self) -> i32 {
        self.clock.get().game_time
    }

    /// TitleDisplay.init clears readiness, not Updater.gameTime.
    pub fn enter_title(&mut self) {
        self.ready = false;
    }

    /// A fully initialized new or loaded world supplies its own time.
    pub fn enter_world(&mut self, clock: ClockSnapshot) {
        self.observe_clock(clock);
        self.ready = true;
    }

    /// Production world entry shares the world's allocation, not a snapshot.
    pub fn attach_world_clock(&mut self, clock: SharedClock, paused: bool) {
        self.clock = clock;
        self.observe_paused(paused);
        self.ready = true;
    }

    pub fn observe_paused(&mut self, paused: bool) {
        self.updater.set_paused(paused);
    }

    pub fn attach_world_runtime(&mut self, clock: SharedClock, updater: Updater) {
        self.clock = clock;
        self.updater = updater;
        self.ready = true;
    }

    pub fn shares_clock(&self, clock: &SharedClock) -> bool {
        self.clock.same_instance(clock)
    }

    /// World has its own compatibility Updater; only tick here outside World.
    pub fn tick_menu(&mut self, focused: bool) {
        // Java's calendar prelude uses the PREVIOUS paused value, even when
        // focus is lost. Entering Title alone does not overwrite that value.
        let plan = self.updater.plan_tick(focused, true);
        self.clock.advance_day(plan.advance_calendar);
        // Java Sound.tick() runs every Updater.tick, including menu-only ticks.
        crate::io::audio::tick_sound_pump();
        self.sound_ticks = self.sound_ticks.wrapping_add(1);
        self.clock.advance_focused(focused);
    }

    /// Menu-path Sound.tick() count (Java Updater cadence, including title).
    pub fn sound_ticks(&self) -> u64 {
        self.sound_ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_world_replaces_every_clock_field_before_readiness() {
        let mut game = Game::default();
        game.enter_world(ClockSnapshot::new(900, 64_799, false));
        game.enter_title();
        game.tick_menu(false);
        game.enter_world(ClockSnapshot::new(-123, 12, true));
        assert!(game.ready());
        assert_eq!(
            (game.game_time(), game.calendar_tick(), game.paused()),
            (-123, 12, true)
        );
        game.enter_title();
        game.tick_menu(false);
        assert_eq!((game.game_time(), game.calendar_tick()), (-123, 12));
        game.enter_world(ClockSnapshot::new(0, 0, true));
        assert_eq!(
            (game.game_time(), game.calendar_tick(), game.paused()),
            (0, 0, true)
        );
    }

    #[test]
    fn title_calendar_uses_previous_pause_through_focus_loss_and_rollover() {
        let mut game = Game::default();
        game.enter_world(ClockSnapshot::new(42, 64_799, false));
        game.enter_title();
        game.tick_menu(false);
        assert_eq!(
            (game.calendar_tick(), game.game_time(), game.paused()),
            (0, 42, false)
        );
        game.tick_menu(true);
        assert_eq!(
            (game.calendar_tick(), game.game_time(), game.paused()),
            (1, 43, true)
        );
        game.tick_menu(true);
        assert_eq!((game.calendar_tick(), game.game_time()), (1, 44));
    }

    #[test]
    fn title_keeps_time_and_loading_replaces_it() {
        let mut game = Game::default();
        assert!(!game.ready());
        game.tick_menu(true);
        game.tick_menu(false);
        assert_eq!(game.game_time(), 1);
        game.enter_world(ClockSnapshot::new(42, 0, true));
        assert!(game.ready());
        game.observe_clock(ClockSnapshot::new(43, 0, true));
        game.enter_title();
        assert!(!game.ready());
        game.tick_menu(true);
        assert_eq!(game.game_time(), 44);
        game.enter_world(ClockSnapshot::new(0, 0, true));
        assert_eq!(game.game_time(), 0);
        game.enter_world(ClockSnapshot::new(i32::MAX, 0, true));
        game.enter_title();
        game.tick_menu(true);
        assert_eq!(game.game_time(), i32::MIN);
    }

    #[test]
    fn menu_tick_pumps_sound_like_java_updater() {
        let mut game = Game::default();
        game.tick_menu(true);
        game.tick_menu(false);
        game.tick_menu(true);
        assert_eq!(game.sound_ticks(), 3);
    }
}
