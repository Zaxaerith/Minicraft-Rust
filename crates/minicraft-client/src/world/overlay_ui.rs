//! World-book and sign-editor overlays drawn while a world Display is open.
use super::{Assets, HEIGHT, Screen, WIDTH};
use crate::progression::Book;

pub(super) fn render_world_book(screen: &mut Screen, assets: &Assets, book: Book, page: usize) {
    let pages = book.pages();
    let page = page.min(pages.len().saturating_sub(1));
    screen.rect(18, 17, WIDTH as i32 - 36, HEIGHT as i32 - 34, 0xD8C68F);
    screen.rect(22, 21, WIDTH as i32 - 44, HEIGHT as i32 - 42, 0x33291C);
    screen.centered_text(assets.font(), book.title(), 27);
    for (row, line) in pages[page].iter().enumerate() {
        screen.text(assets.font(), line, 29, 42 + row as i32 * 8);
    }
    screen.centered_text(
        assets.font(),
        &format!("< PAGE {}/{} >  SELECT CLOSE", page + 1, pages.len()),
        HEIGHT as i32 - 25,
    );
}

pub(super) fn render_sign_editor(screen: &mut Screen, assets: &Assets, text: &str) {
    screen.rect(36, 72, WIDTH as i32 - 72, 69, 0x101018);
    screen.centered_text(assets.font(), "EDIT SIGN", 81);
    let visible = text.chars().take(28).collect::<String>();
    screen.rect(47, 99, WIDTH as i32 - 94, 13, 0x493A28);
    screen.centered_text(assets.font(), &format!("{visible}_"), 102);
    screen.centered_text(assets.font(), "TYPE  BACKSPACE  SELECT SAVE", 124);
}
