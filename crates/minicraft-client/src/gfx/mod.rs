//! The original 288×192 software canvas, sprites, bitmap font, and lighting.
//! Software drawing core; assets and window presentation stay in adapters.
pub mod font;
pub mod font_style;
pub mod gui;
mod image;
mod screen;
pub mod sprite;
pub mod sprite_linker;
pub use image::Image;
pub use screen::{HEIGHT, Screen, WIDTH};
