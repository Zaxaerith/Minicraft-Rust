//! Original Infdev cave/dungeon base terrain. The Java time-seeded stair
//! choices use a separate coordinate stream here. Random ornate lava pools
//! are observed by the Oracle but deferred: their cross-chunk writes depend
//! on generation order in the original implementation.
use super::{Cell, Ground, Simplex};

pub struct LayerSample {
    pub cell: Cell,
    pub time_random_eligible: bool,
}

pub fn sample_layer(seed: i64, depth: i8, x: i64, y: i64) -> Result<LayerSample, String> {
    if depth == 0 {
        let sample = super::sample(seed, x, y);
        return Ok(LayerSample {
            cell: sample.cell,
            time_random_eligible: sample.time_random_eligible,
        });
    }
    if !(-4..=-1).contains(&depth) {
        return Err("Infdev exploration supports depths -4 through 0".into());
    }
    Ok(sample_with(&mut Simplex::infdev(seed), seed, depth, x, y))
}

fn sample_with(noise: &mut Simplex, seed: i64, depth: i8, x: i64, y: i64) -> LayerSample {
    // LevelGen.advanceChunk constructs LevelNoise with zOffset=0 on every
    // depth. The depth selects a biome, not a new noise offset or level seed.
    let mut n16 = [0.; 10];
    for (channel, value) in n16
        .iter_mut()
        .enumerate()
        .take(if depth == -4 { 3 } else { 10 })
    {
        *value = f64::from(noise.noise3(
            f64::from(x as i32) / 16.,
            f64::from(y as i32) / 16.,
            (channel + 30) as f64 * 100.,
        ));
    }
    let stair_random = |probability| {
        let stream = seed ^ i64::from(depth).wrapping_mul(0x517c_c1b7_2722_0a95);
        (super::hash(stream, x, y) >> 11) as f64 * (1. / 9007199254740992.) < probability
    };
    let (ground, eligible) = if depth == -4 {
        let val = (n16[0] - n16[1]).abs();
        if val < 0.15 {
            (Ground::ObsidianWall, false)
        } else if val < 0.18 {
            (Ground::Lava, false)
        } else if stair_random(0.0001) {
            (Ground::StairsDown, true)
        } else {
            let selection = (n16[2] * 9.).abs() as i32 % 3;
            (
                match selection {
                    0 => Ground::Obsidian,
                    1 => Ground::RawObsidian,
                    _ => Ground::Dirt,
                },
                true,
            )
        }
    } else {
        let mut n32 = [0.; 4];
        for channel in [0, 1, 3] {
            n32[channel] = f64::from(noise.noise3(
                f64::from(x as i32) / 32.,
                f64::from(y as i32) / 32.,
                (channel + 40) as f64 * 100.,
            ));
        }
        let val = (n32[0] - n32[1]).abs();
        let mval = ((n16[0] - n16[1]).abs() - n16[2]).abs();
        let nval = ((n16[3] - n16[4]).abs() - n16[5]).abs();
        let wval = ((n16[6] - n16[7]).abs() - n16[8]).abs();
        let level = usize::from(depth.unsigned_abs() - 1);
        if val > 0.25 && wval < [0.35, 0.45, 0.45][level] {
            (
                if depth == -3 {
                    Ground::Lava
                } else {
                    Ground::Water
                },
                false,
            )
        } else if (val > 0.125 && mval < 0.20) || nval < 0.25 {
            (Ground::Dirt, false)
        } else if n16[9] > [0.65, 0.675, 0.7][level] {
            (
                [Ground::IronOre, Ground::GoldOre, Ground::GemOre][level],
                false,
            )
        } else if n32[3] > [0.8, 0.825, 0.85][level] {
            (Ground::Lapis, false)
        } else {
            (
                if stair_random([0.003, 0.0025, 0.002][level]) {
                    Ground::StairsDown
                } else {
                    Ground::Rock
                },
                true,
            )
        }
    };
    LayerSample {
        cell: Cell { ground, data: 0 },
        time_random_eligible: eligible,
    }
}

pub fn generate_layer(seed: i64, depth: i8, cx: i64, cy: i64) -> Result<Vec<Cell>, String> {
    if depth == 0 {
        return Ok(super::generate(seed, cx, cy));
    }
    if !(-4..=-1).contains(&depth) {
        return Err("Infdev exploration supports depths -4 through 0".into());
    }
    let mut noise = Simplex::infdev(seed);
    Ok((0..4096)
        .map(|index| {
            sample_with(
                &mut noise,
                seed,
                depth,
                cx.wrapping_mul(64).wrapping_add(index % 64),
                cy.wrapping_mul(64).wrapping_add(index / 64),
            )
            .cell
        })
        .collect())
}

#[derive(Debug, Default)]
pub struct LayerComparison {
    pub cells: usize,
    pub time_eligible: usize,
    pub time_choice_differences: usize,
    pub decoration_cells: usize,
}

