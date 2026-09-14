//! Java nested Play `Display` inside TitleDisplay (Load World / New World).
use crate::gfx::{HEIGHT, WIDTH};
use crate::resource::localization::Localization;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java TitleDisplay Play submenu: Load when worlds exist, else New only.
/// Spacing 2, CENTER at `(w/2, h*3/5)` — same anchor as the title list.
pub fn build_menu(localization: &Localization, has_worlds: bool, selection: usize) -> Menu {
    let entries = if has_worlds {
        vec![
            Entry::select(
                localization.text("minicraft.displays.title.play.load_world"),
                false,
            ),
            Entry::select(
                localization.text("minicraft.displays.title.play.new_world"),
                false,
            ),
        ]
    } else {
        vec![Entry::select(
            localization.text("minicraft.displays.title.play.new_world"),
            false,
        )]
    };
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java selection: 0 with two entries → WorldSelectDisplay; else WorldGenDisplay.
pub fn selection_is_load(selection: usize, has_worlds: bool) -> bool {
    has_worlds && selection == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_menu_with_worlds_has_load_then_new() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, true, 0);
        assert_eq!(menu.entries.len(), 2);
        assert_eq!(menu.selection, 0);
        assert_eq!(menu.spacing, 2);
        assert!(selection_is_load(0, true));
        assert!(!selection_is_load(1, true));
    }

    #[test]
    fn play_menu_without_worlds_is_new_only() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, false, 0);
        assert_eq!(menu.entries.len(), 1);
        assert!(!selection_is_load(0, false));
    }

    #[test]
    fn play_menu_uses_title_center_anchor() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, true, 0);
        assert_eq!(menu.bounds.0 + menu.bounds.2 / 2, WIDTH as i32 / 2);
        let center_y = menu.bounds.1 + menu.bounds.3 / 2;
        assert!((center_y - HEIGHT as i32 * 3 / 5).abs() <= 8);
    }
}
