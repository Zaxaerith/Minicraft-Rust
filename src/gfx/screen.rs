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
                if let Some((color, alpha)) = image.pixel(source_x + source_offset, source_y + yy)
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

    pub fn centered_text(&mut self, font: &Image, message: &str, y: i32) {
        let x = (WIDTH as i32 - message.chars().count() as i32 * 8) / 2;
        self.text(font, message, x, y);
    }

    pub fn darken_outside(&mut self, center_x: i32, center_y: i32, radius: i32, alpha: u8) {
        if alpha == 0 || radius <= 0 {
            return;
        }
        let inner = radius / 2;
        let inner_squared = inner * inner;
        let outer_squared = radius * radius;
        let span = (outer_squared - inner_squared).max(1);
        for y in 0..HEIGHT as i32 {
            for x in 0..WIDTH as i32 {
                let dx = x - center_x;
                let dy = y - center_y;
                let distance = dx * dx + dy * dy;
                let local_alpha = if distance <= inner_squared {
                    0
                } else if distance >= outer_squared {
                    alpha as i32
                } else {
                    alpha as i32 * (distance - inner_squared) / span
                } as u32;
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
