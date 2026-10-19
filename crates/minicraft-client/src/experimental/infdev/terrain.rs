//! Original Infdev surface biomes, with explicit deterministic stair extension.
//! Scale/channel arithmetic and float weighting follow 42ba9192. No entities.
use super::{Cell, Ground};
use crate::progression::Simplex;

pub mod layers;
pub use layers::{
    LayerComparison, LayerSample, compare_layer_observation, generate_layer, sample_layer,
};

const SCALES: [f64; 10] = [1., 4., 8., 16., 32., 64., 128., 512., 2048., 8192.];
const TEMP: [f64; 10] = [0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 0.05, 0.04, 0.01];
const HEIGHT: [f64; 10] = [0.005, 0.01, 0.02, 0.01, 0.02, 0.05, 0.1, 0.2, 0.7, 0.2];
const HUMID: [f64; 10] = [0.02, 0.04, 0.07, 0.1, 0.4, 0.3, 0.1, 0.05, 0.02, 0.01];

/// The observed original Java HashMap traversal order, including strict > ties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Biome {
    Rock,
    Forest,
    Ocean,
    River,
    Desert,
    RiverBank,
    Surface,
}
pub const BIOMES: [Biome; 7] = [
    Biome::Rock,
    Biome::Forest,
    Biome::Ocean,
    Biome::River,
    Biome::Desert,
    Biome::RiverBank,
    Biome::Surface,
];

pub struct Sample {
    pub cell: Cell,
    pub biome: Biome,
    /// Original Java's System.nanoTime random may choose Rock or Stairs here.
    pub time_random_eligible: bool,
}

