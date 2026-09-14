//! Java `util/BookData.java` — bundled book contents and page-break decoding.

use serde::{Deserialize, Serialize};

/// Java `BookData` book slots (about / credits / instructions / antVenomBook / storylineGuide).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Book {
    Instructions,
    GameGuide,
    About,
    Credits,
    Antidious,
}

impl Book {
    pub const ALL: [Self; 4] = [
        Self::Instructions,
        Self::GameGuide,
        Self::About,
        Self::Credits,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Instructions => "INSTRUCTIONS",
            Self::GameGuide => "GAME GUIDE",
            Self::About => "ABOUT",
            Self::Credits => "CREDITS",
            Self::Antidious => "ANTIDIOUS VENOMI",
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::Instructions => include_str!("../../../../assets/assets/books/instructions.txt"),
            Self::GameGuide => include_str!("../../../../assets/assets/books/game_guide.txt"),
            Self::About => include_str!("../../../../assets/assets/books/about.txt"),
            Self::Credits => include_str!("../../../../assets/assets/books/credits.txt"),
            Self::Antidious => include_str!("../../../../assets/assets/books/antidous.txt"),
        }
    }

    pub fn pages(self) -> Vec<Vec<String>> {
        // BookData.loadBook decodes the literal page-break escape first.
        let text = load_book(self.text());
        let pages = crate::screen::book::pages(&text);
        // Legacy render/input adapters require at least one page. Keep this
        // empty-resource guard outside the Java pagination implementation.
        if pages.is_empty() {
            vec![Vec::new()]
        } else {
            pages
        }
    }
}

/// Java `BookData.loadBook` — replace the literal two-char sequence `\0` with NUL.
pub fn load_book(content: &str) -> String {
    content.replace("\\0", "\0")
}

/// Java `BookData.resetBooks` — all slots become null.
pub fn reset_books() {
    // Bundled books are compile-time constants; nothing to clear.
}

/// Java `BookData.saveBook` — currently a no-op in Java.
pub fn save_book(_book_title: &str) {
    // Intentionally empty (matches Java).
}

#[cfg(test)]
mod tests {
    use super::{Book, load_book};

    #[test]
    fn load_book_decodes_page_break_escape() {
        assert_eq!(load_book("a\\0b"), "a\0b");
    }

    #[test]
    fn bundled_books_paginate() {
        assert!(Book::GameGuide.pages().len() > 1);
        assert_eq!(Book::About.title(), "ABOUT");
        assert_eq!(Book::ALL.len(), 4);
    }
}
