//! Untouched official World.initWorld saves, including naturally spawned mobs.
use super::*;
use crate::item::{Inventory, ToolItem, ToolKind, ToolTier};
use crate::saveload::{migration, migration::tests::fixture};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fields(text: &str) -> Vec<&str> {
    text.trim_end_matches(',')
        .split(',')
        .filter(|s| !s.is_empty())
        .collect()
}
fn canonical(name: &str) -> String {
    let key = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    match key.as_str() {
        "flower230" => "flower",
        "obsidian" => "obsidianfloor",
        "cloudcactus" => "cloudore",
        "lapis" => "lapisore",
        "woodplanks" => "woodfloor",
        "stonebricks" => "stonefloor",
        "stone" => "rawstone",
        "bosswall" => "obsidianbosswall",
        "bossfloor" => "obsidianbossfloor",
        "bossdoor" => "obsidianbossdoor",
        _ => &key,
    }
    .to_owned()
}
fn normalize_name(name: &str) -> &str {
    match name {
        "White Wool" => "Wool",
        "Tnt" => "TNT",
        _ => name,
    }
}
fn inventory_semantics(items: impl IntoIterator<Item = String>) -> Value {
    let tools = ToolKind::ALL
        .into_iter()
        .flat_map(|kind| {
            ToolTier::ALL
                .into_iter()
                .map(move |tier| ToolItem::new(kind, tier).display_name())
        })
        .collect::<Vec<_>>();
    let mut counts = BTreeMap::<String, u64>::new();
    let mut instances = Vec::<String>::new();
    for item in items {
        // Item.getData is a bare name for Book/FurnitureItem; stackables
        // and tools provide their actual count/durability suffix.
        let (name, amount) = item.rsplit_once('_').unwrap_or((&item, "1"));
        if name == "Watering Can" || tools.iter().any(|tool| tool == name) {
            instances.push(item);
        } else {
            *counts.entry(normalize_name(name).to_owned()).or_default() +=
                amount.parse::<u64>().unwrap();
        }
    }
    instances.sort();
    json!({"stackCounts":counts,"instances":instances})
}
fn native_inventory(inventory: &Inventory) -> Value {
    inventory_semantics(
        inventory
            .slots()
            .iter()
            .map(|stack| {
                let amount = if stack.item == ItemId::WateringCan {
                    stack.watering_content.expect("imported can contents")
                } else {
                    stack.count
                };
                format!("{}_{amount}", stack.item.display_name())
            })
            .chain(
                inventory
                    .tools()
                    .iter()
                    .map(|tool| format!("{}_{}", tool.display_name(), tool.durability)),
            ),
    )
}
pub(super) fn source_entities(text: &str) -> Vec<Value> {
    let mut entities = Vec::new();
    for line in fields(text) {
        let (name, body) = line.split_once('[').unwrap();
        let parts = body.trim_end_matches(']').split(':').collect::<Vec<_>>();
        if name == "Player" {
            continue;
        }
        let mut row = json!({"name":name,"x":parts[0].parse::<i32>().unwrap(),"y":parts[1].parse::<i32>().unwrap(),
            "levelIndex":parts.last().unwrap().parse::<usize>().unwrap()});
        match name {
            "Slime" | "Zombie" | "Skeleton" | "Creeper" | "Snake" | "Knight" | "AirWizard"
            | "ObsidianKnight" => {
                assert_eq!(parts.len(), 5);
                row["health"] = json!(parts[2].parse::<u16>().unwrap());
                row["mobLevel"] = json!(parts[3].parse::<u8>().unwrap());
            }
            "Cow" | "Pig" => {
                assert_eq!(parts.len(), 4);
                row["health"] = json!(parts[2].parse::<u16>().unwrap());
            }
            "Sheep" => {
                assert_eq!(parts.len(), 5);
                row["health"] = json!(parts[2].parse::<u16>().unwrap());
                row["sheared"] = json!(parts[3].parse::<bool>().unwrap());
            }
            "Spawner" => {
                assert_eq!(parts.len(), 5);
                row["mob"] = json!(parts[2]);
                row["mobLevel"] = json!(parts[3].parse::<u8>().unwrap());
            }
            "KnightStatue" => {
                assert_eq!(parts.len(), 4);
                row["bossHealth"] = json!(parts[2].parse::<u16>().unwrap());
            }
            "Chest" | "DungeonChest" => {
                let end = parts.len() - 1 - usize::from(name == "DungeonChest");
                row["inventory"] = inventory_semantics(parts[2..end].iter().map(|s| s.to_string()));
                if name == "DungeonChest" {
                    row["locked"] = json!(parts[end].parse::<bool>().unwrap());
                }
            }
            "Lantern" => {
                assert_eq!(parts.len(), 4);
                row["type"] = json!(parts[2].parse::<u8>().unwrap());
            }
            "Enchanter" => {
                assert_eq!(parts.len(), 3);
            }
            _ => panic!("Unaccounted actual source entity in full-world fixture: {line}"),
        }
        entities.push(row);
    }
    entities.sort_by_key(Value::to_string);
    entities
}
fn native_entities(world: &World) -> Vec<Value> {
    let mut entities = world
        .levels
        .iter()
        .enumerate()
        .flat_map(|(index, level)| native_level_entities(level, index))
        .collect::<Vec<_>>();
    entities.sort_by_key(Value::to_string);
    entities
}
pub(super) fn native_level_entities(level: &Level, level_index: usize) -> Vec<Value> {
    let mut entities = Vec::new();
    for entity in level.entities.entities() {
        let mut row = json!({"x":entity.x,"y":entity.y,"levelIndex":level_index});
        match &entity.kind {
            EntityKind::Mob(mob) => {
                row["name"] = json!(format!("{:?}", mob.species));
                row["health"] = json!(mob.health);
                if mob.species.hostile() {
                    row["mobLevel"] = json!(mob.level);
                }
                if mob.species == spawn::NaturalMob::Sheep {
                    row["sheared"] = json!(mob.sheared);
                }
            }
            EntityKind::Furniture(kind) => {
                if let Some(species) = kind.spawner_mob() {
                    row["name"] = json!("Spawner");
                    row["mob"] = json!(format!("{species:?}"));
                    row["mobLevel"] = json!(
                        entity
                            .spawner
                            .as_ref()
                            .expect("Spawner configuration")
                            .level
                    );
                } else {
                    match kind {
                        FurnitureKind::KnightStatue => {
                            row["name"] = json!("KnightStatue");
                            row["bossHealth"] =
                                json!(entity.statue_boss_health.expect("statue health"));
                        }
                        FurnitureKind::Chest | FurnitureKind::DungeonChest => {
                            row["name"] = json!(format!("{kind:?}"));
                            row["inventory"] = native_inventory(entity.storage.as_ref().unwrap());
                            if *kind == FurnitureKind::DungeonChest {
                                row["locked"] = json!(entity.state == 0);
                            }
                        }
                        FurnitureKind::Lantern
                        | FurnitureKind::IronLantern
                        | FurnitureKind::GoldLantern => {
                            row["name"] = json!("Lantern");
                            row["type"] = json!(match kind {
                                FurnitureKind::Lantern => 0,
                                FurnitureKind::IronLantern => 1,
                                _ => 2,
                            });
                        }
                        FurnitureKind::Enchanter => {
                            row["name"] = json!("Enchanter");
                        }
                        _ => panic!("Unexpected imported native furniture: {kind:?}"),
                    }
                }
            }
            _ => panic!("Unexpected imported native entity: {:?}", entity.kind),
        }
        entities.push(row);
    }
    entities.sort_by_key(Value::to_string);
    entities
}
fn compare_cells(source: &std::path::Path, world: &World) {
    for (index, level) in world.levels.iter().enumerate() {
        let names =
            std::fs::read_to_string(source.join(format!("Level{index}.miniplussave"))).unwrap();
        let values =
            std::fs::read_to_string(source.join(format!("Level{index}data.miniplussave"))).unwrap();
        let names = fields(&names);
        let values = fields(&values);
        assert_eq!(names.len(), 4 + 128 * 128);
        assert_eq!(values.len(), 128 * 128);
        assert_eq!(level.depth, names[3].parse::<i8>().unwrap());
        for x in 0..128 {
            for y in 0..128 {
                let saved = x * 128 + y;
                let native = x + y * 128;
                assert_eq!(
                    canonical(names[saved + 4]),
                    canonical(&format!("{:?}", level.tiles[native])),
                    "layer {index} at {x},{y}"
                );
                // Naturally assembled worlds contain only the stable legacy IDs as
                // overlay bases. No Flower ordinal or tile data is filtered here.
                assert_eq!(
                    level.data[native],
                    values[saved].parse::<u16>().unwrap(),
                    "data layer {index} at {x},{y}"
                );
            }
        }
    }
}
fn check_source_files_unchanged(source: &std::path::Path, original: &BTreeMap<String, Vec<u8>>) {
    let current = std::fs::read_dir(source)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into_string().unwrap(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(&current, original, "Java source changed");
}

fn compare_metadata(source: &std::path::Path, world: &World) {
    let game = std::fs::read_to_string(source.join("Game.miniplussave")).unwrap();
    let source_game = fields(&game);
    let native_game = crate::saveload::save::game_fields(world);
    assert_eq!(source_game.len(), 10);
    assert_eq!(source_game[0], "2.3.0-dev2");
    // The existing Java-format exporter intentionally stamps 2.2.4. Every
    // actual persisted Game value has the same field representation here.
    assert_eq!(&source_game[1..], &native_game[1..10]);
    let player = std::fs::read_to_string(source.join("Player.miniplussave")).unwrap();
    let source_player = player
        .trim_end_matches(',')
        .splitn(15, ',')
        .collect::<Vec<_>>();
    let native_player = crate::saveload::save::player_fields(world);
    assert_eq!(source_player.len(), 15);
    assert_eq!(&source_player[..14], &native_player[..14]);
    assert_eq!(
        serde_json::from_str::<Value>(source_player[14]).unwrap(),
        serde_json::from_str::<Value>(&native_player[14]).unwrap()
    );
    let inventory = std::fs::read_to_string(source.join("Inventory.miniplussave")).unwrap();
    assert_eq!(
        inventory_semantics(fields(&inventory).into_iter().map(str::to_owned)),
        native_inventory(&world.player.inventory)
    );
}

#[test]
fn actual_world_initialization_and_natural_runtime_import_preserve_all_saved_entities_and_disk_state()
 {
    for (bytes, summary, unlocks) in [
        (
            include_bytes!("../../tests/fixtures/java230/normal-survival-42.zip").as_slice(),
            include_str!("../../tests/fixtures/java230/world-assembly-42.json"),
            None,
        ),
        (
            include_bytes!("../../tests/fixtures/java230/normal-runtime-42.zip").as_slice(),
            include_str!("../../tests/fixtures/java230/world-runtime-42.json"),
            Some(include_str!(
                "../../tests/fixtures/java230/normal-runtime-unlocks.json"
            )),
        ),
    ] {
        let root = fixture(bytes);
        let source = root.0.join("source");
        let original = std::fs::read_dir(&source)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().into_string().unwrap(),
                    std::fs::read(entry.path()).unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let unlock_path = root.0.join("Unlocks.json");
        if let Some(text) = unlocks {
            std::fs::write(&unlock_path, text).unwrap();
        }
        migration::import_to(
            &source,
            &root.0.join("saves/Imported"),
            unlocks.map(|_| unlock_path.as_path()),
        )
        .unwrap();
        check_source_files_unchanged(&source, &original);
        let record = crate::worlds::load(&root.0).unwrap().pop().unwrap();
        let mut world = crate::worlds::load_state(&record).unwrap().unwrap();
        let summary: Value = serde_json::from_str(summary).unwrap();
        assert_eq!(
            world.current_level,
            summary["currentLevel"].as_u64().unwrap() as usize
        );
        assert_eq!(
            (world.player.x, world.player.y),
            (
                summary["player"]["x"].as_i64().unwrap() as i32,
                summary["player"]["y"].as_i64().unwrap() as i32
            )
        );
        assert_eq!(
            world.respawn_tile,
            Some((
                summary["spawn"]["x"].as_i64().unwrap() as i32,
                summary["spawn"]["y"].as_i64().unwrap() as i32
            ))
        );
        assert_eq!(
            world.clock.get().game_time,
            summary["gameTime"].as_i64().unwrap() as i32
        );
        assert_eq!(
            world.clock.get().day_tick,
            summary["tickCount"].as_u64().unwrap() as u32
        );
        let original_entities =
            String::from_utf8(original["Entities.miniplussave"].clone()).unwrap();
        assert_eq!(native_entities(&world), source_entities(&original_entities));
        compare_cells(&source, &world);
        compare_metadata(&source, &world);
        for (index, row) in summary["layers"].as_array().unwrap().iter().enumerate() {
            assert_eq!(world.java_level_seeds[index], row["seed"].as_i64().unwrap());
        }
        let source_progress: Value =
            serde_json::from_slice(&original["advancements.json"]).unwrap();
        let native: Value = serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
        assert_eq!(
            native["world"]["progress"]["java_advancements"],
            source_progress
        );
        if let Some(text) = unlocks {
            assert_eq!(
                world.java_global_unlocks,
                Some(serde_json::from_str(text).unwrap())
            );
            assert!(world.achievement_unlocked("minicraft.achievement.lowest_caves"));
            assert!(world.achievement_unlocked("minicraft.achievement.obsidian_dungeon"));
        }
        let imported_entities = native_entities(&world);
        crate::worlds::save_state(&record, &world).unwrap();
        world = crate::worlds::load_state(&record).unwrap().unwrap();
        compare_cells(&source, &world);
        compare_metadata(&source, &world);
        assert_eq!(native_entities(&world), imported_entities);
        // Drive the actual current-world simulation after load. This updates
        // imported mobs/spawners; exact AI trajectory equivalence is UNKNOWN.
        world.tick_after_display(true, &Input::default());
        let after_tick = world
            .levels
            .iter()
            .map(|level| serde_json::to_value(&level.entities).unwrap())
            .collect::<Vec<_>>();
        crate::worlds::save_state(&record, &world).unwrap();
        let restored = crate::worlds::load_state(&record).unwrap().unwrap();
        assert_eq!(
            restored
                .levels
                .iter()
                .map(|level| serde_json::to_value(&level.entities).unwrap())
                .collect::<Vec<_>>(),
            after_tick
        );
        check_source_files_unchanged(&source, &original);
    }
}