fn hash(seed: i64, x: i64, y: i64) -> u64 {
    let mut z =
        (seed as u64) ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (y as u64).rotate_left(32);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

pub fn sample(seed: i64, x: i64, y: i64) -> Sample {
    let mut s = Simplex::infdev(seed);
    sample_with(&mut s, seed, x, y)
}

fn sample_with(s: &mut Simplex, seed: i64, x: i64, y: i64) -> Sample {
    let mut n = [[0.; 10]; 10];
    for scale in 0..10 {
        for channel in [0, 1, 2, 7, 8, 9] {
            // Java uses signed int offsets. Preserve its numeric domain; the
            // extended i64 address space has this explicit 2^32 noise period.
            n[scale][channel] = f64::from(s.noise3(
                f64::from(x as i32) / SCALES[scale],
                f64::from(y as i32) / SCALES[scale],
                (channel + scale * 10) as f64 * 100.,
            ));
        }
    }
    let octave = |layer: usize, weights: &[f64; 10]| {
        (0..10).fold(0., |sum, i| sum + n[i][layer] * weights[i])
    };
    let h = octave(8, &HEIGHT);
    let t = octave(9, &TEMP) as f32;
    let humidity = octave(7, &HUMID) as f32;
    let regular = |temp: f32, height: f32, wet: f32, rarity: f32| {
        let x = t - temp;
        let y = h as f32 - height;
        let z = humidity - wet;
        rarity / (x * x + y * y + z * z)
    };
    let a = h as f32 * 3. - 1.;
    let rock = (0..12).fold(a, |v, _| v * a);
    let d = (n[6][9] - n[6][8]).abs() as f32;
    let weights = [
        rock,
        regular(-0.1, 0., 0.5, 1.),
        (-h * 4.).powi(5) as f32,
        (0.5_f32 / d).min(10.),
        regular(0.3, 0., -0.6, 1.),
        (0.55_f32 / d).min(3.),
        regular(-0.25, 0., 0.3, 1.),
    ];
    let mut selected = 0;
    let mut best = f32::NEG_INFINITY;
    for (i, &weight) in weights.iter().enumerate() {
        if weight > best {
            selected = i;
            best = weight;
        }
    }
    let biome = BIOMES[selected];
    let v = (n[5][0] - n[5][1]).abs();
    let m = ((n[3][0] - n[3][1]).abs() - n[3][2]).abs();
    let (mut ground, data, eligible, probability) = match biome {
        Biome::Rock => (Ground::Rock, 0, h > 0.35, 0.002),
        Biome::Ocean | Biome::River => (Ground::Water, 0, false, 0.),
        Biome::RiverBank => (Ground::Sand, 0, false, 0.),
        Biome::Forest => (
            if n[0][0] < 0.6 {
                Ground::Tree
            } else {
                Ground::Grass
            },
            0,
            false,
            0.,
        ),
        Biome::Desert => {
            if v > 0.5 && m < 0.5 {
                (Ground::Rock, 0, v > 0.65 && m < 0.35, 0.005)
            } else {
                (
                    if n[2][0] < -0.25 && n[0][1] < -0.4 {
                        Ground::Cactus
                    } else {
                        Ground::Sand
                    },
                    0,
                    false,
                    0.,
                )
            }
        }
        Biome::Surface => {
            if v > 0.75 && m < 0.35 {
                (Ground::Rock, 0, v > 0.85 && m < 0.25, 0.005)
            } else if n[3][2] < -0.6 && n[0][0] < 0.4 {
                (Ground::Tree, 0, false, 0.)
            } else if n[4][2] + n[2][0] < -0.6 && n[0][1] < -0.4 {
                (
                    Ground::Flower,
                    ((n[2][1] * 24.).abs() as i32 % 12) as u16,
                    false,
                    0.,
                )
            } else {
                (Ground::Grass, 0, false, 0.)
            }
        }
    };
    if eligible && (hash(seed, x, y) >> 11) as f64 * (1. / 9007199254740992.) < probability {
        ground = Ground::StairsDown;
    }
    Sample {
        cell: Cell { ground, data },
        biome,
        time_random_eligible: eligible,
    }
}

pub fn generate(seed: i64, cx: i64, cy: i64) -> Vec<Cell> {
    let mut noise = Simplex::infdev(seed);
    let mut cells = Vec::with_capacity(4096);
    for y in 0..64 {
        for x in 0..64 {
            cells.push(
                sample_with(
                    &mut noise,
                    seed,
                    cx.wrapping_mul(64).wrapping_add(x),
                    cy.wrapping_mul(64).wrapping_add(y),
                )
                .cell,
            );
        }
    }
    cells
}

#[derive(Debug, Default)]
pub struct Comparison {
    pub cells: usize,
    pub time_eligible: usize,
    pub time_choice_differences: usize,
}

/// Compare every retained Java field. Eligible stair cells are NOT claimed
/// equal: report actual differing choices and validate their original domain.
pub fn compare_observation(
    bytes: &[u8],
    seed: i64,
    cx: i64,
    cy: i64,
) -> Result<Comparison, String> {
    if bytes.len() != 4096 * 5 {
        return Err("invalid Java terrain observation size".into());
    }
    let mut result = Comparison::default();
    let mut noise = Simplex::infdev(seed);
    for (i, row) in bytes.as_chunks::<5>().0.iter().enumerate() {
        let x = cx.wrapping_mul(64).wrapping_add((i % 64) as i64);
        let y = cy.wrapping_mul(64).wrapping_add((i / 64) as i64);
        let s = sample_with(&mut noise, seed, x, y);
        let data = u16::from_be_bytes([row[1], row[2]]);
        if row[3] != s.biome as u8
            || row[4] != u8::from(s.time_random_eligible)
            || data != s.cell.data
        {
            return Err(format!(
                "Java biome/data/eligibility differs seed={seed} chunk=({cx},{cy}) first=({x},{y}) Java={row:?} Rust={:?}/data{}/eligible{}",
                s.biome, s.cell.data, s.time_random_eligible
            ));
        }
        if s.time_random_eligible {
            result.time_eligible += 1;
            if ![Ground::Rock as u8, Ground::StairsDown as u8].contains(&row[0])
                || !matches!(s.cell.ground, Ground::Rock | Ground::StairsDown)
            {
                return Err(format!("invalid time-random tile domain at ({x},{y})"));
            }
            if row[0] != s.cell.ground as u8 {
                result.time_choice_differences += 1;
            }
        } else if row[0] != s.cell.ground as u8 {
            return Err(format!(
                "Java deterministic tile differs seed={seed} first=({x},{y}) Java={} Rust={:?}",
                row[0], s.cell.ground
            ));
        }
        result.cells += 1;
    }
    Ok(result)
}
