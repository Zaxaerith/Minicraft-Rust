//! Real constructor stage snapshots, not imported worlds or Rust-generated goldens.
use super::migration35_world_tests::{native_level_entities, source_entities};
use super::*;
use serde_json::Value;
use std::io::{Cursor, Read};
fn source_tile(id: u8) -> Tile {
    match id {
        2 => Tile::Flower230,
        59 => Tile::OrnateWood,
        60..=69 => Tile::from_id(id - 1).expect("explicit dev2 added wool ID"),
        0..=58 => Tile::from_id(id).expect("stable source ID"),
        _ => panic!("unknown Java tile ID{id}"),
    }
}
fn compare(bytes: &[u8]) {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    archive
        .by_name("world36.json")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let source: Value = serde_json::from_str(&text).unwrap();
    let seed = source["seed"].as_i64().unwrap();
    let size = source["size"].as_u64().unwrap() as usize;
    let stages = source["stages"].as_array().unwrap();
    let mut observed = 0;
    let world = assembly36::build(
        seed,
        WorldSpec::new(size, 0, 0),
        PlayOptions {
            tutorials: true,
            quests: true,
            ..PlayOptions::survival(1)
        },
        |level, stage, rng, count| {
            let expected = &stages[observed];
            observed += 1;
            assert_eq!(i64::from(level.depth), expected["depth"].as_i64().unwrap());
            assert_eq!(stage, expected["stage"].as_str().unwrap());
            let description = format!("size{size} seed{seed} depth{} stage{stage}", level.depth);
            let mut cells = Vec::new();
            archive
                .by_name(expected["map"].as_str().unwrap())
                .unwrap()
                .read_to_end(&mut cells)
                .unwrap();
            assert_eq!(cells.len(), size * size * 4);
            let mut first = None;
            let mut changed = 0;
            for (i, cell) in cells.as_chunks::<4>().0.iter().enumerate() {
                let id = u16::from_be_bytes([cell[0], cell[1]]);
                assert!(id <= 69);
                let tile = source_tile(id as u8);
                let data = u16::from_be_bytes([cell[2], cell[3]]);
                if (level.tiles[i], level.data[i]) != (tile, data) {
                    changed += 1;
                    first.get_or_insert((
                        i % size,
                        i / size,
                        tile,
                        data,
                        level.tiles[i],
                        level.data[i],
                    ));
                }
            }
            assert_eq!(changed, 0, "{description}: first={first:?}");
            assert_eq!(
                rng,
                expected["rngState"].as_u64().unwrap(),
                "{description} first RNG divergence"
            );
            assert_eq!(
                u64::from(count),
                expected["chestCount"].as_u64().unwrap(),
                "{description} source constructor counter"
            );
            let encoded = expected["entities"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(",");
            let actual = native_level_entities(level, assembly36::java_index(level.depth));
            let wanted = source_entities(&encoded);
            if actual != wanted {
                let i = actual
                    .iter()
                    .zip(&wanted)
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(wanted.len()));
                panic!(
                    "{description} every persisted entity/loot: count{}/{} first[{i}]={:?}/{:?}",
                    actual.len(),
                    wanted.len(),
                    actual.get(i),
                    wanted.get(i)
                );
            }
        },
    );
    assert_eq!(observed, stages.len());
    assert_eq!(world.generation_version(), GenerationVersion::Java230World);
    assert_eq!(
        (world.player.x, world.player.y),
        (
            source["playerX"].as_i64().unwrap() as i32,
            source["playerY"].as_i64().unwrap() as i32
        )
    );
    assert_eq!(
        world.respawn_tile,
        Some((
            source["spawnX"].as_i64().unwrap() as i32,
            source["spawnY"].as_i64().unwrap() as i32
        ))
    );
    for level in &world.levels {
        let original = stages
            .iter()
            .find(|s| s["stage"] == "raw" && s["depth"] == i64::from(level.depth))
            .unwrap();
        assert_eq!(
            world.java_level_seeds[assembly36::java_index(level.depth)],
            original["seed"].as_i64().unwrap()
        );
    }
    assert!(world.validate_save().is_ok());
    let read = |archive: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str| {
        let mut s = String::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        s
    };
    let player = read(&mut archive, "Player.miniplussave");
    let wanted = player
        .trim_end_matches(',')
        .splitn(15, ',')
        .collect::<Vec<_>>();
    assert_eq!(
        crate::saveload::save::player_fields(&world)[..15],
        wanted,
        "all initial Player15 fields, including recipe progress"
    );
    let game = read(&mut archive, "Game.miniplussave");
    let game = game.trim_end_matches(',').split(',').collect::<Vec<_>>();
    assert_eq!(
        crate::saveload::save::game_fields(&world)[1..10],
        game[1..10],
        "all initial Game values except the deliberately unchanged legacy Java-export stamp"
    );
    assert!(
        read(&mut archive, "Inventory.miniplussave")
            .trim_matches(',')
            .is_empty()
    );
    assert_eq!(world.player.inventory.used_slots(), 0);
    assert_eq!(world.progress.tutorial_completed_count(), 0);
    assert_eq!(world.progress.quest_completed_count(), 0);
    assert_eq!(world.progress.achievement_count(), 0);
    assert_eq!(
        read(&mut archive, "advancements.json").trim(),
        r#"{"Version": "2.3.0-dev2"}"#
    );
    assert_stairs(&world);
    disk_resume(world, size, seed);
}
#[test]
fn java36_seed42_all_constructor_stages_tiles_data_rng_entities_and_spawn() {
    compare(include_bytes!(
        "../../tests/fixtures/java230/world36-128-42.zip"
    ));
}

