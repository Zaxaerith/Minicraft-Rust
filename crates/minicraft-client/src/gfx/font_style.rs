//! Java FontStyle: single-line RelPos draw, shadows, and paragraph line setup.
use super::{Image, Screen, font};
use crate::screen::rel_pos::RelPos;

/// Java Font.textHeight() is the fixed 8px glyph box.
pub const TEXT_HEIGHT: i32 = 8;

pub struct ParagraphLayout {
    pub anchor: (i32, i32),
    pub block_position: RelPos,
    pub line_position: RelPos,
    pub spacing: i32,
}

/// Mutable style object matching Java FontStyle's chained configuration.
/// Drawing never mutates the public fields; pad/anchor work on locals.
#[derive(Clone)]
pub struct FontStyle {
    pub color: u32,
    pub shadow_color: u32,
    /// Bits 0..7 correspond to N, NE, E, SE, S, SW, W, NW.
    pub shadow_mask: u8,
    pub anchor: (i32, i32),
    pub rel_text_pos: RelPos,
    pub rel_line_pos: RelPos,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::new(0xFF_FF_FF)
    }
}

impl FontStyle {
    pub fn new(color: u32) -> Self {
        Self {
            color,
            shadow_color: 0x33_33_33,
            shadow_mask: 0,
            // Java default: centered on Screen.w/2, Screen.h/2 with no shadow.
            anchor: (super::WIDTH as i32 / 2, super::HEIGHT as i32 / 2),
            rel_text_pos: RelPos::CENTER,
            rel_line_pos: RelPos::CENTER,
        }
    }

    pub fn set_color(mut self, color: u32) -> Self {
        self.color = color;
        self
    }

    /// Java setYPos(pos, resetAlignment): BOTTOM text / TOP lines.
    pub fn set_y_pos(self, y: i32) -> Self {
        self.set_y_pos_aligned(y, true)
    }

    pub fn set_y_pos_aligned(mut self, y: i32, reset_alignment: bool) -> Self {
        self.anchor.1 = y;
        if reset_alignment {
            self.rel_text_pos = RelPos::new(self.rel_text_pos.x_index(), 2);
            self.rel_line_pos = RelPos::new(self.rel_line_pos.x_index(), 0);
        }
        self
    }

    pub fn set_x_pos(self, x: i32) -> Self {
        self.set_x_pos_aligned(x, true)
    }

    pub fn set_x_pos_aligned(mut self, x: i32, reset_alignment: bool) -> Self {
        self.anchor.0 = x;
        if reset_alignment {
            self.rel_text_pos = RelPos::new(2, self.rel_text_pos.y_index());
            self.rel_line_pos = RelPos::new(0, self.rel_line_pos.y_index());
        }
        self
    }

    pub fn set_anchor(mut self, x: i32, y: i32) -> Self {
        self.anchor = (x, y);
        self
    }

    /// setBoth also flips rel_line_pos to the opposite of rel_text_pos.
    pub fn set_rel_text_pos(mut self, pos: RelPos, set_both: bool) -> Self {
        self.rel_text_pos = pos;
        if set_both {
            self.rel_line_pos = pos.opposite();
        }
        self
    }

    pub fn set_rel_line_pos(mut self, pos: RelPos) -> Self {
        self.rel_line_pos = pos;
        self
    }

    /// false → bottom-right single shadow "0001"; true → full outline "10101010".
    pub fn set_shadow_type(mut self, color: u32, full: bool) -> Self {
        self.shadow_mask = if full { 0b1010_1010 } else { 1 << 3 };
        self.shadow_color = color;
        self
    }

    pub fn set_shadow_mask(mut self, color: u32, mask: u8) -> Self {
        self.shadow_color = color;
        self.shadow_mask = mask;
        self
    }

    /// Java FontStyle.draw(String, Screen): place one line relative to the anchor.
    pub fn draw(&self, screen: &mut Screen, image: &Image, text: &str) {
        let size = (font::text_width(text), TEXT_HEIGHT);
        let (x, y) = self.rel_text_pos.around(size, self.anchor);
        self.draw_at(screen, image, text, x, y);
    }

    /// Java Font.drawCentered(msg, screen, y, color).
    pub fn draw_centered(screen: &mut Screen, image: &Image, text: &str, y: i32, color: u32) {
        Self::new(color).set_y_pos(y).draw(screen, image, text);
    }

    /// Java Font.drawParagraph: configure once, then draw each line with pad.
    pub fn draw_paragraph(&self, screen: &mut Screen, image: &Image, lines: &[&str], spacing: i32) {
        let width = lines
            .iter()
            .map(|text| font::text_width(text))
            .max()
            .unwrap_or(0);
        let height = lines.len() as i32 * (TEXT_HEIGHT + spacing);
        // Java: positionRect(size, anchor) builds a 2× box around the anchor point.
        let para_bounds = self.rel_text_pos.around((width, height), self.anchor);
        for (index, text) in lines.iter().enumerate() {
            // setupParagraphLine: one-line TOP_LEFT strip inside paraBounds, offset by line.
            let strip_y = para_bounds.1 + index as i32 * (TEXT_HEIGHT + spacing);
            let pad_x = width - font::text_width(text);
            let pad_y = spacing;
            // Java uses the opposite relTextPos as the strip's internal anchor.
            let strip_anchor = (
                para_bounds.0 + self.rel_text_pos.opposite().x_index() * width / 2,
                strip_y,
            );
            let _ = strip_anchor;
            self.draw_line_in(
                screen,
                image,
                text,
                width,
                pad_x,
                pad_y,
                (para_bounds.0, strip_y, width, TEXT_HEIGHT + spacing),
            );
        }
    }

    fn draw_line_in(
        &self,
        screen: &mut Screen,
        image: &Image,
        text: &str,
        block_width: i32,
        pad_x: i32,
        pad_y: i32,
        line_rect: (i32, i32, i32, i32),
    ) {
        let text_width = font::text_width(text);
        let size = (text_width + pad_x, TEXT_HEIGHT + pad_y);
        // Java draw(): place the (possibly padded) text box relative to the line anchor.
        let anchor = (
            line_rect.0 + self.rel_text_pos.opposite().x_index() * block_width / 2,
            line_rect.1,
        );
        let text_bounds = self.rel_text_pos.around(size, anchor);
        // When pad is zero Java still uses relTextPos on the pure text size.
        let (x, y) = if pad_x == 0 && pad_y == 0 {
            self.rel_text_pos.around((text_width, TEXT_HEIGHT), anchor)
        } else {
            let _ = text_bounds;
            self.rel_line_pos.in_rect(
                (text_width, TEXT_HEIGHT),
                (text_bounds.0, text_bounds.1, size.0, size.1),
            )
        };
        self.draw_at(screen, image, text, x, y);
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

    /// Compatibility wrapper used by quest descriptions (BOTTOM/TOP block).
    pub fn draw_paragraph_layout(
        &self,
        screen: &mut Screen,
        image: &Image,
        lines: &[&str],
        layout: &ParagraphLayout,
    ) {
        let styled = self
            .clone()
            .set_anchor(layout.anchor.0, layout.anchor.1)
            .set_rel_text_pos(layout.block_position, false)
            .set_rel_line_pos(layout.line_position);
        styled.draw_paragraph(screen, image, lines, layout.spacing);
    }
}
