//! Local assets, resource packs, localization data, skins, and books.
pub mod localization;
pub mod pack;

pub use localization::{LOCALES, Localization, sorted_locale_indices};
pub use pack::{ResourcePack, discover};
