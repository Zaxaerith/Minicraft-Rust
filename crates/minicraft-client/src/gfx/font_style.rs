//! Single-line styling and Java-compatible paragraph placement.
use super::{Image, Screen, font};
use crate::screen::rel_pos::RelPos;

pub struct ParagraphLayout {
    pub anchor: (i32, i32),
    pub block_position: RelPos,
    pub line_position: RelPos,
    pub spacing: i32,
}

pub struct FontStyle {
    pub color: u32,
    pub shadow_color: u32,
    /// Bits 0..7 correspond to N, NE, E, SE, S, SW, W, NW.
    pub shadow_mask: u8,
}

impl FontStyle {
    pub fn draw_paragraph(
        &self,
        screen: &mut Screen,
        image: &Image,
        lines: &[&str],
        layout: &ParagraphLayout,
    ) {
        let width = lines
            .iter()
            .map(|text| font::text_width(text))
            .max()
            .unwrap_or(0);
        // Java includes spacing after the last line in the paragraph bounds.
        let height = lines.len() as i32 * (8 + layout.spacing);
        let (left, top) = layout.block_position.around((width, height), layout.anchor);
        for (index, text) in lines.iter().enumerate() {
            let area = (
                left,
                top + index as i32 * (8 + layout.spacing),
                width,
                8 + layout.spacing,
            );
            let anchor = layout.block_position.opposite().point(area);
            let padded = layout.block_position.around((width, area.3), anchor);
            let (x, y) = if width == font::text_width(text) && layout.spacing == 0 {
                layout.block_position.around((width, 8), anchor)
            } else {
                layout.line_position.in_rect(
                    (font::text_width(text), 8),
                    (padded.0, padded.1, width, area.3),
                )
            };
            self.draw_at(screen, image, text, x, y);
        }
    }

    pub fn draw_at(&self, screen: &mut Screen, image: &Image, text: &str, x: i32, y: i32) {
        const OFFSETS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        for (bit, (dx, dy)) in OFFSETS.into_iter().enumerate() {
            if self.shadow_mask & (1 << bit) != 0 {
                font::draw(screen, image, text, x + dx, y + dy, Some(self.shadow_color));
            }
        }
        font::draw(screen, image, text, x, y, Some(self.color));
    }
}
