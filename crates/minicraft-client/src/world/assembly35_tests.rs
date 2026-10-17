//! Full official World.initWorld evidence, separate from raw LevelGen parity.
//! Historical generation remains intentionally unchanged until a new complete
//! assembly contract is designed; differences are measured, never renamed away.
use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

const SUMMARY: &str = include_str!("../../tests/fixtures/java230/world-assembly-42.json");
const NORMAL: &[u8] = include_bytes!("../../tests/fixtures/java230/normal-survival-42.zip");

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
fn source_files(bytes: &[u8]) -> BTreeMap<String, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut file = zip.by_index(i).unwrap();
            let mut text = String::new();
            file.read_to_string(&mut text).unwrap();
            (file.name().to_owned(), text)
        })
        .collect()
}
fn fields(text: &str) -> Vec<&str> {
    text.trim_end_matches(',').split(',').collect()
}
fn source_level(files: &BTreeMap<String, String>, index: usize) -> (Vec<String>, Vec<u16>) {
    let tiles = fields(&files[&format!("Level{index}.miniplussave")]);
    let data = fields(&files[&format!("Level{index}data.miniplussave")]);
    let mut names = vec![String::new(); 128 * 128];
    let mut values = vec![0; 128 * 128];
    for y in 0..128 {
        for x in 0..128 {
            let from = x * 128 + y;
            let to = x + y * 128;
            names[to] = canonical(tiles[from + 4]);
            values[to] = data[from].parse().unwrap();
        }
    }
    (names, values)
}
fn hash<'a>(cells: impl Iterator<Item = (&'a str, u16)>) -> String {
    let mut value = 0xcbf29ce484222325_u64;
    for (name, data) in cells {
        for byte in format!("{name}:{data};").bytes() {
            value = (value ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    format!("{value:016x}")
}
fn coords(names: &[String], needle: &str) -> Vec<(usize, usize)> {
    names
        .iter()
        .enumerate()
        .filter_map(|(i, name)| (name == needle).then_some((i % 128, i / 128)))
        .collect()
}
fn json_coords(value: &Value) -> Vec<(usize, usize)> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["x"].as_u64().unwrap() as usize,
                p["y"].as_u64().unwrap() as usize,
            )
        })
        .collect()
}

