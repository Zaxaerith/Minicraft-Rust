//! Java KnightStatue: three taps spawn ObsidianKnight; tryPush is a no-op.

/// Java KnightStatue.tapCount threshold.
pub const TAPS_TO_AWAKEN: u16 = 3;

/// Whether the statue can be pushed (Java KnightStatue.tryPush is empty).
pub const fn can_push() -> bool {
    false
}
