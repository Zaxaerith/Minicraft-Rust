//! Java `BookItem` — readable books (Game Guide / Antidious).
use super::ItemId;

/// Book variants keyed by ItemId (Java `BookItem.book` content supplier).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookKind {
    /// Java `BookItem("Book", … Localization default_book)`.
    GameGuide,
    /// Java `BookItem("Antidious", … BookData.antVenomBook)`.
    Antidious,
}

impl BookKind {
    pub const fn item(self) -> ItemId {
        match self {
            Self::GameGuide => ItemId::Book,
            Self::Antidious => ItemId::AntidiousBook,
        }
    }

    /// Java `BookItem.hasTitlePage` — only Antidious opens with a title page.
    pub const fn has_title_page(self) -> bool {
        matches!(self, Self::Antidious)
    }
}

pub const ALL: [ItemId; 2] = [ItemId::Book, ItemId::AntidiousBook];

pub const fn book_for_item(item: ItemId) -> Option<BookKind> {
    match item {
        ItemId::Book => Some(BookKind::GameGuide),
        ItemId::AntidiousBook => Some(BookKind::Antidious),
        _ => None,
    }
}

/// Java `BookItem.interactsWithWorld` is false.
pub const fn interacts_with_world(item: ItemId) -> bool {
    book_for_item(item).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn books_map_both_ways() {
        assert_eq!(book_for_item(ItemId::Book), Some(BookKind::GameGuide));
        assert_eq!(
            book_for_item(ItemId::AntidiousBook),
            Some(BookKind::Antidious)
        );
        assert!(BookKind::Antidious.has_title_page());
        assert!(!BookKind::GameGuide.has_title_page());
        assert_eq!(book_for_item(ItemId::Wood), None);
    }
}
