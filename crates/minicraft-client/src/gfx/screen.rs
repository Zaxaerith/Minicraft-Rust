use super::Image;
use std::{fs::File, io::BufWriter, path::Path};

pub const WIDTH: usize = 288;
pub const HEIGHT: usize = 192;

use super::font;
use super::lighting::{self, LightOverlay, LightSources};

pub struct Screen {
    pixels: Vec<u32>,
    lights: LightSources,
    light_overlay: LightOverlay,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
            lights: LightSources::default(),
            light_overlay: LightOverlay::new(WIDTH, HEIGHT),
        }
    }

    /// Java Screen.renderLight: store a light at screen coordinates.
    pub fn render_light(&mut self, x: i32, y: i32, radius: i32) {
        if radius > 0 {
            self.lights.insert(x, y, radius);
        }
    }

    /// Java Screen.overlay: darken with collected lights and clear the queue.
    pub fn overlay(
        &mut self,
        level_index: usize,
        xa: i32,
        ya: i32,
        day_tick: i32,
        past_day_one: bool,
    ) {
        let dark = lighting::dark_factor(level_index, day_tick, past_day_one);
        let opacity = lighting::overlay_opacity(level_index, dark);
        let shapes = self.lights.drain_shapes();
        let alphas = self.light_overlay.render(&shapes, xa, ya);
        lighting::composite_overlay(&mut self.pixels, &alphas, opacity);
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let left = x.max(0);
        let top = y.max(0);
        let right = x.saturating_add(width).min(WIDTH as i32);
        let bottom = y.saturating_add(height).min(HEIGHT as i32);
        if right <= left || bottom <= top {
            return;
        }
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
        self.render_sprite(
            image,
            x,
            y,
            super::sprite::SpriteRender {
                source_x,
                source_y,
                width,
                height,
                mirror_x: flip_x,
                mirror_y: flip_y,
                ..Default::default()
            },
        );
    }

    /// Like blit_region but supports Java LinkedSprite color / fullBright overrides.
    #[allow(clippy::too_many_arguments)]
    pub fn blit_region_override(
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
        color: u32,
        full_bright: bool,
    ) {
        self.render_sprite(
            image,
            x,
            y,
            super::sprite::SpriteRender {
                source_x,
                source_y,
                width,
                height,
                mirror_x: flip_x,
                mirror_y: flip_y,
                color,
                full_bright,
                ..Default::default()
            },
        );
    }

    pub fn render_sprite(
        &mut self,
        image: &Image,
        x: i32,
        y: i32,
        sprite: super::sprite::SpriteRender,
    ) {
        for yy in 0..sprite.height {
            let dy = y as i64 + yy as i64;
            if !(0..HEIGHT as i64).contains(&dy) {
                continue;
            }
            for xx in 0..sprite.width {
                let dx = x as i64 + xx as i64;
                if !(0..WIDTH as i64).contains(&dx) {
                    continue;
                }
                let sx = if sprite.mirror_x {
                    sprite.width - xx - 1
                } else {
                    xx
                };
                let sy = if sprite.mirror_y {
                    sprite.height - yy - 1
                } else {
                    yy
                };
                if let Some((source, alpha)) =
                    image.pixel(sprite.source_x + sx, sprite.source_y + sy)
                    && alpha != 0
                {
                    // MinicraftImage reduces PNG alpha to a binary visibility flag.
                    self.pixels[dx as usize + dy as usize * WIDTH] = sprite.pixel_color(source);
                }
            }
        }
    }

    pub fn text(&mut self, font: &Image, message: &str, x: i32, y: i32) {
        font::draw(self, font, message, x, y, None);
    }

    pub fn text_colored(&mut self, font: &Image, message: &str, x: i32, y: i32, color: u32) {
        font::draw(self, font, message, x, y, Some(color));
    }

    /// Java Screen whiteTint replaces only white pixels, not every shade.
    pub fn blit_font_glyph(&mut self, image: &Image, x: i32, y: i32, index: usize, tint: u32) {
        self.render_sprite(
            image,
            x,
            y,
            super::sprite::SpriteRender {
                source_x: index % 32 * 8,
                source_y: index / 32 * 8,
                width: 8,
                height: 8,
                white_tint: Some(tint),
                ..Default::default()
            },
        );
    }

    pub fn centered_text(&mut self, font: &Image, message: &str, y: i32) {
        font::draw_centered(self, font, message, y, 0xFF_FF_FF);
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

#[cfg(test)]
mod tests {
    use super::{HEIGHT, Screen, WIDTH};

    #[test]
    fn multiple_light_sources_preserve_each_lit_center() {
        let mut screen = Screen::new();
        screen.clear(0xFF_FF_FF);
        screen.render_light(40, 40, 24);
        screen.render_light(240, 140, 32);
        // Caves: full opacity darkness away from lights.
        screen.overlay(0, 0, 0, 0, false);
        let center_a = screen.pixels[40 + 40 * WIDTH];
        let center_b = screen.pixels[240 + 140 * WIDTH];
        let mid = screen.pixels[(WIDTH / 2) + (HEIGHT / 2) * WIDTH];
        assert!(center_a > mid);
        assert!(center_b > mid);
        assert!(mid < 0x40_40_40);
        let _ = HEIGHT;
    }
}
