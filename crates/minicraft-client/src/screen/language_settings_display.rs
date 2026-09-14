//! Java `LanguageSettingsDisplay.java` — locale picker.
use crate::gfx::{HEIGHT, WIDTH};
use crate::resource::localization::{LOCALES, Localization};

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java: `Builder(false, 2, CENTER).setTitle(language_settings).setSelectable(true)
/// .setPositioning(w/2, 10, BOTTOM).setSize(w, h-30)`.
pub fn build_menu(localization: &Localization, selection: usize) -> Menu {
    let order = crate::resource::localization::sorted_locale_indices();
    let entries: Vec<Entry> = order
        .iter()
        .map(|index| Entry::select(LOCALES[*index].1, false))
        .collect();
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_title(localization.text("minicraft.displays.language_settings.title"))
        .set_positioning((WIDTH as i32 / 2, 10), RelPos::BOTTOM)
        .set_size(WIDTH as i32, HEIGHT as i32 - 30)
        .set_selection(selection as i32)
        .create_menu()
}

/// Sorted display labels (debug locale first, then name order).
pub fn locale_labels() -> Vec<&'static str> {
    let order = crate::resource::localization::sorted_locale_indices();
    order.iter().map(|index| LOCALES[*index].1).collect()
}

/// Locale id for a menu selection index.
pub fn locale_for_selection(selection: usize) -> Option<&'static str> {
    let order = crate::resource::localization::sorted_locale_indices();
    order
        .get(selection)
        .map(|index| LOCALES[*index].0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_menu_matches_java_bottom_anchor_and_full_width() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, 0);
        assert_eq!(menu.spacing, 2);
        assert_eq!(menu.title, localization.text("minicraft.displays.language_settings.title"));
        // setSize(w, h-30) with BOTTOM at y=10.
        assert_eq!(menu.bounds.2, WIDTH as i32);
        assert_eq!(menu.bounds.3, HEIGHT as i32 - 30);
        // RelPos.BOTTOM places the menu top-left on the anchor y.
        assert_eq!(menu.bounds.1, 10);
        assert_eq!(menu.bounds.1 + menu.bounds.3, 10 + (HEIGHT as i32 - 30));
        assert!(!menu.entries.is_empty());
        assert!(menu.selectable);
    }

    #[test]
    fn locale_selection_resolves_to_sorted_ids() {
        let labels = locale_labels();
        assert!(!labels.is_empty());
        assert!(locale_for_selection(0).is_some());
        assert!(locale_for_selection(labels.len()).is_none());
    }
}
