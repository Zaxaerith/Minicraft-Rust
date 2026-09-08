//! Game elapsed time and day position; simulation ticks are not this clock.
use super::updater::{advance_calendar, advance_game_time};
use std::{cell::Cell, rc::Rc};

/// Single-threaded runtime handle. Cloning shares one clock, never its state.
/// Save/load copies values explicitly and creates a fresh allocation on load.
#[derive(Debug, Default, Clone)]
pub struct SharedClock(Rc<Cell<Clock>>);

impl SharedClock {
    pub fn new(clock: Clock) -> Self {
        Self(Rc::new(Cell::new(clock)))
    }
    pub fn get(&self) -> Clock {
        self.0.get()
    }
    pub fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn set_game_time(&self, value: i32) {
        let mut clock = self.get();
        clock.game_time = value;
        self.0.set(clock);
    }
    pub fn set_day_tick(&self, value: u32) {
        let mut clock = self.get();
        clock.day_tick = value;
        self.0.set(clock);
    }
    pub fn advance_day(&self, enabled: bool) -> bool {
        let mut clock = self.get();
        let rollover = clock.advance_day(enabled);
        self.0.set(clock);
        rollover
    }
    pub fn advance_focused(&self, focused: bool) {
        let mut clock = self.get();
        clock.advance_focused(focused);
        self.0.set(clock);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub game_time: i32,
    pub day_tick: u32,
}

impl Clock {
    pub fn advance_day(&mut self, enabled: bool) -> bool {
        advance_calendar(&mut self.day_tick, enabled)
    }

    pub fn advance_focused(&mut self, focused: bool) {
        advance_game_time(&mut self.game_time, focused);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloned_handles_share_one_allocation_but_new_clocks_are_isolated() {
        let clock = SharedClock::new(Clock {
            game_time: i32::MAX,
            day_tick: 64_799,
        });
        let alias = clock.clone();
        assert!(clock.same_instance(&alias));
        alias.advance_focused(true);
        assert_eq!(clock.get().game_time, i32::MIN);
        assert!(clock.advance_day(true));
        assert_eq!(alias.get().day_tick, 0);
        let independent = SharedClock::new(clock.get());
        assert!(!clock.same_instance(&independent));
        independent.advance_focused(true);
        assert_eq!(clock.get().game_time, i32::MIN);
        drop(clock);
        alias.advance_focused(true);
        assert_eq!(alias.get().game_time, i32::MIN + 1);
    }
}
