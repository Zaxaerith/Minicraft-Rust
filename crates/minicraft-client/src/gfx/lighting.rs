//! Java LightOverlay: gray-buffer radial lights, Bayer dither, screen composite.
use std::collections::BTreeMap;

pub const MAX_DARK: i32 = 128;
const DAY_QUARTER: i32 = 16_200;

/// Same-coordinate last write wins; radii are not max-combined.
#[derive(Default)]
pub struct LightSources {
    points: BTreeMap<(i32, i32), i32>,
}

pub struct LightShape {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
    pub filled_square: bool,
}

impl LightSources {
    pub fn insert(&mut self, x: i32, y: i32, radius: i32) {
        self.points.insert((x, y), radius);
    }

    pub fn values(&self) -> impl Iterator<Item = (i32, i32, i32)> + '_ {
        self.points.iter().map(|(&(x, y), &r)| (x, y, r))
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Java substitutes a 16×16 white square when all angular octants have
    /// a nearby source at least as large. Classification uses the whole map.
    pub fn drain_shapes(&mut self) -> Vec<LightShape> {
        let shapes = self
            .points
            .iter()
            .map(|(&(x, y), &radius)| {
                let mut surrounds = [false; 8];
                for dx in -16..=16 {
                    for dy in -16..=16 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if self
                            .points
                            .get(&(x.wrapping_add(dx), y.wrapping_add(dy)))
                            .is_some_and(|&other| radius <= other)
                        {
                            let mut theta = f64::from(dy).atan2(f64::from(dx));
                            if theta < 0.0 {
                                theta += 2.0 * std::f64::consts::PI;
                            }
                            surrounds[(theta * 4.0 / std::f64::consts::PI) as usize] = true;
                        }
                    }
                }
                LightShape {
                    x,
                    y,
                    radius,
                    filled_square: surrounds.into_iter().all(|v| v),
                }
            })
            .collect();
        self.points.clear();
        shapes
    }
}

const DITHER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

pub fn overlay_alpha(grade: u8, x: i32, y: i32, camera_x: i32, camera_y: i32) -> u8 {
    let index = (x.wrapping_add(camera_x) & 3) + (y.wrapping_add(camera_y) & 3) * 4;
    if grade / 10 <= DITHER[index as usize] {
        255 - grade
    } else {
        0
    }
}

/// BigDecimal's 255 - floor((i/50)^4 * 255), without floating-point drift.
pub fn gradient_stops() -> [u8; 51] {
    std::array::from_fn(|i| 255 - ((i as u64).pow(4) * 255 / 50_u64.pow(4)) as u8)
}

/// Java LightOverlay.getOverlayOpacity: caves are fully dark; elsewhere darkFactor/160.
/// `level_index` uses Java's levels-array index (surface = 3).
pub fn overlay_opacity(level_index: usize, dark_factor: f64) -> f64 {
    if level_index < 3 {
        1.0
    } else {
        dark_factor / 160.0
    }
}

/// Java Screen.overlay darkFactor for surface/sky/dungeon, from day tick.
pub fn dark_factor(level_index: usize, day_tick: i32, past_day_one: bool) -> f64 {
    if !(3..5).contains(&level_index) {
        return if level_index >= 5 {
            f64::from(MAX_DARK)
        } else {
            0.0
        };
    }
    let rel_time = f64::from(day_tick % DAY_QUARTER) / f64::from(DAY_QUARTER);
    let mut dark = match day_tick / DAY_QUARTER {
        0 => {
            if past_day_one {
                (1.0 - rel_time) * f64::from(MAX_DARK)
            } else {
                0.0
            }
        }
        1 => 0.0,
        2 => rel_time * f64::from(MAX_DARK),
        _ => f64::from(MAX_DARK),
    };
    if level_index > 3 {
        dark -= dark.min(10.0);
    }
    dark
}

/// Java World.lvlIdx inverse for the fixed six-level order.
pub fn java_level_index(depth: i8) -> usize {
    match depth {
        -3 => 0,
        -2 => 1,
        -1 => 2,
        0 => 3,
        1 => 4,
        -4 => 5,
        _ => 3,
    }
}

