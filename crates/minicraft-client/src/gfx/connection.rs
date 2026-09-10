//! SpriteAnimation's four-quadrant connective rendering.
use super::{
    Image, Screen,
    sprite::{Px, Sprite},
};

pub struct Connection<'a> {
    pub border: &'a Image,
    pub corner: Option<&'a Image>,
    pub singleton: bool,
}

impl Connection<'_> {
    pub fn render(
        &self,
        screen: &mut Screen,
        full: &Image,
        frame: usize,
        position: (i32, i32),
        neighbors: [bool; 8],
    ) {
        let [u, d, l, r, ul, dl, ur, dr] = neighbors;
        let directions = [(u, l, ul), (u, r, ur), (d, l, dl), (d, r, dr)];
        let mut cells = directions
            .into_iter()
            .enumerate()
            .map(|(index, (v, h, diagonal))| {
                let (sheet, sx, sy, mirror) = if v && h {
                    if !diagonal && let Some(corner) = self.corner {
                        // Java passes mirror=3 for concave corner Px cells.
                        (corner, index % 2, index / 2, 3)
                    } else if self.singleton || !diagonal {
                        (full, 1 - index % 2, frame * 2 + 1 - index / 2, 0)
                    } else {
                        (self.border, 1, 1, 0)
                    }
                } else {
                    (
                        self.border,
                        if h { 1 } else { index % 2 * 2 },
                        if v { 1 } else { index / 2 * 2 },
                        0,
                    )
                };
                Px {
                    sheet,
                    sheet_x: sx,
                    sheet_y: sy,
                    mirror,
                }
            });
        let sprite = Sprite {
            rows: vec![cells.by_ref().take(2).collect(), cells.collect()],
            white_tint: None,
        };
        sprite.render(screen, position.0, position.1, 0, false, 0);
    }
}
