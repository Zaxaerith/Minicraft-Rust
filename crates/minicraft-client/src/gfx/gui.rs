//! Java Menu frame tiles and title background painting.
use super::{Image, Screen};
pub fn render_menu_frame(
    screen: &mut Screen,
    hud: &Image,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let right = x + width - 8;
    let bottom = y + height - 8;
    let mut draw_y = y;
    while draw_y <= bottom {
        let mut draw_x = x;
        while draw_x <= right {
            let edge_x = draw_x == x || draw_x == right;
            let edge_y = draw_y == y || draw_y == bottom;
            let source_x = if edge_x && edge_y {
                0
            } else if edge_y {
                8
            } else if edge_x {
                16
            } else {
                24
            };
            screen.blit_region_transformed(
                hud,
                source_x,
                48,
                8,
                8,
                draw_x,
                draw_y,
                draw_x == right,
                draw_y == bottom,
            );
            // Menu.renderFrame overlaps the last tile when bounds are not
            // multiples of eight; never omit the right or bottom border.
            draw_x = if draw_x < right && draw_x + 8 > right {
                right
            } else {
                draw_x + 8
            };
        }
        draw_y = if draw_y < bottom && draw_y + 8 > bottom {
            bottom
        } else {
            draw_y + 8
        };
    }
}

pub fn render_title(
    screen: &mut Screen,
    hud: &Image,
    font: &Image,
    title: &str,
    x: i32,
    y: i32,
    color: u32,
) {
    for offset in (0..title.encode_utf16().count() as i32 * 8).step_by(8) {
        screen.blit_region(hud, x + offset, y, 24, 48, 8, 8, false);
    }
    screen.text_colored(font, title, x, y, color);
}
