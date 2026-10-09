//! Java `WorldSelectDisplay.java` — world list menu and copy/rename/delete.
use crate::gfx::{HEIGHT, WIDTH};
use crate::resource::localization::Localization;
use crate::worlds::WorldRecord;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java: `Builder(false, 0, CENTER).setDisplayLength(5).setScrollPolicies(1, true)`.
pub const DISPLAY_LENGTH: i32 = 5;
pub const SCROLL_PADDING: f32 = 1.0;
pub const SCROLL_WRAP: bool = true;

/// Java WorldSelectDisplay empty-list placeholder string.
pub const NO_WORLDS_FOUND: &str = "NO WORLDS FOUND";

/// Java `updateEntries` menu from world names.
pub fn build_menu(worlds: &[WorldRecord], _localization: &Localization, selection: usize) -> Menu {
    let entries: Vec<Entry> = if worlds.is_empty() {
        vec![Entry::string(NO_WORLDS_FOUND)]
    } else {
        worlds
            .iter()
            .map(|world| Entry::select(world.name.clone(), false))
            .collect()
    };
    let mut builder = MenuBuilder::new(false, 0, RelPos::CENTER, entries)
        .set_display_length(DISPLAY_LENGTH)
        .set_scroll_policies(SCROLL_PADDING, SCROLL_WRAP)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 / 2), RelPos::CENTER);
    if !worlds.is_empty() {
        builder = builder.set_selection(selection.min(worlds.len().saturating_sub(1)) as i32);
    }
    builder.create_menu()
}

/// Java WorldSelectDisplay shift actions: C=copy, R=rename, D=delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldEdit {
    Copy,
    Rename,
    Delete,
}

/// Help line baselines from Java render: h-60, h-40, h-24, h-16, h-8.
pub const HELP_LINE_Y: [i32; 5] = [
    HEIGHT as i32 - 60,
    HEIGHT as i32 - 40,
    HEIGHT as i32 - 24,
    HEIGHT as i32 - 16,
    HEIGHT as i32 - 8,
];

/// Version warning / version label rows (Font.textHeight*5 and *7/2).
pub fn version_label_y() -> i32 {
    crate::gfx::font_style::TEXT_HEIGHT * 7 / 2
}

pub fn version_warning_y() -> i32 {
    crate::gfx::font_style::TEXT_HEIGHT * 5
}

/// After delete, Java clamps selection to the last remaining world.
pub fn clamp_selection_after_delete(selection: usize, remaining: usize) -> usize {
    if remaining == 0 {
        0
    } else {
        selection.min(remaining - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{GameMode, WorldSpec};

    fn worlds(names: &[&str]) -> Vec<WorldRecord> {
        names
            .iter()
            .map(|name| WorldRecord {
                name: (*name).to_owned(),
                seed: 1,
                spec: WorldSpec::default(),
                mode: GameMode::Survival,
                score_minutes: 20,
                directory: std::path::PathBuf::new(),
            })
            .collect()
    }

    #[test]
    fn worlds_menu_uses_display_length_five_and_wrap_scroll() {
        let localization = Localization::load("en-us", &[]);
        let list = worlds(&["A", "B", "C"]);
        let menu = build_menu(&list, &localization, 2);
        // Java clamps displayLength to entry count when wrap is on.
        assert_eq!(menu.display_length, 3);
        assert!(menu.wrap);
        // Java clamps displayLength before padding: floor(1.0 * 3 / 2) = 1.
        assert_eq!(menu.padding, (SCROLL_PADDING * 3_f32 / 2.0) as i32);
        assert_eq!(menu.selection, 2);
        assert_eq!(menu.bounds.0 + menu.bounds.2 / 2, WIDTH as i32 / 2);
    }

    #[test]
    fn long_world_list_keeps_display_length_five() {
        let localization = Localization::load("en-us", &[]);
        let names: Vec<String> = (0..12).map(|i| format!("W{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let list = worlds(&refs);
        let menu = build_menu(&list, &localization, 0);
        assert_eq!(menu.display_length, DISPLAY_LENGTH);
    }

    #[test]
    fn empty_world_list_shows_placeholder_and_is_not_selectable() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&[], &localization, 0);
        assert_eq!(menu.entries.len(), 1);
        assert_eq!(menu.entries[0].text(), NO_WORLDS_FOUND);
        assert!(!menu.selectable);
    }

    #[test]
    fn delete_clamps_selection_like_java() {
        assert_eq!(clamp_selection_after_delete(3, 3), 2);
        assert_eq!(clamp_selection_after_delete(0, 0), 0);
    }
}