macro_rules! case {
    ($name:ident,$file:literal) => {
        #[test]
        fn $name() {
            compare(include_bytes!($file));
        }
    };
}
case!(
    java36_seed256_all_stages,
    "../../tests/fixtures/java230/world36-128-256.zip"
);
case!(
    java36_seed_zero_all_stages,
    "../../tests/fixtures/java230/world36-128-0.zip"
);
case!(
    java36_negative_seed_all_stages,
    "../../tests/fixtures/java230/world36-128--1.zip"
);
case!(
    java36_size256_all_stages,
    "../../tests/fixtures/java230/world36-256-42.zip"
);
case!(
    java36_size512_all_stages,
    "../../tests/fixtures/java230/world36-512-42.zip"
);

fn assert_stairs(world: &World) {
    for parent in &world.levels {
        if parent.depth == -4 {
            continue;
        }
        let lower = world
            .levels
            .iter()
            .find(|l| l.depth == parent.depth - 1)
            .unwrap();
        for (i, t) in parent.tiles.iter().enumerate() {
            if *t == Tile::StairsDown {
                assert_eq!(
                    lower.tiles[i],
                    Tile::StairsUp,
                    "depth{} stairs at{},{}",
                    parent.depth,
                    i % world.width,
                    i / world.width
                );
            }
        }
    }
}
fn disk_resume(mut world: World, size: usize, seed: i64) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parity/java230")
        .join(format!(
            "world36-disk-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&path).unwrap();
    let root = crate::saveload::migration::tests::FixtureRoot(path);
    let record = crate::worlds::create_named(
        &root.0,
        "New",
        seed,
        WorldSpec::new(size, 0, 0),
        GameMode::Survival,
        20,
    )
    .unwrap();
    // Explicit generation selection is verified independently of saved state.
    assert_eq!(record.generation_version, GenerationVersion::Java230World);
    assert!(crate::worlds::load_state(&record).unwrap().is_none());
    let surface = world.current_level;
    if size == 128 && seed == 42 {
        // Real scheduled staircase transitions, including sky and dungeon,
        // rather than changing current_level directly to fake traversal.
        for dir in [-1, 1, 1, 1, 1, 1, -1, -1, -1, -1] {
            let wanted = if dir == 1 {
                Tile::StairsDown
            } else {
                Tile::StairsUp
            };
            let i = world.levels[world.current_level]
                .tiles
                .iter()
                .position(|t| *t == wanted)
                .unwrap();
            world.player.x = (i % size) as i32 * 16 + 8;
            world.player.y = (i / size) as i32 * 16 + 8;
            let before = world.levels[world.current_level].depth;
            world.on_stair_delay = 0;
            world.tick_stair_and_fall();
            assert_eq!(world.pending_level_change, dir);
            for _ in 0..32 {
                world.tick_death_and_transition(&Input::default());
            }
            assert_eq!(world.levels[world.current_level].depth, before - dir);
        }
        assert_eq!(world.current_level, surface);
    }
    // Persist a genuine harvested flower to prove load never reruns LevelGen.
    let flower = world.levels[surface]
        .tiles
        .iter()
        .position(|t| *t == Tile::Flower230)
        .unwrap();
    world.attack_target_tile((flower % size) as i32, (flower / size) as i32, flower);
    assert_eq!(world.levels[surface].tiles[flower], Tile::Grass);
    crate::worlds::save_state(&record, &world).unwrap();

    let saved = state(&world);
    let loaded = crate::worlds::load_state(&record).unwrap().unwrap();
    same_state(state(&loaded), &saved);
    assert_stairs(&loaded);
    if size == 128 && seed == 42 {
        crate::worlds::copy_record(&root.0, &record, "Copy").unwrap();
        let copy = crate::worlds::load(&root.0)
            .unwrap()
            .into_iter()
            .find(|r| r.name == "Copy")
            .unwrap();
        assert_eq!(copy.generation_version, GenerationVersion::Java230World);
        crate::worlds::rename_record(&root.0, &copy, "Renamed").unwrap();
        let renamed = crate::worlds::load(&root.0)
            .unwrap()
            .into_iter()
            .find(|r| r.name == "Renamed")
            .unwrap();
        assert_eq!(renamed.generation_version, GenerationVersion::Java230World);
        same_state(
            state(&crate::worlds::load_state(&renamed).unwrap().unwrap()),
            &saved,
        );
        let primary = record.directory.join("state.json");
        std::fs::copy(&primary, record.directory.join("state.json.bak")).unwrap();
        let mut broken = saved.clone();
        broken["world"]["generation_version"] = serde_json::json!("future-unknown");
        std::fs::write(&primary, broken.to_string()).unwrap();
        same_state(
            state(&crate::worlds::load_state(&record).unwrap().unwrap()),
            &saved,
        );
        std::fs::remove_file(record.directory.join("state.json.bak")).unwrap();
        std::fs::write(&primary, broken.to_string()).unwrap();
        assert!(crate::worlds::load_state(&record).is_err());
    }
}
fn state(world: &World) -> Value {
    let mut value: Value = serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    // Only these actual HashSets have no order. Maps, entities, inventory and
    // all other arrays retain their complete strict ordering and values.
    for key in [
        "tutorial_completed",
        "quest_completed",
        "group_unlocked",
        "quest_unlocked",
        "criterion_hits",
        "achievements",
        "unlocking_hits",
        "recipe_completed",
    ] {
        value["world"]["progress"][key]
            .as_array_mut()
            .unwrap()
            .sort_by_key(Value::to_string);
    }
    value
}
fn same_state(actual: Value, wanted: &Value) {
    fn first(a: &Value, b: &Value, path: String) -> Option<String> {
        if a == b {
            return None;
        }
        match (a, b) {
            (Value::Object(a), Value::Object(b)) if a.keys().eq(b.keys()) => a
                .iter()
                .find_map(|(k, v)| first(v, &b[k], format!("{path}.{k}"))),
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
                .iter()
                .zip(b)
                .enumerate()
                .find_map(|(i, (a, b))| first(a, b, format!("{path}[{i}]"))),
            _ => Some(format!("{path}: actual={a:.120}, wanted={b:.120}")),
        }
    }
    assert!(
        actual == *wanted,
        "disk state mismatch: {}",
        first(&actual, wanted, String::new()).unwrap_or_default()
    );
}
