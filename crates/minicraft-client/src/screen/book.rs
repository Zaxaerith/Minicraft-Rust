//! BookDisplay pagination; the legacy display remains the rendering adapter.
use crate::gfx::font;

pub const PAGE_WIDTH: i32 = 256;
pub const PAGE_HEIGHT: i32 = 128;
pub const LINE_SPACING: i32 = 3;

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
