//! Java `BookDisplay.java` — page geometry, pagination, and page menu layout.
use crate::gfx::{HEIGHT, WIDTH};
use crate::gfx::font;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java BookDisplay text area: minX=15, maxX=15+8*32, minY=8*5, maxY=minY+(8<<4).
pub const MIN_X: i32 = 15;
pub const MAX_X: i32 = 15 + 8 * 32;
pub const MIN_Y: i32 = 8 * 5;
pub const MAX_Y: i32 = 8 * 5 + (8 << 4);

/// Derived page content size (maxX-minX, maxY-minY).
pub const PAGE_WIDTH: i32 = MAX_X - MIN_X;
pub const PAGE_HEIGHT: i32 = MAX_Y - MIN_Y;
/// Java BookDisplay spacing = 3.
pub const LINE_SPACING: i32 = 3;

/// Java default empty book text.
pub const DEFAULT_BOOK: &str = "This book has no text.";

/// Split book text into pages; `\0` is a page break (empty sections drop out).
pub fn pages(text: &str) -> Vec<Vec<String>> {
    let mut pages = Vec::new();
    // Empty NUL-delimited sections produce no page in Java BookDisplay.
    for section in text.split('\0') {
        let mut remainder = section.to_owned();
        while !remainder.is_empty() {
            let mut lines =
                font::get_lines(&remainder, PAGE_WIDTH, PAGE_HEIGHT, LINE_SPACING, true);
            remainder = lines.pop().unwrap_or_default();
            pages.push(lines);
        }
    }
    pages
}

/// Java page-count chip: framed menu at (w/2, 0) BOTTOM with "Page" + "n/m".
pub fn build_page_count_menu(page: usize, page_count: usize) -> Menu {
    let entries = vec![
        Entry::string_colored("Page", crate::gfx::color::BLACK_RGB),
        Entry::string_colored(
            format!("{}/{}", page + 1, page_count),
            crate::gfx::color::BLACK_RGB,
        ),
    ];
    MenuBuilder::new(true, LINE_SPACING, RelPos::CENTER, entries)
        .set_positioning((WIDTH as i32 / 2, 0), RelPos::BOTTOM)
        .set_selection(1)
        .create_menu()
}

/// Java body page menu: framed, size = content + 2*boxWidth insets.
pub fn build_page_menu(lines: &[String]) -> Menu {
    let entries: Vec<Entry> = lines
        .iter()
        .map(|line| Entry::string(line.clone()))
        .collect();
    MenuBuilder::new(true, LINE_SPACING, RelPos::CENTER, entries)
        .set_size(
            PAGE_WIDTH + 8 * 2,
            PAGE_HEIGHT + 8 * 2,
        )
        .set_positioning((WIDTH as i32 / 2, MIN_Y), RelPos::CENTER)
        .create_menu()
}

/// Legacy single-menu page builder used by the compatibility host.
pub fn build_menu(crate_book_pages: &[Vec<String>], page: usize) -> Menu {
    let page_count = crate_book_pages.len().max(1);
    let page = page.min(page_count - 1);
    let lines = crate_book_pages
        .get(page)
        .cloned()
        .unwrap_or_default();
    build_page_menu(&lines)
}

/// Java turnPage: returns the new page index if the turn is legal.
pub fn turn_page(page: usize, dir: i32, page_count: usize) -> Option<usize> {
    let next = page as i32 + dir;
    if next >= 0 && (next as usize) < page_count {
        Some(next as usize)
    } else {
        None
    }
}

/// Help/footer unused; keep screen height helper for host renderers.
pub fn page_menu_height() -> i32 {
    HEIGHT as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_geometry_matches_java_min_max() {
        assert_eq!(PAGE_WIDTH, 256);
        assert_eq!(PAGE_HEIGHT, 128);
        assert_eq!(LINE_SPACING, 3);
        assert_eq!(MIN_X, 15);
        assert_eq!(MAX_X, 271);
        assert_eq!(MIN_Y, 40);
        assert_eq!(MAX_Y, 168);
    }

    #[test]
    fn page_menu_is_framed_with_java_size() {
        let menu = build_page_menu(&["hello".into(), "world".into()]);
        assert!(menu.has_frame);
        assert_eq!(menu.bounds.2, PAGE_WIDTH + 16);
        assert_eq!(menu.bounds.3, PAGE_HEIGHT + 16);
        assert_eq!(menu.spacing, LINE_SPACING);
    }

    #[test]
    fn turn_page_clamps_like_java() {
        assert_eq!(turn_page(0, -1, 3), None);
        assert_eq!(turn_page(0, 1, 3), Some(1));
        assert_eq!(turn_page(2, 1, 3), None);
        assert_eq!(turn_page(2, -1, 3), Some(1));
    }

    #[test]
    fn pages_split_on_nul_and_wrap_text() {
        let text = format!("{}\0{}", "word ".repeat(50), "second section");
        let pages = pages(&text);
        assert!(pages.len() >= 2);
    }
}
