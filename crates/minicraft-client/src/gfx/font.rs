//! Bitmap glyph layout and measurement corresponding to Java Font.
use super::color::{self, COLOR_CHAR};
use super::{Image, Screen};

const FONT_CHARS: &str = concat!(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ012345",
    "6789.,!?'\"-+=/\\%()<>:;^@ÁÉÍÓÚÑ¿¡",
    "ÃÊÇÔÕĞÇÜİÖŞÆØÅŰŐ[]#|{}_АБВГДЕЁЖЗ",
    "ИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯÀÂÄÈÎÌÏÒ",
    "ÙÛÝ*«»£$&€§ªºabcdefghijklmnopqrs",
    "tuvwxyzáàãâäéèêëíìîïóòõôöúùûüçñý",
    "ÿабвгдеёжзийклмнопрстуфхцчшщъыьэ",
    "юяışő✓"
);

/// Java String.length counts UTF-16 units, including unsupported glyphs.
/// Font.textWidth removes five units for every inline color marker.
pub fn text_width(text: &str) -> i32 {
    let units = text.encode_utf16().count();
    let codes = text.chars().filter(|&ch| ch == COLOR_CHAR).count();
    units.saturating_sub(codes * 5).min((i32::MAX / 8) as usize) as i32 * 8
}

fn units_width(text: &[u16]) -> i32 {
    text.len()
        .saturating_sub(text.iter().filter(|&&c| c == 0xa7).count() * 5) as i32
        * 8
}

/// Java getLine's index is measured in UTF-16 units, including its newline
/// normalization before finding the split in the original string.
fn line_end(text: &[u16], width: i32) -> usize {
    if width <= 0 {
        return 0;
    }
    let mut normalized = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let newline = if text[i] == 32 && text.get(i + 1) == Some(&10) {
            i + 1
        } else {
            i
        };
        if text[newline] == 10 {
            normalized.extend_from_slice(&[32, 10, 32]);
            i = newline + 1;
            if text.get(i) == Some(&32) {
                i += 1;
            }
        } else {
            normalized.push(text[i]);
            i += 1;
        }
    }
    let words: Vec<&[u16]> = normalized.split(|&c| c == 32).collect();
    let mut current = units_width(words[0]);
    if current > width {
        let mut end = 1;
        while end < words[0].len() && units_width(&words[0][..end + 1]) <= width {
            end += 1;
        }
        return end;
    }
    let mut end = words[0].len();
    for word in words.iter().skip(1) {
        if *word == [10] {
            break;
        }
        // Measure the leading space together with the word: malformed color
        // codes can clamp a word's width, so adding eight afterwards is wrong.
        let mut spaced = vec![32];
        spaced.extend_from_slice(word);
        current += units_width(&spaced);
        if current > width {
            break;
        }
        end += 1 + word.len();
    }
    end
}

/// Java Font.getLines: the final element is the unconsumed remainder when
/// height runs out, not another wrapped line. Unpaired UTF-16 is represented
/// with U+FFFD at the Rust String boundary.
pub fn get_lines(
    text: &str,
    width: i32,
    height: i32,
    spacing: i32,
    keep_empty: bool,
) -> Vec<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut rest = units.as_slice();
    let mut lines = Vec::new();
    let mut used_height: i64 = 8;
    while !rest.is_empty() {
        let mut end = line_end(rest, width).min(rest.len());
        lines.push(String::from_utf16_lossy(&rest[..end]));
        if matches!(rest.get(end), Some(32 | 10)) {
            end += 1;
        }
        rest = &rest[end..];
        used_height += i64::from(spacing) + 8;
        // Avoid the Java non-progress loop for invalid width/line spacing.
        if used_height > i64::from(height) || (end == 0 && spacing <= -8) {
            break;
        }
    }
    if !rest.is_empty() || keep_empty {
        lines.push(String::from_utf16_lossy(rest));
    }
    lines
}

/// Java Color.toStringCode: marker followed by four character-valued bytes,
/// not printable hexadecimal digits.
pub fn color_code(color: u32) -> String {
    color::to_string_code(color)
}

