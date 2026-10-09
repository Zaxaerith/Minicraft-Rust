//! Java `ControlsDisplay.java` — keyboard/controller help line list.
use crate::gfx::{HEIGHT, WIDTH};
use crate::resource::localization::Localization;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java initKeyControls / initControllerControls entry counts.
pub const KEYBOARD_LINES: usize = 23;
pub const CONTROLLER_LINES: usize = 16;

/// Java: `Builder(false, 0, CENTER).setSelectable(true)
/// .setPositioning(w/2, 20, BOTTOM).setDisplayLength(17)`.
pub const DISPLAY_LENGTH: i32 = 17;

/// Java ControlsDisplay help list.
pub fn build_menu(localization: &Localization, keyboard: bool, selection: usize) -> Menu {
    let prefix = if keyboard {
        "minicraft.displays.controls.display.keyboard"
    } else {
        "minicraft.displays.controls.display.controller"
    };
    let count = if keyboard {
        KEYBOARD_LINES
    } else {
        CONTROLLER_LINES
    };
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let key = format!("{prefix}.{i:02}");
        // Java StringEntry.useLines runs Font.getLines with Screen.w - 20
        // before adding rows. A long localized line may occupy several rows.
        for line in crate::gfx::font::get_lines(
            localization.text(&key),
            WIDTH as i32 - 20,
            HEIGHT as i32 * 2,
            0,
            false,
        ) {
            // Java StringEntry keeps WHITE for both selected and unselected rows.
            entries.push(Entry::select_colored(line, super::entry::COL_SLCT));
        }
    }
    MenuBuilder::new(false, 0, RelPos::CENTER, entries)
        .set_display_length(DISPLAY_LENGTH)
        .set_positioning((WIDTH as i32 / 2, 20), RelPos::BOTTOM)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java ControlsDisplay.tick: cursor-left/right toggles keyboard/controller.
pub fn toggle_device(keyboard: bool) -> bool {
    !keyboard
}

/// Title / mode label rows: y=0 and y=10 (WHITE).
pub const TITLE_Y: i32 = 0;
pub const MODE_Y: i32 = 10;

/// Keyboard desc at h-16; controller desc at h-24 and h-16; help at h-8.
pub fn keyboard_desc_y() -> i32 {
    HEIGHT as i32 - 16
}

pub fn controller_desc_y() -> [i32; 2] {
    [HEIGHT as i32 - 24, HEIGHT as i32 - 16]
}

pub fn help_y() -> i32 {
    HEIGHT as i32 - 8
}

/// Alternate key-binding capture list length used by the compatibility host.
pub const BINDING_ROWS: usize = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_controls_menu_matches_java_display_length_and_anchor() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, true, 0);
        assert!(menu.entries.len() > KEYBOARD_LINES);
        assert!(
            menu.entries
                .iter()
                .all(|entry| entry.width() <= WIDTH as i32 - 20)
        );
        assert_eq!(menu.display_length, DISPLAY_LENGTH);
        assert_eq!(menu.spacing, 0);
        assert!(menu.selectable);
        // Java RelPos.BOTTOM hangs the menu below the anchor (top-left on anchor).
        assert_eq!(menu.bounds.1, 20);
    }

    #[test]
    fn controller_controls_menu_has_sixteen_lines() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, false, 0);
        assert!(menu.entries.len() >= CONTROLLER_LINES);
        assert!(
            menu.entries
                .iter()
                .all(|entry| entry.width() <= WIDTH as i32 - 20)
        );
    }

    #[test]
    fn left_right_toggles_device() {
        assert!(!toggle_device(true));
        assert!(toggle_device(false));
    }
}
