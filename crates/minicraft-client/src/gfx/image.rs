use std::io::Cursor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pixels: Vec<u32>,
    alpha: Vec<u8>,
}

impl Image {
    /// Fixed-size top-left extraction used by Java's resource-sheet loader.
    /// Reject undersized images instead of silently manufacturing pixels.
    pub fn crop(&self, width: usize, height: usize) -> Result<Self, String> {
        if width == 0 || height == 0 || width > self.width || height > self.height {
            return Err(format!(
                "cannot extract {width}x{height} from {}x{}",
                self.width, self.height
            ));
        }
        let mut pixels = Vec::with_capacity(width * height);
        let mut alpha = Vec::with_capacity(width * height);
        for row in 0..height {
            let start = row * self.width;
            pixels.extend_from_slice(&self.pixels[start..start + width]);
            alpha.extend_from_slice(&self.alpha[start..start + width]);
        }
        Ok(Self {
            width,
            height,
            pixels,
            alpha,
        })
    }

    pub fn from_png(bytes: &[u8]) -> Result<Self, String> {
        const MAX_PIXELS: usize = 4 * 1024 * 1024;
        const MAX_DECODE_BYTES: usize = 32 * 1024 * 1024;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("PNG exceeds 16 MiB input limit".to_owned());
        }
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_limits(png::Limits {
            bytes: MAX_DECODE_BYTES,
        });
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
        let header = reader.info();
        let count = (header.width as usize)
            .checked_mul(header.height as usize)
            .ok_or("PNG dimensions overflow")?;
        if header.width == 0
            || header.height == 0
            || header.width > 4096
            || header.height > 4096
            || count > MAX_PIXELS
            || reader.output_buffer_size() > MAX_DECODE_BYTES
        {
            return Err(format!(
                "PNG dimensions exceed decode limits: {}x{}",
                header.width, header.height
            ));
        }
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|error| error.to_string())?;
        let source = &buffer[..info.buffer_size()];
        let mut pixels = Vec::with_capacity(count);
        let mut alpha = Vec::with_capacity(count);

        match info.color_type {
            png::ColorType::Rgba => {
                for pixel in source.as_chunks::<4>().0 {
                    pixels.push(rgb(pixel[0], pixel[1], pixel[2]));
                    alpha.push(pixel[3]);
                }
            }
            png::ColorType::Rgb => {
                for pixel in source.as_chunks::<3>().0 {
                    pixels.push(rgb(pixel[0], pixel[1], pixel[2]));
                    alpha.push(255);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for pixel in source.as_chunks::<2>().0 {
                    pixels.push(rgb(pixel[0], pixel[0], pixel[0]));
                    alpha.push(pixel[1]);
                }
            }
            png::ColorType::Grayscale => {
                for value in source {
                    pixels.push(rgb(*value, *value, *value));
                    alpha.push(255);
                }
            }
            png::ColorType::Indexed => {
                return Err("indexed PNG was not expanded by the decoder".to_owned());
            }
        }

        Ok(Self {
            width: info.width as usize,
            height: info.height as usize,
            pixels,
            alpha,
        })
    }

    /// Opaque image from packed 0xRRGGBB pixels (unit tests and tools).
    pub fn from_rgb(width: usize, height: usize, pixels: Vec<u32>) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("image dimensions must be positive".to_owned());
        }
        let count = width
            .checked_mul(height)
            .ok_or("image dimensions overflow")?;
        if pixels.len() != count {
            return Err(format!(
                "expected {} pixels for {width}x{height}, got {}",
                count,
                pixels.len()
            ));
        }
        Ok(Self {
            width,
            height,
            pixels,
            alpha: vec![255; count],
        })
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<(u32, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = x + y * self.width;
        Some((self.pixels[index], self.alpha[index]))
    }
}

const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_limits_reject_huge_headers_before_frame_allocation() {
        let mut bytes = Vec::new();
        let encoder = png::Encoder::new(&mut bytes, 100_000, 100_000);
        let writer = encoder.write_header().unwrap();
        drop(writer);
        assert!(Image::from_png(&bytes).is_err());
        assert!(Image::from_png(b"").is_err());
        assert!(Image::from_rgb(usize::MAX, 2, vec![]).is_err());
        assert!(Image::from_png(include_bytes!("../../../../assets/pack.png")).is_ok());
    }
}