/// Font.drawColor segmentation, including Java's white fallback for short codes.
/// Empty input is a safe no-op instead of Java's charAt(0) exception.
pub fn draw_color(screen: &mut Screen, font: &Image, text: &str, x: i32, y: i32) {
    if text.is_empty() {
        return;
    }
    let message = if text.starts_with(COLOR_CHAR) {
        text.to_owned()
    } else {
        color_code(color::WHITE) + text
    };
    let mut leading = 0;
    for segment in message.split(COLOR_CHAR).filter(|part| !part.is_empty()) {
        let units: Vec<u16> = segment.encode_utf16().collect();
        let (content, tint) = if units.len() >= 4 {
            // Java Color.get(String): four raw component chars after the marker.
            let packed = ((units[0] as u32) << 24)
                .wrapping_add((units[1] as u32) << 16)
                .wrapping_add((units[2] as u32) << 8)
                .wrapping_add(units[3] as u32);
            (
                String::from_utf16_lossy(&units[4..]),
                color::upgrade(packed),
            )
        } else {
            (segment.to_owned(), color::WHITE_RGB)
        };
        draw(screen, font, &content, x + leading, y, Some(tint));
        leading += text_width(&content);
    }
}

/// Plain Font.draw; inline color parsing is a separate Java drawColor operation.
/// Colored glyphs use Java's white-only replacement rather than sprite tinting.
pub fn draw(screen: &mut Screen, font: &Image, text: &str, x: i32, y: i32, tint: Option<u32>) {
    for (offset, unit) in text.encode_utf16().enumerate() {
        let Some(index) = FONT_CHARS
            .encode_utf16()
            .position(|candidate| candidate == unit)
        else {
            continue;
        };
        // For the color marker alone Java textWidth returns zero.
        let advance = if unit == 0x00a7 { 0 } else { 8 };
        let draw_x = x + offset as i32 * advance;
        let source_x = index % 32 * 8;
        let source_y = index / 32 * 8;
        if let Some(color) = tint {
            screen.blit_font_glyph(font, draw_x, y, index, color);
        } else {
            screen.blit_region(font, draw_x, y, source_x, source_y, 8, 8, false);
        }
    }
}

/// Java Font.drawCentered: FontStyle(color).setYPos(y).draw(msg).
pub fn draw_centered(screen: &mut Screen, font: &Image, message: &str, y: i32, color: u32) {
    super::font_style::FontStyle::draw_centered(screen, font, message, y, color);
}

#[cfg(test)]
mod tests {
    use super::{FONT_CHARS, color_code, text_width};
    use crate::gfx::color::{self, COLOR_CHAR};

    #[test]
    fn text_width_matches_java_formula() {
        assert_eq!(text_width(""), 0);
        assert_eq!(text_width("A"), 8);
        assert_eq!(text_width("Hello"), 40);
        // One color marker removes five UTF-16 units before * 8.
        let colored = format!("{}AB", color_code(color::WHITE));
        assert_eq!(text_width(&colored), 16);
        // "§ABCD" is a complete color code with empty payload → width 0.
        assert_eq!(text_width("§ABCD"), 0);
        // Code plus two payload glyphs.
        assert_eq!(text_width("§ABCDXY"), 16);
        // Never negative: a lone marker has length 1 - 5 → clamp to 0.
        assert_eq!(text_width("§"), 0);
    }

    #[test]
    fn color_code_is_marker_plus_four_raw_bytes() {
        let code = color_code(color::RED);
        assert_eq!(code.chars().next(), Some(COLOR_CHAR));
        assert_eq!(code.chars().count(), 5);
        let units: Vec<u16> = code.encode_utf16().collect();
        assert_eq!(units[1], 1);
        assert_eq!(units[2], 198);
        assert_eq!(units[3], 44);
        assert_eq!(units[4], 44);
    }

    #[test]
    fn font_chars_match_java_layout() {
        // First glyph row is the Latin uppercase alphabet + digits.
        // Java String.length is UTF-16 units; same 8×32+6 layout.
        assert_eq!(FONT_CHARS.encode_utf16().count(), 230);
        assert_eq!(FONT_CHARS.chars().next(), Some('A'));
        assert!(FONT_CHARS.contains('✓'));
        // Java Font.chars includes § (inline color marker); it is in the sheet.
        assert!(FONT_CHARS.contains('§'));
    }
}
