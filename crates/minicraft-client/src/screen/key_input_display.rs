//! Java `KeyInputDisplay.java` — key binding list layout and duplicate detection.
use crate::gfx::font_style::TEXT_HEIGHT;
use crate::gfx::{HEIGHT, WIDTH};

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java: `Builder(false, 0, CENTER, KeyInputEntry…).setTitle(key_input)
/// .setPositioning(w/2, h - textHeight*5, TOP)`.
pub fn build_menu(title: &str, rows: &[(String, String, Vec<String>)], selection: usize) -> Menu {
    let entries: Vec<Entry> = rows
        .iter()
        .map(|(action, mapping, duplicated)| {
            let mut entry = Entry::key_input(action.clone(), mapping.clone());
            if let Entry::KeyInput {
                duplicated: slot, ..
            } = &mut entry
            {
                *slot = duplicated.clone();
            }
            entry
        })
        .collect();
    MenuBuilder::new(false, 0, RelPos::CENTER, entries)
        .set_title(title)
        .set_positioning(
            (WIDTH as i32 / 2, HEIGHT as i32 - TEXT_HEIGHT * 5),
            RelPos::TOP,
        )
        .set_selection(selection as i32)
        .create_menu()
}

/// Java KeyInputDisplay.getDuplicatedMappings: mappings that appear more than once.
/// `prefs` entries are `action;map|map|…`.
pub fn duplicated_mappings(prefs: &[String]) -> Vec<String> {
    use std::collections::HashSet;
    let mut existed = HashSet::new();
    let mut duplicated = HashSet::new();
    for pref in prefs {
        let mappings = pref
            .split_once(';')
            .map(|(_, rest)| rest)
            .unwrap_or(pref)
            .split('|');
        for mapping in mappings {
            if existed.contains(mapping) {
                duplicated.insert(mapping.to_owned());
            } else {
                existed.insert(mapping.to_owned());
            }
        }
    }
    let mut list: Vec<String> = duplicated.into_iter().collect();
    list.sort();
    list
}

/// Java KeyInputEntry.getWidth() is always Screen.w.
pub fn entry_width() -> i32 {
    WIDTH as i32
}

/// Help lines at `h - textHeight*(4-i)` for i=0..3.
pub fn help_line_y(index: usize) -> i32 {
    HEIGHT as i32 - TEXT_HEIGHT * (4 - index as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_input_menu_title_anchor_matches_java() {
        let rows = vec![
            ("UP".to_owned(), "UP".to_owned(), Vec::new()),
            ("DOWN".to_owned(), "DOWN".to_owned(), Vec::new()),
        ];
        let menu = build_menu("Key Bindings", &rows, 0);
        assert_eq!(menu.entries.len(), 2);
        assert_eq!(menu.title, "Key Bindings");
        // Java RelPos.TOP: menu bottom sits on the anchor (menu hangs above).
        assert_eq!(
            menu.bounds.1 + menu.bounds.3,
            HEIGHT as i32 - TEXT_HEIGHT * 5
        );
        assert_eq!(menu.entries[0].width(), entry_width());
    }

    #[test]
    fn duplicated_mappings_are_detected_like_java() {
        let prefs = vec![
            "up;UP".to_owned(),
            "down;DOWN".to_owned(),
            "attack;UP".to_owned(),
            "menu;ESCAPE|M".to_owned(),
            "exit;ESCAPE".to_owned(),
        ];
        let dups = duplicated_mappings(&prefs);
        assert_eq!(dups, vec!["ESCAPE".to_owned(), "UP".to_owned()]);
    }

    #[test]
    fn help_lines_stack_from_bottom() {
        assert_eq!(help_line_y(0), HEIGHT as i32 - TEXT_HEIGHT * 4);
        assert_eq!(help_line_y(3), HEIGHT as i32 - TEXT_HEIGHT);
    }
}
