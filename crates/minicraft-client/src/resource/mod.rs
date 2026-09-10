//! Local assets, resource packs, localization data, skins, and books.
pub mod localization;
pub mod pack;

pub use localization::{LOCALES, Localization};
pub use pack::{ResourcePack, discover};
