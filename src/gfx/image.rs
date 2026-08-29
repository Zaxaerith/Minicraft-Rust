use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pixels: Vec<u32>,
    alpha: Vec<u8>,
}

impl Image {
    pub fn from_png(bytes: &[u8]) -> Result<Self, String> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|error| error.to_string())?;
        let source = &buffer[..info.buffer_size()];
        let count = info.width as usize * info.height as usize;
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
