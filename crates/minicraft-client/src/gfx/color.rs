//! Java `minicraft.gfx.Color` constants and pack/unpack helpers.
//!
//! Java stores the visibility flag in bit 24 (`Color.get(1, …)`); screen
//! buffers keep only 24-bit RGB via `Color.upgrade`.

/// Java `Color.COLOR_CHAR` (`§`): inline color-code introducer.
pub const COLOR_CHAR: char = '\u{00a7}';

const fn get(a: u32, r: u32, g: u32, b: u32) -> u32 {
    (a << 24) | (r << 16) | (g << 8) | b
}

const fn get_gray(a: u32, copy: u32) -> u32 {
    get(a, copy, copy, copy)
}

pub const TRANSPARENT: u32 = get_gray(0, 0);
pub const WHITE: u32 = get_gray(1, 255);
pub const GRAY: u32 = get_gray(1, 153);
pub const DARK_GRAY: u32 = get_gray(1, 51);
pub const BLACK: u32 = get_gray(1, 0);
pub const RED: u32 = get(1, 198, 44, 44);
pub const GREEN: u32 = get(1, 77, 212, 77);
pub const BLUE: u32 = get(1, 32, 32, 136);
pub const YELLOW: u32 = get(1, 255, 255, 0);
pub const MAGENTA: u32 = get(1, 255, 0, 255);
pub const CYAN: u32 = get(1, 90, 204, 204);

/// `Color.upgrade` — drop the 25th visibility bit for screen storage.
pub const fn upgrade(rgb_minicraft: u32) -> u32 {
    rgb_minicraft & 0x00ff_ffff
}

/// 24-bit forms used by software screen pixels / `whiteTint` after upgrade.
pub const WHITE_RGB: u32 = upgrade(WHITE);
pub const GRAY_RGB: u32 = upgrade(GRAY);
pub const DARK_GRAY_RGB: u32 = upgrade(DARK_GRAY);
pub const BLACK_RGB: u32 = upgrade(BLACK);
pub const RED_RGB: u32 = upgrade(RED);
pub const GREEN_RGB: u32 = upgrade(GREEN);
pub const BLUE_RGB: u32 = upgrade(BLUE);
pub const YELLOW_RGB: u32 = upgrade(YELLOW);
pub const MAGENTA_RGB: u32 = upgrade(MAGENTA);
pub const CYAN_RGB: u32 = upgrade(CYAN);

/// Java `Color.toStringCode`: marker + four raw component characters.
pub fn to_string_code(color: u32) -> String {
    [
        COLOR_CHAR,
        char::from((color >> 24) as u8),
        char::from((color >> 16) as u8),
        char::from((color >> 8) as u8),
        char::from(color as u8),
    ]
    .iter()
    .collect()
}

/// Java `Color.get(String)`: four components, optional leading `COLOR_CHAR`.
pub fn parse_code(code: &str) -> Option<u32> {
    let units: Vec<u16> = code.encode_utf16().collect();
    let leading = usize::from(units.len() == 5);
    if units.len() < 4 + leading {
        return None;
    }
    Some(get(
        u32::from(units[leading]),
        u32::from(units[leading + 1]),
        u32::from(units[leading + 2]),
        u32::from(units[leading + 3]),
    ))
}

/// Java `Color.rgb`: readable 0–5×100 form with components capped at 250.
pub fn rgb(red: i32, green: i32, blue: i32) -> i32 {
    let limit = |num: i32| num.clamp(0, 250);
    limit(red) / 50 * 100 + limit(green) / 50 * 10 + limit(blue) / 50
}

/// Java `Color.decodeRGB`: 216-scale byte → 0–5 triple.
pub fn decode_rgb(rgb_byte: u8) -> [u8; 3] {
    [(rgb_byte / 36) % 6, (rgb_byte / 6) % 6, rgb_byte % 6]
}

/// Java `Color.unGet`: rgbByte → readable hundreds/tens/ones.
pub fn un_get(rgb_byte: u8) -> i32 {
    let cols = decode_rgb(rgb_byte);
    i32::from(cols[0]) * 100 + i32::from(cols[1]) * 10 + i32::from(cols[2])
}

/// Java `Color.separateEncodedSprite` without readable conversion.
pub fn separate_encoded_sprite(rgb4_sprite: u32) -> [u8; 4] {
    [
        (rgb4_sprite >> 24) as u8,
        (rgb4_sprite >> 16) as u8,
        (rgb4_sprite >> 8) as u8,
        rgb4_sprite as u8,
    ]
}

/// Java `Color.getLightnessFromRGB`: HSL L from 24-bit RGB.
pub fn lightness_from_rgb(color: u32) -> f32 {
    let r = ((color >> 16) & 0xff) as f32;
    let g = ((color >> 8) & 0xff) as f32;
    let b = (color & 0xff) as f32;
    (r.max(g).max(b) + r.min(g).min(b)) / 510.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_palette_constants_match_get() {
        assert_eq!(TRANSPARENT, 0);
        assert_eq!(WHITE, 0x01ff_ffff);
        assert_eq!(GRAY, 0x0199_9999);
        assert_eq!(DARK_GRAY, 0x0133_3333);
        assert_eq!(BLACK, 0x0100_0000);
        assert_eq!(RED, 0x01c6_2c2c);
        assert_eq!(GREEN, 0x014d_d44d);
        assert_eq!(BLUE, 0x0120_2088);
        assert_eq!(YELLOW, 0x01ff_ff00);
        assert_eq!(MAGENTA, 0x01ff_00ff);
        assert_eq!(CYAN, 0x015a_cccc);
        assert_eq!(WHITE_RGB, 0xff_ffff);
        assert_eq!(GRAY_RGB, 0x99_9999);
    }

    #[test]
    fn to_string_code_round_trips_through_parse() {
        for color in [
            WHITE,
            GRAY,
            DARK_GRAY,
            BLACK,
            RED,
            GREEN,
            BLUE,
            YELLOW,
            MAGENTA,
            CYAN,
        ] {
            let code = to_string_code(color);
            assert_eq!(code.chars().count(), 5);
            assert_eq!(code.chars().next(), Some(COLOR_CHAR));
            assert_eq!(parse_code(&code), Some(color));
            assert_eq!(
                parse_code(&code[COLOR_CHAR.len_utf8()..]),
                Some(color)
            );
        }
    }

    #[test]
    fn rgb_clamps_and_packs_readable_digits() {
        assert_eq!(rgb(255, 255, 255), 555);
        assert_eq!(rgb(0, 0, 0), 0);
        assert_eq!(rgb(50, 100, 150), 123);
        // limit(-1)=0, limit(300)=250 → 5, limit(49)=0 → 050
        assert_eq!(rgb(-1, 300, 49), 50);
    }

    #[test]
    fn decode_unget_and_separate() {
        assert_eq!(decode_rgb(215), [5, 5, 5]);
        assert_eq!(decode_rgb(0), [0, 0, 0]);
        assert_eq!(un_get(215), 555);
        assert_eq!(separate_encoded_sprite(RED), [1, 198, 44, 44]);
    }

    #[test]
    fn lightness_of_black_and_white() {
        assert_eq!(lightness_from_rgb(0x000000), 0.0);
        assert_eq!(lightness_from_rgb(0xffffff), 1.0);
        // Java formula (max+min)/510; 0x808080 → 256/510 ≈ 0.50196
        assert!((lightness_from_rgb(0x808080) - 256.0 / 510.0).abs() < 1e-6);
    }
}
