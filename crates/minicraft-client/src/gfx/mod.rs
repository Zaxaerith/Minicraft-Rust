//! The original 288×192 software canvas, sprites, bitmap font, and lighting.
//! Software drawing core; assets and window presentation stay in adapters.
mod font;
pub mod gui;
mod image;
mod screen;
pub use image::Image;
pub use screen::{HEIGHT, Screen, WIDTH};
