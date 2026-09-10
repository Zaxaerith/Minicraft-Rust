//! Player walk frames and attack slash rendering.
use super::{Direction, Screen};

pub(crate) fn player_sprite_frame(direction: Direction, walk_distance: u32) -> (usize, bool) {
    let frame = walk_distance as usize / 8 % 2;
    match direction {
        Direction::Down => (0, frame == 1),
        Direction::Up => (16, frame == 1),
        Direction::Left => (32 + frame * 16, true),
        Direction::Right => (32 + frame * 16, false),
    }
}

pub(super) fn render_attack_slash(
    screen: &mut Screen,
    hud: &crate::gfx::Image,
    x: i32,
    y: i32,
    direction: Direction,
) {
    let mut part = |source_x, draw_x, draw_y, flip_x, flip_y| {
        screen.blit_region_transformed(hud, source_x, 0, 8, 8, draw_x, draw_y, flip_x, flip_y);
    };
    match direction {
        Direction::Up => {
            part(24, x, y - 4, false, false);
            part(24, x + 8, y - 4, true, false);
        }
        Direction::Left => {
            part(32, x - 4, y, true, false);
            part(32, x - 4, y + 8, true, true);
        }
        Direction::Right => {
            part(32, x + 12, y, false, false);
            part(32, x + 12, y + 8, false, true);
        }
        Direction::Down => {
            part(24, x, y + 12, false, true);
            part(24, x + 8, y + 12, true, true);
        }
    }
}
