//! Platform-independent input, settings, localization, files, and audio contracts.
pub mod audio;
pub mod input;
pub mod input_event;
pub mod key_names;
pub mod settings;

pub use input_event::Input;
pub use settings::{Config, KeyBindings, Settings};
