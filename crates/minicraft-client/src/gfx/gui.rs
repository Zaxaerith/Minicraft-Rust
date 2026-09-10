//! Java Menu frame tiles, title background, and inventory counter digits.
use super::{Image, Screen, sprite::SpriteRender};

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

/// PlayerInvDisplay / ContainerDisplay capacity counter plate + digits.
pub fn render_inventory_counter(
    screen: &mut Screen,
    counter: &Image,
    right: i32,
    top: i32,
    size: usize,
    capacity: usize,
) {
    if size < 10 {
        screen.blit_region(counter, right - 16, top - 3, 12, 12, 3, 13, false);
        screen.blit_region(counter, right - 13, top - 3, 20, 12, 15, 13, false);
    } else {
        screen.blit_region(counter, right - 21, top - 3, 12, 12, 23, 13, false);
    }
    let current_color = inventory_counter_color(size, capacity, true);
    render_counter_digits(
        screen,
        counter,
        right - if size < 10 { 14 } else { 19 },
        top - 1,
        5,
        5,
        7,
        size,
        current_color,
    );
    render_counter_digits(
        screen,
        counter,
        right - 8,
        top + 3,
        0,
        4,
        5,
        capacity,
        0x999999,
    );
}

pub fn render_inventory_counter_minimized(
    screen: &mut Screen,
    counter: &Image,
    right: i32,
    top: i32,
    size: usize,
) {
    if size < 10 {
        screen.blit_region(counter, right - 12, top - 1, 0, 12, 4, 9, false);
        screen.blit_region(counter, right - 8, top - 1, 8, 12, 4, 9, false);
        render_counter_digits(
            screen,
            counter,
            right - 10,
            top + 1,
            0,
            4,
            5,
            size,
            inventory_counter_faded_color(size, 27),
        );
    } else {
        screen.blit_region(counter, right - 16, top - 1, 0, 12, 12, 9, false);
        render_counter_digits(
            screen,
            counter,
            right - 14,
            top + 1,
            0,
            4,
            5,
            size,
            inventory_counter_faded_color(size, 27),
        );
    }
}

fn render_counter_digits(
    screen: &mut Screen,
    counter: &Image,
    x: i32,
    y: i32,
    source_y: usize,
    width: usize,
    height: usize,
    number: usize,
    color: u32,
) {
    for (index, digit) in number.to_string().bytes().enumerate() {
        screen.render_sprite(
            counter,
            x + index as i32 * width as i32,
            y,
            SpriteRender {
                source_x: usize::from(digit - b'0') * width,
                source_y,
                width,
                height,
                white_tint: Some(color),
                ..Default::default()
            },
        );
    }
}

pub fn inventory_counter_color(size: usize, capacity: usize, main: bool) -> u32 {
    let capacity = capacity.max(1) as f32;
    let inner = (size as f32 / capacity).clamp(0.0, 1.0) - 1.0;
    let heaviness = (-(inner * inner) + 1.0).clamp(0.0, 1.0);
    if main {
        if heaviness < 0.5 {
            ((heaviness / 0.5 * 255.0) as u32) << 16 | 0x00FF00
        } else {
            (((1.0 - heaviness) / 0.5 * 255.0) as u32) << 8 | 0xFF0000
        }
    } else if heaviness < 0.3 {
        (heaviness / 0.3 * 255.0) as u32 | 0xFFFF00
    } else {
        (((1.0 - heaviness) / 0.7 * 255.0) as u32) << 8 | 0xFF0000
    }
}

fn inventory_counter_faded_color(size: usize, capacity: usize) -> u32 {
    let color = inventory_counter_color(size, capacity, false);
    let blend = |background: u32, foreground: u32| {
        (background as i32 + ((foreground as i32 - background as i32) as f32 * 0.35) as i32) as u32
    };
    let red = blend(0x2A, (color >> 16) & 0xFF);
    let green = blend(0x32, (color >> 8) & 0xFF);
    let blue = blend(0x99, color & 0xFF);
    red << 16 | green << 8 | blue
}
