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
                    depth: 0,
                    x: (x % 8) as i64 - 4,
                    y: (x / 8) as i64 - 4,
                }
            } else {
                Key {
                    depth: 0,
                    x: x as i64,
                    y: 0,
                }
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

/// Synthetic mixed-entity storage fixture, NOT a naturally earned playthrough.
/// Uses the real object/cache/checkpoint/validation/GC pipeline and asserts every
/// serialized entity after eviction and restart; outputs one reusable CSV.
pub fn measure_survival(output: &Path) -> Result<(), String> {
    use super::super::{
        entities::{
            DeathChest, Kind, Page, Position,
            dynamic::{Animal, ItemDrop},
        },
        survival::{Loot, StationKind},
    };
    use crate::{
        entity::mob::NaturalMob,
        item::{Inventory, ItemId, ItemStack, ToolItem, ToolKind, ToolTier},
    };
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let mut csv = String::from(
        "chunks,entities,stations,open_ms,save_ms,gc_ms,bytes_before_gc,bytes_after_gc,files_after_gc\n",
    );
    for count in [64usize, 256] {
        let owned = objects::Stage::create(output)?;
        let mut w = World::create_survival(&owned.0, "Mixed", 42, 8)?;
        let mut expected = Vec::new();
        let mut total = 0;
        for i in 0..count {
            let key = Key {
                depth: -((i % 5) as i8),
                x: i as i64 - count as i64 / 2,
                y: if i % 2 == 0 { -2 } else { 2 },
            };
            w.ensure(key)?;
            let mut page = Page::empty(key);
            let mut insert = |kind: Kind, index: i64| -> Result<(), String> {
                page.insert(
                    Position {
                        depth: key.depth,
                        x: key.x * 64 + index,
                        y: key.y * 64 + 8,
                    },
                    kind,
                    None,
                )
            };
            for (n, k) in [
                StationKind::Workbench,
                StationKind::Furnace,
                StationKind::Anvil,
            ]
            .into_iter()
            .enumerate()
            {
                insert(Kind::Station(k), n as i64 + 8)?;
            }
            if key.depth == 0 {
                for (n, k) in [NaturalMob::Cow, NaturalMob::Pig, NaturalMob::Sheep]
                    .into_iter()
                    .enumerate()
                {
                    insert(Kind::Animal(Animal::of(k)), n as i64 + 12)?;
                }
            } else if key.depth >= -3 {
                insert(
                    Kind::Enemy(Animal::of_level(NaturalMob::Zombie, (-key.depth) as u8)),
                    12,
                )?;
            }
            insert(
                Kind::Drop(ItemDrop {
                    loot: Loot::Stack(ItemStack::with_water(23)),
                    delay: 17,
                }),
                16,
            )?;
            let mut inventory = Inventory::new(27);
            inventory.add(ItemId::Wood, 731);
            inventory.add_stack(ItemStack::with_water(731));
            let mut tool = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
            tool.durability = 23;
            inventory.add_tool(tool);
            insert(
                Kind::DeathChest(DeathChest {
                    inventory,
                    remaining: 1234,
                }),
                18,
            )?;
            page.validate()?;
            // Intentionally edit only the nine fixture cells; retain actual
            // generated chunks and use legal land beneath the storage probes.
            let chunk = w.resident.get_mut(&key).expect("ensured fixture");
            for entity in &page.entities {
                chunk.cells[usize::from(entity.y) * 64 + usize::from(entity.x)] = Cell {
                    ground: Ground::Dirt,
                    data: 0,
                };
            }
            chunk.dirty = true;
            w.survival.as_mut().unwrap().entity_pages_present = true;
            total += page.entities.len();
            expected.push(page.clone());
            w.entity_cache.insert(
                key,
                entities::CachedEntities {
                    page,
                    dirty: true,
                    touched: i as u64,
                },
            );
            w.trim_entities()?;
        }
        let started = Instant::now();
        w.checkpoint()?;
        let save_ms = started.elapsed().as_secs_f64() * 1000.;
        let directory = w.directory.clone();
        drop(w);
        let started = Instant::now();
        let mut w = World::open(&directory, 8)?;
        let open_ms = started.elapsed().as_secs_f64() * 1000.;
        for page in &expected {
            if &w.entity_page(page.key)? != page {
                return Err("mixed fixture lost entity identity/state".into());
            }
            if w.entity_cache.len() > 8 {
                return Err("unbounded mixed fixture cache".into());
            }
        }
        w.checkpoint()?;
        let (_, before) = disk_usage(&directory)?;
        let started = Instant::now();
        w.collect_garbage()?;
        let gc_ms = started.elapsed().as_secs_f64() * 1000.;
        let (files, after) = disk_usage(&directory)?;
        for page in &expected {
            if &w.entity_page(page.key)? != page {
                return Err("GC changed mixed fixture".into());
            }
        }
        // Subsequent real updates coexist with stations/chests and use the same
        // bounded activity/transaction path; the exact persistence check above
        // precedes these intentional AI changes.
        let first = expected[0].key;
        w.visit_layer(first.depth, first.x * 64 + 32, first.y * 64 + 32)?;
        for _ in 0..30 {
            w.survival_action(super::super::survival::Action::Tick)?;
        }
        w.checkpoint()?;
        drop(w);
        drop(World::open(&directory, 1)?);
        let row = format!(
            "{count},{total},{},{open_ms:.3},{save_ms:.3},{gc_ms:.3},{before},{after},{files}\n",
            count * 3
        );
        print!("INFDEV_MIXED_STORAGE {row}");
        csv.push_str(&row);
    }
    fs::write(output.join("infdev-mixed-storage.csv"), csv).map_err(|e| e.to_string())
}

fn disk_usage(root: &Path) -> Result<(u64, u64), String> {
    let (mut files, mut bytes) = (0, 0);
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let m = entry.metadata().map_err(|e| e.to_string())?;
        if m.is_dir() {
            let (f, b) = disk_usage(&entry.path())?;
            files += f;
            bytes += b;
        } else {
            files += 1;
            bytes += m.len();
        }
    }
    Ok((files, bytes))
}
