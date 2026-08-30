use super::Image;
use std::{fs::File, io::BufWriter, path::Path};

pub const WIDTH: usize = 288;
pub const HEIGHT: usize = 192;

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

pub struct Screen {
    pixels: Vec<u32>,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
        }
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        let left = x.max(0);
        let top = y.max(0);
        let right = (x + width).min(WIDTH as i32);
        let bottom = (y + height).min(HEIGHT as i32);
        for yy in top..bottom {
            let start = left as usize + yy as usize * WIDTH;
            self.pixels[start..start + (right - left) as usize].fill(color);
        }
    }

    pub fn frame(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        self.rect(x, y, width, 1, color);
        self.rect(x, y + height - 1, width, 1, color);
        self.rect(x, y, 1, height, color);
        self.rect(x + width - 1, y, 1, height, color);
    }

    pub fn blit(&mut self, image: &Image, x: i32, y: i32) {
        self.blit_region(image, x, y, 0, 0, image.width, image.height, false);
    }

    pub fn blit_tinted(&mut self, image: &Image, x: i32, y: i32, tint: u32) {
        self.blit_region_tinted(image, x, y, 0, 0, image.width, image.height, tint);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn blit_region(
        &mut self,
        image: &Image,
        x: i32,
        y: i32,
        source_x: usize,
        source_y: usize,
        width: usize,
        height: usize,
        flip_x: bool,
    ) {
        self.blit_region_transformed(
            image, source_x, source_y, width, height, x, y, flip_x, false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn blit_region_transformed(
        &mut self,
        image: &Image,
        source_x: usize,
        source_y: usize,
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        flip_x: bool,
        flip_y: bool,
    ) {
        for yy in 0..height {
            let destination_y = y + yy as i32;
            if !(0..HEIGHT as i32).contains(&destination_y) {
                continue;
            }
            for xx in 0..width {
                let destination_x = x + xx as i32;
                if !(0..WIDTH as i32).contains(&destination_x) {
                    continue;
                }
                let source_offset = if flip_x { width - xx - 1 } else { xx };
                let source_row = if flip_y { height - yy - 1 } else { yy };
                if let Some((color, alpha)) =
                    image.pixel(source_x + source_offset, source_y + source_row)
                    && alpha != 0
                {
                    let index = destination_x as usize + destination_y as usize * WIDTH;
                    self.pixels[index] = if alpha == 255 {
                        color
                    } else {
                        blend(self.pixels[index], color, alpha)
                    };
                }
            }
        }
    }

    pub fn text(&mut self, font: &Image, message: &str, x: i32, y: i32) {
        for (offset, character) in message.chars().enumerate() {
            if let Some(index) = FONT_CHARS
                .chars()
                .position(|candidate| candidate == character)
            {
                self.blit_region(
                    font,
                    x + offset as i32 * 8,
                    y,
                    index % 32 * 8,
                    index / 32 * 8,
                    8,
                    8,
                    false,
                );
            }
        }
    }

    pub fn text_colored(&mut self, font: &Image, message: &str, x: i32, y: i32, color: u32) {
        for (offset, character) in message.chars().enumerate() {
            if let Some(index) = FONT_CHARS
                .chars()
                .position(|candidate| candidate == character)
            {
                self.blit_region_tinted(
                    font,
                    x + offset as i32 * 8,
                    y,
                    index % 32 * 8,
                    index / 32 * 8,
                    8,
                    8,
                    color,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn blit_region_tinted(
        &mut self,
        image: &Image,
        x: i32,
        y: i32,
        source_x: usize,
        source_y: usize,
        width: usize,
        height: usize,
        tint: u32,
    ) {
        for yy in 0..height {
            let destination_y = y + yy as i32;
            if !(0..HEIGHT as i32).contains(&destination_y) {
                continue;
            }
            for xx in 0..width {
                let destination_x = x + xx as i32;
                if !(0..WIDTH as i32).contains(&destination_x) {
                    continue;
                }
                if let Some((source, alpha)) = image.pixel(source_x + xx, source_y + yy)
                    && alpha != 0
                {
                    let brightness =
                        (((source >> 16) & 255) + ((source >> 8) & 255) + (source & 255)) / 3;
                    let red = ((tint >> 16) & 255) * brightness / 255;
                    let green = ((tint >> 8) & 255) * brightness / 255;
                    let blue = (tint & 255) * brightness / 255;
                    let color = red << 16 | green << 8 | blue;
                    let index = destination_x as usize + destination_y as usize * WIDTH;
                    self.pixels[index] = if alpha == 255 {
                        color
                    } else {
                        blend(self.pixels[index], color, alpha)
                    };
                }
            }
        }
    }

    pub fn centered_text(&mut self, font: &Image, message: &str, y: i32) {
        let x = (WIDTH as i32 - message.chars().count() as i32 * 8) / 2;
        self.text(font, message, x, y);
    }

    pub fn darken_with_lights(&mut self, lights: &[(i32, i32, i32)], alpha: u8) {
        if alpha == 0 || lights.is_empty() {
            return;
        }
        for y in 0..HEIGHT as i32 {
            for x in 0..WIDTH as i32 {
                let mut local_alpha = alpha as i32;
                for &(center_x, center_y, radius) in lights {
                    if radius <= 0 {
                        continue;
                    }
                    let inner = radius / 2;
                    let inner_squared = inner * inner;
                    let outer_squared = radius * radius;
                    let dx = x - center_x;
                    let dy = y - center_y;
                    let distance = dx * dx + dy * dy;
                    let candidate = if distance <= inner_squared {
                        0
                    } else if distance >= outer_squared {
                        alpha as i32
                    } else {
                        alpha as i32 * (distance - inner_squared)
                            / (outer_squared - inner_squared).max(1)
                    };
                    local_alpha = local_alpha.min(candidate);
                }
                let local_alpha = local_alpha as u32;
                if local_alpha == 0 {
                    continue;
                }
                let index = x as usize + y as usize * WIDTH;
                let color = self.pixels[index];
                let multiplier = 255 - local_alpha;
                let red = ((color >> 16) & 255) * multiplier / 255;
                let green = ((color >> 8) & 255) * multiplier / 255;
                let blue = (color & 255) * multiplier / 255;
                self.pixels[index] = red << 16 | green << 8 | blue;
            }
        }
    }

    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        let file = File::create(path)
            .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), WIDTH as u32, HEIGHT as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        let mut bytes = Vec::with_capacity(self.pixels.len() * 3);
        for color in &self.pixels {
            bytes.extend_from_slice(&[(color >> 16) as u8, (color >> 8) as u8, *color as u8]);
        }
        writer
            .write_image_data(&bytes)
            .map_err(|error| error.to_string())
    }
}

fn blend(background: u32, foreground: u32, alpha: u8) -> u32 {
    let alpha = alpha as u32;
    let inverse = 255 - alpha;
    let red = (((foreground >> 16) & 255) * alpha + ((background >> 16) & 255) * inverse) / 255;
    let green = (((foreground >> 8) & 255) * alpha + ((background >> 8) & 255) * inverse) / 255;
    let blue = ((foreground & 255) * alpha + (background & 255) * inverse) / 255;
    (red << 16) | (green << 8) | blue
}

#[cfg(test)]
mod tests {
    use super::{HEIGHT, Screen, WIDTH};

    #[test]
    fn multiple_light_sources_preserve_each_lit_center() {
        let mut screen = Screen::new();
        screen.clear(0xFF_FF_FF);
        screen.darken_with_lights(&[(40, 40, 24), (240, 140, 32)], 200);
        assert_eq!(screen.pixels[40 + 40 * WIDTH], 0xFF_FF_FF);
        assert_eq!(screen.pixels[240 + 140 * WIDTH], 0xFF_FF_FF);
        assert!(screen.pixels[(WIDTH / 2) + (HEIGHT / 2) * WIDTH] < 0x80_80_80);
    }
}
