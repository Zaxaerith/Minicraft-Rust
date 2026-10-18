//! Explicit disk-storage measurement, kept out of the normal game/test path.
//! All data belongs to one exclusive temporary directory removed on return.
use super::*;
use std::io::Write;
use std::time::Instant;

fn stage(dataset: &str, count: usize, operation: &str) -> Result<(), String> {
    println!("INFDEV_STORAGE_STAGE\t{dataset}\t{count}\t{operation}");
    std::io::stdout().flush().map_err(|e| e.to_string())
}

/// Full validation and GC timing; filesystem cache is deliberately not flushed.
/// Synthetic chunks are distinct edited terrain, not repeated index references.
pub fn measure_storage(output: &Path) -> Result<(), String> {
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    real_path(output, true)?;
    for (dataset, count) in [
        ("edited_unique", 1),
        ("edited_unique", 256),
        ("edited_unique", 2048),
        ("edited_unique", 8205),
        ("natural_java_surface", 64),
    ] {
        stage(dataset, count, "prepare")?;
        let owned = objects::Stage::create(output)?;
        let mut w = World::create(&owned.0, "Probe", 42, 2)?;
        let mut entries = Vec::with_capacity(count);
        let base = terrain::generate(42, 0, 0);
        for x in 0..count {
            let key = if dataset == "natural_java_surface" {
                Key {
                    x: (x % 8) as i64 - 4,
                    y: (x / 8) as i64 - 4,
                }
            } else {
                Key { x: x as i64, y: 0 }
            };
            let cells = if dataset == "natural_java_surface" {
                terrain::generate(42, key.x, key.y)
            } else {
                let mut cells = base.clone();
                // Encode an identity using legal Grass/Sand cells. This gives
                // each key its own 12-KiB object without inventing tile data.
                for (bit, cell) in cells[..16].iter_mut().enumerate() {
                    *cell = Cell {
                        ground: if x & (1 << bit) == 0 {
                            Ground::Grass
                        } else {
                            Ground::Sand
                        },
                        data: 0,
                    };
                }
                cells
            };
            entries.push((key, w.write_chunk(&cells)?));
        }
        entries.sort_unstable_by_key(|(key, _)| *key);
        w.saved.root = w.build_balanced(&entries)?;
        w.resident.clear();
        w.saved.position = (0, 0);
        w.commit_with_hook(|| Ok(()))?;
        let directory = w.directory.clone();
        drop(w);

        stage(dataset, count, "open")?;
        let started = Instant::now();
        let mut w = World::open(&directory, 2)?;
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        if w.saved_chunk_count()? != count as u64 {
            return Err("measurement fixture lost index entries".into());
        }
        println!("INFDEV_STORAGE_RESULT\t{dataset}\t{count}\topen\t{elapsed:.3}");

        stage(dataset, count, "gc")?;
        let started = Instant::now();
        let collection = w.collect_garbage()?;
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        println!(
            "INFDEV_STORAGE_RESULT\t{dataset}\t{count}\tgc\t{elapsed:.3}\t{}",
            collection.retained_objects
        );
        if w.saved_chunk_count()? != count as u64 {
            return Err("collection changed the live fixture index".into());
        }
        stage(dataset, count, "cleanup")?;
        drop(w);
        drop(owned);
    }
    Ok(())
}
