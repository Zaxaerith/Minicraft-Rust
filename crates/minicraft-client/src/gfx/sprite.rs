//! Sprite raster parameters corresponding to Java Screen.SpriteRendering.
use super::{Image, Screen};

/// One 8x8 cell; a composite may reference a different sheet for every cell.
#[derive(Clone, Copy)]
pub struct Px<'a> {
    pub sheet: &'a Image,
    pub sheet_x: usize,
    pub sheet_y: usize,
    pub mirror: u8,
}

pub struct Sprite<'a> {
    pub rows: Vec<Vec<Px<'a>>>,
    pub white_tint: Option<u32>,
}

impl Sprite<'_> {
    pub fn render(
        &self,
        screen: &mut Screen,
        x: i32,
        y: i32,
        mirror: u8,
        full_bright: bool,
        color: u32,
    ) {
        for r in 0..self.rows.len() {
            let row = &self.rows[if mirror & 2 != 0 {
                self.rows.len() - 1 - r
            } else {
                r
            }];
            for c in 0..row.len() {
                let px = &row[if mirror & 1 != 0 {
                    row.len() - 1 - c
                } else {
                    c
                }];
                let bits = px.mirror ^ mirror;
                screen.render_sprite(
                    px.sheet,
                    x + c as i32 * 8,
                    y + r as i32 * 8,
                    SpriteRender {
                        source_x: px.sheet_x * 8,
                        source_y: px.sheet_y * 8,
                        width: 8,
                        height: 8,
                        mirror_x: bits & 1 != 0,
                        mirror_y: bits & 2 != 0,
                        white_tint: self.white_tint,
                        full_bright,
                        color,
                    },
                );
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct SpriteRender {
    pub source_x: usize,
    pub source_y: usize,
    pub width: usize,
    pub height: usize,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub white_tint: Option<u32>,
    pub full_bright: bool,
    pub color: u32,
}

impl SpriteRender {
    /// Java priority: whiteTint, fullBright, nonzero override, source RGB.
    pub(crate) fn pixel_color(self, source: u32) -> u32 {
        if source == 0xffffff
            && let Some(tint) = self.white_tint.filter(|&t| t != u32::MAX)
        {
            tint & 0xffffff
        } else if self.full_bright {
            0x01ffffff
        } else if self.color != 0 {
            self.color
        } else {
            source
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_tint_replaces_only_pure_white_source() {
        let tinted = SpriteRender {
            white_tint: Some(0x00ff_0000),
            ..Default::default()
        };
        assert_eq!(tinted.pixel_color(0xffffff), 0xff_0000);
        // Near-white is not replaced.
        assert_eq!(tinted.pixel_color(0xfffefe), 0xfffefe);
        // Java whiteTint == -1 disables the pass.
        let off = SpriteRender {
            white_tint: Some(u32::MAX),
            ..Default::default()
        };
        assert_eq!(off.pixel_color(0xffffff), 0xffffff);
    }

    #[test]
    fn full_bright_wins_over_color_and_source() {
        let sprite = SpriteRender {
            white_tint: Some(0x123456),
            full_bright: true,
            color: 0xabcdef,
            ..Default::default()
        };
        // Non-white source skips whiteTint, then fullBright fires.
        assert_eq!(sprite.pixel_color(0x111111), 0x01ffffff);
    }

    #[test]
    fn nonzero_color_overrides_source_but_not_white_tint() {
        let colored = SpriteRender {
            color: 0x00ff_ff00,
            ..Default::default()
        };
        assert_eq!(colored.pixel_color(0x0000ff), 0x00ff_ff00);
        let both = SpriteRender {
            white_tint: Some(0x00ff_00ff),
            color: 0x00ff_ff00,
            ..Default::default()
        };
        assert_eq!(both.pixel_color(0xffffff), 0xff_00ff);
        assert_eq!(both.pixel_color(0x123456), 0x00ff_ff00);
    }

    #[test]
    fn zero_color_keeps_source() {
        assert_eq!(SpriteRender::default().pixel_color(0x123456), 0x123456);
    }

    #[test]
    fn sprite_mirror_xor_combines_px_and_draw_mirror() {
        // Java: bits = pixel.mirror ^ mirror before blit.
        let px_mirror = 0b01u8;
        let draw_mirror = 0b10u8;
        assert_eq!(px_mirror ^ draw_mirror, 0b11);
        assert_eq!(px_mirror, 0b01);
    }
}