/// All original fields are compared. Decorator writes are identified by the
/// actual Java setTile call stack, rather than inferred from discrepancies.
pub fn compare_layer_observation(
    bytes: &[u8],
    seed: i64,
    depth: i8,
    cx: i64,
    cy: i64,
) -> Result<LayerComparison, String> {
    if !(-4..=-1).contains(&depth) || bytes.len() != 4096 * 6 {
        return Err("invalid Java underground observation".into());
    }
    let mut report = LayerComparison::default();
    let mut noise = Simplex::infdev(seed);
    for (i, row) in bytes.as_chunks::<6>().0.iter().enumerate() {
        let x = cx.wrapping_mul(64).wrapping_add((i % 64) as i64);
        let y = cy.wrapping_mul(64).wrapping_add((i / 64) as i64);
        let actual = sample_with(&mut noise, seed, depth, x, y);
        if u16::from_be_bytes([row[1], row[2]]) != actual.cell.data
            || row[3] != depth.unsigned_abs()
            || row[4] != u8::from(actual.time_random_eligible)
            || row[5] > 1
        {
            return Err(format!(
                "Java underground data/biome/random flags differ depth={depth} ({x},{y}) Java={row:?}"
            ));
        }
        if row[5] == 1 {
            if depth != -4
                || ![
                    Ground::ObsidianWall as u8,
                    Ground::OrnateObsidian as u8,
                    Ground::Lava as u8,
                    Ground::ObsidianDoor as u8,
                ]
                .contains(&row[0])
            {
                return Err(format!(
                    "unexpected original decorator tile depth={depth} ({x},{y}) Java={row:?}"
                ));
            }
            report.decoration_cells += 1;
        } else if actual.time_random_eligible {
            report.time_eligible += 1;
            let deterministic = if depth == -4 {
                // Java's alternate branch is the same deterministic base tile.
                let mut base = sample_with(&mut noise, seed, depth, x, y).cell.ground;
                if base == Ground::StairsDown {
                    let value = f64::from(noise.noise3(
                        f64::from(x as i32) / 16.,
                        f64::from(y as i32) / 16.,
                        3200.,
                    ));
                    base = match (value * 9.).abs() as i32 % 3 {
                        0 => Ground::Obsidian,
                        1 => Ground::RawObsidian,
                        _ => Ground::Dirt,
                    };
                }
                base
            } else {
                Ground::Rock
            };
            if ![deterministic as u8, Ground::StairsDown as u8].contains(&row[0]) {
                return Err(format!(
                    "Java underground time-choice domain differs depth={depth} ({x},{y}) Java={row:?} Rust={:?}",
                    actual.cell
                ));
            }
            report.time_choice_differences += usize::from(row[0] != actual.cell.ground as u8);
        } else if row[0] != actual.cell.ground as u8 {
            return Err(format!(
                "Java underground tile differs depth={depth} ({x},{y}) Java={row:?} Rust={:?}",
                actual.cell
            ));
        }
        report.cells += 1;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn original_java_underground_chunks_preserve_fields_and_observe_random_writes() {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(include_bytes!(
            "../../../../tests/fixtures/infdev4a/underground-observation.zip"
        )))
        .unwrap();
        let mut cases = String::new();
        zip.by_name("underground-cases.tsv")
            .unwrap()
            .read_to_string(&mut cases)
            .unwrap();
        let mut total = LayerComparison::default();
        let mut grounds = std::collections::BTreeSet::new();
        for line in cases.lines() {
            let columns: Vec<_> = line.split('\t').collect();
            let mut bytes = Vec::new();
            zip.by_name(columns[4])
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            grounds.extend(bytes.iter().step_by(6).copied());
            let observation = compare_layer_observation(
                &bytes,
                columns[0].parse().unwrap(),
                columns[1].parse().unwrap(),
                columns[2].parse().unwrap(),
                columns[3].parse().unwrap(),
            )
            .unwrap();
            total.cells += observation.cells;
            total.time_eligible += observation.time_eligible;
            total.time_choice_differences += observation.time_choice_differences;
            total.decoration_cells += observation.decoration_cells;
            bytes[1] ^= 1;
            assert!(
                compare_layer_observation(
                    &bytes,
                    columns[0].parse().unwrap(),
                    columns[1].parse().unwrap(),
                    columns[2].parse().unwrap(),
                    columns[3].parse().unwrap()
                )
                .is_err(),
                "retained Java data cannot be hidden by random masks"
            );
        }
        assert_eq!(total.cells, 36 * 4096);
        for ground in [
            Ground::Dirt,
            Ground::IronOre,
            Ground::GoldOre,
            Ground::GemOre,
            Ground::Lapis,
            Ground::Water,
            Ground::Lava,
            Ground::Rock,
            Ground::ObsidianWall,
            Ground::Obsidian,
            Ground::RawObsidian,
            Ground::OrnateObsidian,
            Ground::ObsidianDoor,
            Ground::StairsDown,
        ] {
            assert!(
                grounds.contains(&(ground as u8)),
                "missing real Java material {ground:?}"
            );
        }
        assert!(total.time_eligible > 1000);
        assert!(total.decoration_cells > 0);
        println!(
            "Java underground: {} cells, {} time choices ({} differing), {} actual random pool final writes; time choices and pools are not equality claims",
            total.cells, total.time_eligible, total.time_choice_differences, total.decoration_cells
        );
    }

    #[test]
    fn depth_generation_is_deterministic_and_keeps_surface_history_and_signed_boundaries() {
        for depth in -4..=0 {
            let a = generate_layer(42, depth, -1, 0).unwrap();
            let _ = generate_layer(-1, depth, 0, -1).unwrap();
            assert_eq!(a, generate_layer(42, depth, -1, 0).unwrap());
            for x in [-65_i64, -64, -1, 0, 63, 64, i64::MIN, i64::MAX] {
                let generated = generate_layer(42, depth, x.div_euclid(64), -1).unwrap();
                let local = (x.rem_euclid(64) + 63 * 64) as usize;
                assert_eq!(
                    sample_layer(42, depth, x, -1).unwrap().cell,
                    generated[local]
                );
            }
        }
        assert_eq!(
            generate_layer(256, 0, -1, 1).unwrap(),
            super::super::generate(256, -1, 1)
        );
        for invalid in [i8::MIN, -5, 1, i8::MAX] {
            assert!(generate_layer(42, invalid, 0, 0).is_err());
            assert!(sample_layer(42, invalid, 0, 0).is_err());
        }
    }
}
