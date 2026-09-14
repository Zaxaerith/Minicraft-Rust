//! Java-equivalent Display, Menu, entry, and concrete screen ownership.
//!
//! Module names follow Java `minicraft.screen.*`:
//! - `menu` → `Menu.java`
//! - `display` / `display_host` → `Display.java`
//! - `entry` → `entry/*`
//! - `rel_pos` → `RelPos.java`
//! - concrete Display subclasses (title, world_gen, …) match their Java files.

pub mod achievements_display;
pub mod book;
pub mod controls_display;
pub mod display;
pub mod display_host;
pub mod end_game;
pub mod entry;
pub mod help_display;
pub mod info_display;
pub mod key_input_display;
pub mod language_settings_display;
pub mod menu;
pub mod options_main_menu_display;
pub mod pause;
pub mod play_display;
pub mod rel_pos;
pub mod resource_pack_display;
pub mod skin_display;
pub mod title_display;
pub mod world_gen_display;
pub mod world_select_display;
