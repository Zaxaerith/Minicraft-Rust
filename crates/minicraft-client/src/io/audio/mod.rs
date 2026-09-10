//! Platform-independent clip decoding, event scheduling, and PCM mixing.
mod mixer;
pub mod sound;
pub use mixer::PcmMixer;
pub use sound::{SoundEffect, validate_embedded_sounds};