/// Renderer skips lighting on surface during the day phase of the calendar.
pub fn should_skip_overlay(level_index: usize, day_tick: i32) -> bool {
    level_index == 3 && (DAY_QUARTER..=DAY_QUARTER * 2).contains(&day_tick)
}

/// Screen-space light buffer matching LightOverlay's TYPE_BYTE_GRAY + dither.
pub struct LightOverlay {
    width: usize,
    height: usize,
    gray: Vec<u8>,
    stops: [u8; 51],
}

impl LightOverlay {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            gray: vec![0; width * height],
            stops: gradient_stops(),
        }
    }

    fn clear_gray(&mut self) {
        self.gray.fill(0);
    }

    fn composite_white(&mut self, index: usize, alpha: u8) {
        // SrcOver white onto black gray: result = alpha.
        let prev = self.gray[index];
        let a = u32::from(alpha);
        let inv = 255 - a;
        self.gray[index] = ((255 * a + u32::from(prev) * inv) / 255) as u8;
    }

    fn fill_square(&mut self, cx: i32, cy: i32, half: i32) {
        let left = (cx - half).max(0);
        let top = (cy - half).max(0);
        let right = (cx + half).min(self.width as i32);
        let bottom = (cy + half).min(self.height as i32);
        for y in top..bottom {
            for x in left..right {
                self.composite_white(x as usize + y as usize * self.width, 255);
            }
        }
    }

    /// RadialGradientPaint white with quartic alpha onto the gray buffer.
    fn fill_radial(&mut self, cx: i32, cy: i32, radius: i32) {
        if radius <= 0 {
            return;
        }
        let r = f64::from(radius);
        let left = (cx - radius).max(0);
        let top = (cy - radius).max(0);
        let right = (cx + radius + 1).min(self.width as i32);
        let bottom = (cy + radius + 1).min(self.height as i32);
        for y in top..bottom {
            for x in left..right {
                let dx = f64::from(x - cx);
                let dy = f64::from(y - cy);
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > r {
                    continue;
                }
                let fraction = (dist / r).clamp(0.0, 1.0);
                let scaled = fraction * 50.0;
                let i0 = (scaled.floor() as usize).min(50);
                let i1 = (i0 + 1).min(50);
                let t = scaled - i0 as f64;
                let a0 = f64::from(self.stops[i0]);
                let a1 = f64::from(self.stops[i1]);
                let alpha = (a0 + (a1 - a0) * t).round() as u8;
                self.composite_white(x as usize + y as usize * self.width, alpha);
            }
        }
    }

    /// Render lights into the gray buffer, then dither into overlay alpha bytes.
    pub fn render(&mut self, shapes: &[LightShape], xa: i32, ya: i32) -> Vec<u8> {
        self.clear_gray();
        for shape in shapes {
            if shape.filled_square {
                self.fill_square(shape.x, shape.y, 8);
            } else {
                self.fill_radial(shape.x, shape.y, shape.radius);
            }
        }
        let mut overlay = vec![0u8; self.width * self.height];
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let i = x as usize + y as usize * self.width;
                let grade = self.gray[i];
                overlay[i] = overlay_alpha(grade, x, y, xa, ya);
            }
        }
        overlay
    }
}

