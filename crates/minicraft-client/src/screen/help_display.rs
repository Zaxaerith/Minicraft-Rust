//! Java TitleDisplay Help submenu (BlankEntry + 4 SelectEntry books).
use crate::gfx::{HEIGHT, WIDTH};
use crate::progression::Book;
use crate::resource::localization::Localization;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java help book keys, in Book::ALL order.
pub const HELP_KEYS: [&str; 4] = [
    "minicraft.displays.title.help.instructions",
    "minicraft.displays.title.help.storyline_guide",
    "minicraft.displays.title.help.about",
    "minicraft.displays.title.help.credits",
];

/// Java: `Menu.Builder(false, 1, CENTER, BlankEntry, …).setTitle(help)`.
pub fn build_menu(localization: &Localization, selection: usize) -> Menu {
    let mut entries = vec![Entry::blank()];
    entries.extend(
        HELP_KEYS
            .iter()
            .map(|key| Entry::select(localization.text(key), false)),
    );
    MenuBuilder::new(false, 1, RelPos::CENTER, entries)
        .set_title(localization.text("minicraft.displays.title.help"))
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 / 2), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Menu selection → book. Blank row is non-selectable so selection is 1..=4.
pub fn book_for_selection(selection: usize) -> Option<Book> {
    // Selection is the full menu index; blank row (0) has no book.
    if selection == 0 {
        return None;
    }
    let action = selection - 1;
    Book::ALL.get(action).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_menu_has_blank_plus_four_books_and_title() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, 1);
        assert_eq!(menu.entries.len(), 5);
        assert!(!menu.entries[0].is_selectable());
        assert!(menu.entries[1].is_selectable());
        assert_eq!(menu.selection, 1);
        assert_eq!(menu.spacing, 1);
        assert!(!menu.title.is_empty());
    }

    #[test]
    fn help_selection_maps_to_books_in_order() {
        use std::mem::discriminant;
        assert!(book_for_selection(0).is_none());
        assert_eq!(
            book_for_selection(1).map(|b| discriminant(&b)),
            Some(discriminant(&Book::ALL[0]))
        );
        assert_eq!(
            book_for_selection(4).map(|b| discriminant(&b)),
            Some(discriminant(&Book::ALL[3]))
        );
        assert!(book_for_selection(5).is_none());
    }
}
