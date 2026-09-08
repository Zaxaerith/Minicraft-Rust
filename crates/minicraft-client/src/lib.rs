//! Staged client architecture for the Minicraft+ 2.2.4 Java-parity port.
//!
//! The root `minicraft-rust` package remains the runnable compatibility client
//! while systems are migrated behind tests into this crate.

pub use minicraft_common as common;

pub mod core;
pub mod entity;
pub mod gfx;
pub mod io;
pub mod item;
pub mod level;
pub mod network;
pub mod platform;
pub mod progression;
pub mod resource;
pub mod saveload;
pub mod screen;