/// Screen composite of the dark overlay after a 2% white lightening fill.
pub fn composite_overlay(pixels: &mut [u32], overlay_alpha: &[u8], opacity: f64) {
    let opacity = opacity.clamp(0.0, 1.0);
    let light = 0.02f64;
    for (pixel, &dark_alpha) in pixels.iter_mut().zip(overlay_alpha) {
        let mut r = ((*pixel >> 16) & 255) as f64;
        let mut g = ((*pixel >> 8) & 255) as f64;
        let mut b = (*pixel & 255) as f64;
        // SRC_OVER white at 0.02
        r = 255.0 * light + r * (1.0 - light);
        g = 255.0 * light + g * (1.0 - light);
        b = 255.0 * light + b * (1.0 - light);
        // SRC_OVER black overlay: effective alpha = dark_alpha/255 * opacity
        let dark = f64::from(dark_alpha) / 255.0 * opacity;
        let keep = 1.0 - dark;
        r *= keep;
        g *= keep;
        b *= keep;
        *pixel = ((r.round() as u32) << 16) | ((g.round() as u32) << 8) | (b.round() as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_stops_match_quartic_formula() {
        let stops = gradient_stops();
        assert_eq!(stops[0], 255);
        assert_eq!(stops[50], 0);
        // Java BigDecimal.intValue() truncates; (i/50)^4*255 stays 0 until i=13.
        assert_eq!(stops[1], 255);
        assert_eq!(stops[12], 255);
        assert_eq!(stops[13], 254);
        assert!(stops[25] < stops[13]);
        assert!(
            stops
                .windows(2)
                .all(|pair| pair[0] >= pair[1]),
            "stops must be non-increasing"
        );
    }

    #[test]
    fn caves_are_fully_opaque() {
        assert_eq!(overlay_opacity(0, 0.0), 1.0);
        assert_eq!(overlay_opacity(2, 64.0), 1.0);
    }

    #[test]
    fn surface_dark_factor_uses_maxdark_over_160() {
        let night = dark_factor(3, 48_600, true);
        assert!((night - 128.0).abs() < f64::EPSILON);
        assert!((overlay_opacity(3, night) - 0.8).abs() < 1e-9);
        let day = dark_factor(3, 20_000, true);
        assert_eq!(day, 0.0);
    }

    #[test]
    fn sky_lighter_than_surface_by_ten() {
        let surface = dark_factor(3, 48_600, true);
        let sky = dark_factor(4, 48_600, true);
        assert!((surface - sky - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn first_morning_stays_lit_without_past_day_one() {
        assert_eq!(dark_factor(3, 100, false), 0.0);
        let lit = dark_factor(3, 100, true);
        assert!(lit > 100.0);
    }

    #[test]
    fn surface_day_skips_overlay() {
        assert!(should_skip_overlay(3, DAY_QUARTER));
        assert!(should_skip_overlay(3, DAY_QUARTER * 2));
        assert!(!should_skip_overlay(3, DAY_QUARTER - 1));
        assert!(!should_skip_overlay(3, DAY_QUARTER * 2 + 1));
        assert!(!should_skip_overlay(2, DAY_QUARTER + 1));
    }

    #[test]
    fn depth_maps_to_java_level_index() {
        assert_eq!(java_level_index(-3), 0);
        assert_eq!(java_level_index(0), 3);
        assert_eq!(java_level_index(1), 4);
        assert_eq!(java_level_index(-4), 5);
    }

    #[test]
    fn radial_center_is_lit_and_far_pixels_stay_dark() {
        let mut overlay = LightOverlay::new(64, 64);
        let shapes = vec![LightShape {
            x: 32,
            y: 32,
            radius: 16,
            filled_square: false,
        }];
        let alphas = overlay.render(&shapes, 0, 0);
        // Center grade is near white → overlay alpha near 0.
        assert!(alphas[32 + 32 * 64] < 32);
        // Far corner remains fully dark-capable.
        assert_eq!(alphas[0], 255);
    }

    #[test]
    fn surrounded_sources_use_filled_square() {
        let mut sources = LightSources::default();
        // Ring of equal-radius lights around the center so every octant has one.
        for &(dx, dy) in &[
            (10, 0),
            (7, 7),
            (0, 10),
            (-7, 7),
            (-10, 0),
            (-7, -7),
            (0, -10),
            (7, -7),
        ] {
            sources.insert(dx, dy, 12);
        }
        sources.insert(0, 0, 12);
        let shapes = sources.drain_shapes();
        let center = shapes.iter().find(|s| s.x == 0 && s.y == 0).unwrap();
        assert!(center.filled_square);
    }

    #[test]
    fn composite_lightens_then_darkens() {
        let mut pixels = vec![0x40_40_40u32];
        let mut overlay = vec![0u8];
        composite_overlay(&mut pixels, &overlay, 1.0);
        assert!(pixels[0] > 0x40_40_40);
        overlay[0] = 255;
        composite_overlay(&mut pixels, &overlay, 1.0);
        assert_eq!(pixels[0], 0);
    }
}
