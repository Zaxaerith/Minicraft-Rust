//! Phase4A terrain: original Infdev LevelNoise height, deliberately simplified
//! material selection. This is NOT Java biome/structure parity.
use crate::progression::Simplex;

use super::{CHUNK_SIZE, Cell, ChunkCoord, Ground};

/// Original LevelNoise.getHeight: ten scales, layer 8 of ten, z offset zero.
/// Java promotes each Simplex float to double before weighting, in this order.
pub fn height(seed: i64, x: i32, y: i32) -> f64 {
    let mut noise = Simplex::infdev(seed);
    [
        (1., 0.005),
        (4., 0.01),
        (8., 0.02),
        (16., 0.01),
        (32., 0.02),
        (64., 0.05),
        (128., 0.1),
        (512., 0.2),
        (2048., 0.7),
        (8192., 0.2),
    ]
    .into_iter()
    .enumerate()
    .fold(0., |sum, (index, (scale, weight))| {
        sum + f64::from(noise.noise3(
            f64::from(x) / scale,
            f64::from(y) / scale,
            (8 + index * 10) as f64 * 100.,
        )) * weight
    })
}

pub(super) fn generate(seed: i64, coord: ChunkCoord) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);
    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            // Validated chunk coordinates span precisely the i32 tile domain.
            let x = coord.x * CHUNK_SIZE as i32 + x as i32;
            let y = coord.y * CHUNK_SIZE as i32 + y as i32;
            let h = height(seed, x, y);
            let ground = if h < -0.25 {
                Ground::Water
            } else if h < -0.18 {
                Ground::Sand
            } else if h > 0.3 {
                Ground::Rock
            } else {
                Ground::Grass
            };
            cells.push(Cell { ground, data: 0 });
        }
    }
    cells
}
