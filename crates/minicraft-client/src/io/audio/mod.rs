//! Platform-independent clip decoding, event scheduling, and PCM mixing.
mod mixer;
pub mod sound;
pub use mixer::PcmMixer;
pub use sound::{SoundEffect, validate_embedded_sounds};
use std::sync::atomic::{AtomicU64, Ordering};

/// Java `Sound.tick()` cadence counter. Incremented once per Updater tick
/// (before the focus/input branch), matching Java's mixer pump call site.
static SOUND_TICK: AtomicU64 = AtomicU64::new(0);

/// Number of mixer-pump ticks observed since process start.
pub fn sound_tick() -> u64 {
    SOUND_TICK.load(Ordering::Relaxed)
}

/// Java `Sound.tick()`: pump the PCM mixer once for this Updater tick.
pub fn tick_sound_pump() {
    SOUND_TICK.fetch_add(1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_tick_increments_once_per_pump() {
        let before = sound_tick();
        tick_sound_pump();
        // Other threads may also pump; the contract is monotonic +1 from us.
        assert!(sound_tick() > before);
        tick_sound_pump();
        tick_sound_pump();
        assert!(sound_tick() >= before + 3);
    }
}