#[test]
fn official_world_init_all_cells_entities_seeds_and_connections_match_source_fixture() {
    let summary: Value = serde_json::from_str(SUMMARY).unwrap();
    let files = source_files(NORMAL);
    assert_eq!(summary["version"], "2.3.0-dev2");
    assert_eq!(summary["worldSeed"], 42);
    assert_eq!(summary["player"], json!({"x":1352,"y":1064}));
    assert_eq!(summary["inventory"], json!([]));
    let seed_order = [
        (-3, -6_169_532_649_852_302_182_i64),
        (-2, 5_111_195_811_822_994_797),
        (-1, 5_694_868_678_511_409_995),
        (0, -5_843_495_416_241_995_736),
        (1, -5_025_562_857_975_149_833),
        (-4, -1_782_466_964_123_969_572),
    ];
    let mut saved_entities = fields(&files["Entities.miniplussave"])
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    saved_entities.sort();
    let mut described_entities = Vec::new();
    for row in summary["layers"].as_array().unwrap() {
        let index = row["index"].as_u64().unwrap() as usize;
        let depth = row["depth"].as_i64().unwrap() as i8;
        assert_eq!(seed_order[index], (depth, row["seed"].as_i64().unwrap()));
        let header = fields(&files[&format!("Level{index}.miniplussave")]);
        assert_eq!(
            header[..4],
            ["128", "128", &row["seed"].to_string(), &depth.to_string()]
        );
        let (names, data) = source_level(&files, index);
        assert_eq!(
            hash(names.iter().zip(&data).map(|(n, d)| (n.as_str(), *d))),
            row["hash"].as_str().unwrap()
        );
        assert_eq!(coords(&names, "stairsup"), json_coords(&row["stairsUp"]));
        assert_eq!(
            coords(&names, "stairsdown"),
            json_coords(&row["stairsDown"])
        );
        described_entities.extend(
            row["entities"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e.as_str().unwrap().to_owned()),
        );
        // This uses the derived Java layer seed, not the world seed. Raw map
        // equality was already established; assembly adds real structures.
        let raw = generation::level_with_version(
            128,
            128,
            depth,
            row["seed"].as_i64().unwrap(),
            WorldSpec::default(),
            GenerationVersion::Java230Flora,
        );
        let raw_names = raw
            .tiles
            .iter()
            .map(|tile| canonical(&format!("{tile:?}")))
            .collect::<Vec<_>>();
        assert_eq!(
            hash(
                raw_names
                    .iter()
                    .zip(&raw.data)
                    .map(|(n, d)| (n.as_str(), *d))
            ),
            row["assemblyDeltaFromSameSeedRawMap"]["rawHash"]
                .as_str()
                .unwrap()
        );
        let changed = names
            .iter()
            .zip(&data)
            .zip(raw_names.iter().zip(&raw.data))
            .filter(|((a, ad), (b, bd))| a != b || ad != bd)
            .count();
        assert_eq!(
            changed as u64,
            row["assemblyDeltaFromSameSeedRawMap"]["changedCells"]
                .as_u64()
                .unwrap()
        );
    }
    described_entities.sort();
    assert_eq!(saved_entities, described_entities);
    assert_eq!(summary["stairConnections"].as_array().unwrap().len(), 17);
    for connection in summary["stairConnections"].as_array().unwrap() {
        assert_eq!(connection["connected"], true);
        let x = connection["x"].as_u64().unwrap() as usize;
        let y = connection["y"].as_u64().unwrap() as usize;
        for (key, expected) in [("upperDepth", "stairsdown"), ("lowerDepth", "stairsup")] {
            let depth = connection[key].as_i64().unwrap();
            let layer = summary["layers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["depth"] == depth)
                .unwrap();
            let (names, _) = source_level(&files, layer["index"].as_u64().unwrap() as usize);
            assert_eq!(names[x + y * 128], expected);
        }
    }
}

#[test]
fn historical_java230flora_full_world_difference_is_explicit_and_reproducible() {
    let summary: Value = serde_json::from_str(SUMMARY).unwrap();
    let files = source_files(NORMAL);
    let world = World::new_with_generation(
        42,
        WorldSpec::default(),
        PlayOptions::survival(1),
        GenerationVersion::Java230Flora,
    );
    let mut report = Vec::new();
    for row in summary["layers"].as_array().unwrap() {
        let depth = row["depth"].as_i64().unwrap() as i8;
        let native = world
            .levels
            .iter()
            .find(|level| level.depth == depth)
            .unwrap();
        let (java_names, java_data) = source_level(&files, row["index"].as_u64().unwrap() as usize);
        let native_names = native
            .tiles
            .iter()
            .map(|tile| canonical(&format!("{tile:?}")))
            .collect::<Vec<_>>();
        let differences = java_names
            .iter()
            .zip(&java_data)
            .zip(native_names.iter().zip(&native.data))
            .enumerate()
            .filter_map(|(i, ((a, ad), (b, bd)))| (a != b || ad != bd).then_some(i))
            .collect::<Vec<_>>();
        assert!(
            !differences.is_empty(),
            "Unexpected full-world equality at depth {depth}; review the generation contract and oracle evidence before changing this historical test"
        );
        let first = differences[0];
        let native_entities = native
            .entities
            .entities()
            .iter()
            .map(|e| json!({"x":e.x,"y":e.y,"kind":e.kind,"storage":e.storage}))
            .collect::<Vec<_>>();
        report.push(json!({"depth":depth,"javaSeed":row["seed"],"historicalRustLayerSeed":42,
            "javaHash":row["hash"],"rustHash":hash(native_names.iter().zip(&native.data).map(|(n,d)|(n.as_str(),*d))),
            "changedCells":differences.len(),"firstDifference":{"x":first%128,"y":first/128,"javaTile":java_names[first],"javaData":java_data[first],"rustTile":native_names[first],"rustData":native.data[first]},
            "javaStairsUp":row["stairsUp"],"javaStairsDown":row["stairsDown"],
            "rustStairsUp":coords(&native_names,"stairsup"),"rustStairsDown":coords(&native_names,"stairsdown"),
            "javaEntities":row["entities"],"rustEntities":native_entities}));
    }
    // Exact coordinates and counts are evidence, not a passing claim of Java
    // equality. The test retains every Java entity and every map cell.
    println!("WORLD_ASSEMBLY35_DIFFERENCES={}", json!(report));
    assert_eq!(world.generation_version(), GenerationVersion::Java230Flora);
}

#[test]
fn official_natural_runtime_snapshot_matches_every_saved_cell_and_entity() {
    let summary: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/world-runtime-42.json"
    ))
    .unwrap();
    let files = source_files(include_bytes!(
        "../../tests/fixtures/java230/normal-runtime-42.zip"
    ));
    assert_eq!(summary["version"], "2.3.0-dev2");
    assert_eq!(summary["totalLevelTicks"], 300);
    assert_eq!(summary["playerTicks"], false);
    assert_eq!(summary["gameTime"], 300);
    assert_eq!(summary["playerHealth"], 10);
    assert_eq!(summary["currentLevel"], 5);
    let visits = summary["visits"].as_array().unwrap();
    assert_eq!(visits.len(), 5);
    for (visit, depth) in visits.iter().zip([0, -1, -2, -3, -4]) {
        assert_eq!(visit["depth"], depth);
        assert_eq!(visit["ticks"], 60);
    }
    let mut described = Vec::new();
    let mut ordinary = 0;
    let mut counts = BTreeMap::<String, usize>::new();
    for row in summary["layers"].as_array().unwrap() {
        let (names, data) = source_level(&files, row["index"].as_u64().unwrap() as usize);
        assert_eq!(
            hash(names.iter().zip(&data).map(|(n, d)| (n.as_str(), *d))),
            row["hash"].as_str().unwrap()
        );
        assert_eq!(coords(&names, "stairsup"), json_coords(&row["stairsUp"]));
        assert_eq!(
            coords(&names, "stairsdown"),
            json_coords(&row["stairsDown"])
        );
        for saved in row["entities"].as_array().unwrap() {
            let saved = saved.as_str().unwrap();
            let (name, body) = saved.split_once('[').unwrap();
            *counts.entry(name.to_owned()).or_default() += 1;
            if matches!(
                name,
                "Slime" | "Zombie" | "Skeleton" | "Creeper" | "Snake" | "Knight"
            ) {
                ordinary += 1;
                let values = body.trim_end_matches(']').split(':').collect::<Vec<_>>();
                assert_eq!(values.len(), 5);
                assert_eq!(
                    values[3].parse::<i64>().unwrap(),
                    -row["depth"].as_i64().unwrap()
                );
                assert!(values[2].parse::<u16>().unwrap() > 0);
            }
            described.push(saved.to_owned());
        }
    }
    let mut saved = fields(&files["Entities.miniplussave"])
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    saved.sort();
    described.sort();
    assert_eq!(saved, described);
    assert_eq!(
        ordinary,
        summary["ordinaryEnemies"].as_u64().unwrap() as usize
    );
    assert_eq!(ordinary, 115);
    assert_eq!(counts["Spawner"], 7);
    assert_eq!(counts["KnightStatue"], 1);
    assert_eq!(counts["DungeonChest"], 17);
    assert_eq!(counts["Player"], 1);
    assert_eq!(counts["AirWizard"], 1);
    // The raw Java runtime creates unseeded per-entity RNGs. This fixture is a
    // real recorded official Save, not a cross-JVM trajectory golden sample.
    assert_eq!(summary["transitions"].as_array().unwrap().len(), 4);
}
