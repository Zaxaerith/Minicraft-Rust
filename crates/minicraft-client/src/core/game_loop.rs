//! Fixed-speed timing extracted from Java 2.2.4 `Initializer.run()`.
//!
//! Rendering cadence never changes the normal 60 TPS simulation clock. Preserve
//! Java's double arithmetic and subtraction order, including uncapped catch-up.
//! The host supplies gamespeed (one while a Display is active).

use std::time::{Duration, Instant};

pub const NORMAL_TPS: u32 = 60;
const NANOS_PER_TICK: f64 = 1E9_f64 / NORMAL_TPS as f64;

#[derive(Debug, Default)]
pub struct FixedTickAccumulator {
    unprocessed: f64,
}

impl FixedTickAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self, elapsed: Duration) -> usize {
        self.advance_scaled(elapsed, 1.0)
    }

    pub fn advance_scaled(&mut self, elapsed: Duration, speed: f64) -> usize {
        // Do not replace this with integer rounding or floor/remainder: those
        // alter Java's boundary behavior after fractional frame intervals.
        self.unprocessed += elapsed.as_nanos() as f64 / (NANOS_PER_TICK / speed);
        let mut updates = 0;
        while self.unprocessed >= 1.0 {
            updates += 1;
            self.unprocessed -= 1.0;
        }
        updates
    }
}

/// Monotonic wall-clock adapter used by the live application.
/// Explicit timestamps let tests replay the same path without sleeping.
pub struct GameLoopClock {
    last: Instant,
    accumulator: FixedTickAccumulator,
}

impl Default for GameLoopClock {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLoopClock {
    pub fn new() -> Self {
        Self::starting_at(Instant::now())
    }

    pub fn starting_at(now: Instant) -> Self {
        Self {
            last: now,
            accumulator: FixedTickAccumulator::new(),
        }
    }

    pub fn updates(&mut self) -> usize {
        self.updates_at(Instant::now())
    }

    pub fn updates_scaled(&mut self, speed: f64) -> usize {
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(self.last);
        self.last = now;
        self.accumulator.advance_scaled(elapsed, speed)
    }

    pub fn updates_at(&mut self, now: Instant) -> usize {
        let elapsed = now.saturating_duration_since(self.last);
        self.last = now;
        self.accumulator.advance(elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Moved from app.rs with the production implementation.
    #[test]
    fn render_fps_does_not_change_the_fixed_sixty_tps_clock() {
        let mut accumulator = FixedTickAccumulator::new();
        let updates = (0..120)
            .map(|frame| {
                let nanos = 8_333_333 + u64::from(frame < 40);
                accumulator.advance(Duration::from_nanos(nanos))
            })
            .sum::<usize>();
        assert_eq!(updates, 60);
        assert_eq!(
            FixedTickAccumulator::new().advance(Duration::from_millis(100)),
            6
        );
    }

    #[test]
    fn starts_without_a_preloaded_tick_and_zero_elapsed_emits_nothing() {
        let start = Instant::now();
        let mut clock = GameLoopClock::starting_at(start);
        assert_eq!(clock.updates_at(start), 0);
        assert_eq!(clock.updates_at(start), 0);
    }

    #[test]
    fn fractional_time_survives_frames_that_do_not_emit_updates() {
        let start = Instant::now();
        let mut clock = GameLoopClock::starting_at(start);
        assert_eq!(clock.updates_at(start + Duration::from_nanos(8_333_333)), 0);
        assert_eq!(clock.updates_at(start + Duration::from_nanos(8_333_333)), 0);
        assert_eq!(
            clock.updates_at(start + Duration::from_nanos(16_666_667)),
            1
        );
    }

    #[test]
    fn wall_clock_and_accumulator_agree_at_all_required_render_rates() {
        for fps in [30_u64, 60, 120, 300] {
            let start = Instant::now();
            let mut clock = GameLoopClock::starting_at(start);
            let mut accumulator = FixedTickAccumulator::new();
            let mut last = Duration::ZERO;
            let mut total = 0;
            for frame in 1..=fps * 10 {
                let elapsed = Duration::from_nanos(frame * 1_000_000_000 / fps);
                let updates = clock.updates_at(start + elapsed);
                assert_eq!(
                    updates,
                    accumulator.advance(elapsed - last),
                    "fps={fps}, frame={frame}"
                );
                last = elapsed;
                total += updates;
            }
            assert_eq!(total, 600, "fps={fps}");
        }
    }

    #[test]
    fn catch_up_is_not_capped_and_restarting_discards_old_time() {
        let start = Instant::now();
        let mut clock = GameLoopClock::starting_at(start);
        assert_eq!(clock.updates_at(start + Duration::from_secs(2)), 120);
        assert_eq!(clock.updates_at(start + Duration::from_secs(2)), 0);
        clock = GameLoopClock::starting_at(start + Duration::from_secs(300));
        assert_eq!(clock.updates_at(start + Duration::from_secs(300)), 0);
        assert_eq!(clock.updates_at(start + Duration::from_millis(300_100)), 6);
    }
}
