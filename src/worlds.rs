use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::{
    storage::{atomic_write, read_candidates},
    world::{GameMode, World, WorldSpec, import_java_save, probe_java_save},
};

#[derive(Debug, Clone)]
pub struct WorldRecord {
    pub name: String,
    pub seed: i64,
    pub spec: WorldSpec,
    pub mode: GameMode,
    pub score_minutes: usize,
    pub directory: PathBuf,
}

pub fn load(game_dir: &Path) -> Result<Vec<WorldRecord>, String> {
    let saves = game_dir.join("saves");
    fs::create_dir_all(&saves)
        .map_err(|error| format!("cannot create {}: {error}", saves.display()))?;
    let mut records = Vec::new();
    let entries = fs::read_dir(&saves)
        .map_err(|error| format!("cannot read {}: {error}", saves.display()))?;
    for entry in entries.flatten() {
        let directory = entry.path();
        let path = directory.join("world.json");
        if let Ok(text) = fs::read_to_string(&path)
            && let Some(record) = parse_at(&text, directory.clone())
        {
            records.push(record);
            continue;
        }
        if let Ok(Some(java)) = probe_java_save(&directory) {
            let name = directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("JAVA WORLD")
                .to_owned();
            records.push(WorldRecord {
                name,
                seed: java.seed,
                spec: WorldSpec::new(java.size, 0, 0),
                mode: java.mode,
                score_minutes: java.score_minutes,
                directory,
            });
        }
    }
    records.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(records)
}

pub fn create(
    game_dir: &Path,
    seed: i64,
    spec: WorldSpec,
    mode: GameMode,
    score_minutes: usize,
) -> Result<WorldRecord, String> {
    let existing = load(game_dir)?;
    let mut number = 1;
    loop {
        let name = format!("WORLD {number}");
        if existing.iter().all(|world| world.name != name) {
            let directory = game_dir.join("saves").join(format!("world-{number}"));
            fs::create_dir_all(&directory)
                .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
            let record = WorldRecord {
                name,
                seed,
                spec,
                mode,
                score_minutes,
                directory: directory.clone(),
            };
            let value = json!({
                "format": 1,
                "game_version": "2.2.4-rust",
                "name": record.name,
                "seed": record.seed,
                "size": record.spec.size,
                "theme": record.spec.theme.index(),
                "terrain_type": record.spec.terrain.index(),
                "mode": record.mode.index(),
                "score_minutes": record.score_minutes,
                "state": "seed-only",
            });
            let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
            let path = directory.join("world.json");
            atomic_write(&path, text.as_bytes())?;
            return Ok(record);
        }
        number += 1;
    }
}

pub fn save_state(record: &WorldRecord, world: &World) -> Result<(), String> {
    let text = world.to_save_string()?;
    atomic_write(&record.directory.join("state.json"), text.as_bytes())
}

pub fn load_state(record: &WorldRecord) -> Result<Option<World>, String> {
    let path = record.directory.join("state.json");
    let candidates = read_candidates(&path)?;
    if candidates.is_empty() {
        if record.directory.join("Game.miniplussave").exists() {
            let world = import_java_save(&record.directory)?;
            save_state(record, &world)?;
            return Ok(Some(world));
        }
        return Ok(None);
    }
    let mut errors = Vec::new();
    for (candidate, text) in candidates {
        match World::from_save_string(&text) {
            Ok(world) => return Ok(Some(world)),
            Err(error) => errors.push(format!("{}: {error}", candidate.display())),
        }
    }
    Err(format!("no valid world state found: {}", errors.join("; ")))
}

fn parse_at(text: &str, directory: PathBuf) -> Option<WorldRecord> {
    let value: Value = serde_json::from_str(text).ok()?;
    Some(WorldRecord {
        name: value["name"].as_str()?.to_owned(),
        seed: value["seed"].as_i64()?,
        spec: WorldSpec::new(
            value["size"].as_u64().unwrap_or(128) as usize,
            value["theme"].as_u64().unwrap_or(0) as usize,
            value["terrain_type"].as_u64().unwrap_or(0) as usize,
        ),
        mode: GameMode::from_index(value["mode"].as_u64().unwrap_or(0) as usize),
        score_minutes: match value["score_minutes"].as_u64().unwrap_or(20) as usize {
            minutes @ (10 | 20 | 40 | 60 | 120) => minutes,
            _ => 20,
        },
        directory,
    })
}

#[cfg(test)]
fn parse(text: &str) -> Option<WorldRecord> {
    parse_at(text, PathBuf::new())
}

#[cfg(test)]
mod tests {
    use super::{WorldRecord, load_state, parse, save_state};

    #[test]
    fn malformed_world_records_are_ignored() {
        assert!(parse("not json").is_none());
        let world = parse(r#"{"name":"WORLD 1","seed":42}"#).unwrap();
        assert_eq!(world.name, "WORLD 1");
        assert_eq!(world.seed, 42);
        assert_eq!(world.spec, crate::world::WorldSpec::default());

        let configured =
            parse(r#"{"name":"WORLD 2","seed":9,"size":512,"theme":4,"terrain_type":3}"#).unwrap();
        assert_eq!(configured.spec.size, 512);
        assert_eq!(configured.spec.theme.index(), 4);
        assert_eq!(configured.spec.terrain.index(), 3);

        let score = parse(r#"{"name":"SCORE","seed":1,"mode":3,"score_minutes":60}"#).unwrap();
        assert_eq!(score.mode, crate::world::GameMode::Score);
        assert_eq!(score.score_minutes, 60);
    }

    #[test]
    fn corrupt_primary_world_recovers_from_atomic_backup() {
        let directory = std::env::temp_dir().join(format!(
            "minicraft-rust-world-backup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let record = WorldRecord {
            name: "BACKUP".to_owned(),
            seed: 7,
            spec: crate::world::WorldSpec::default(),
            mode: crate::world::GameMode::Survival,
            score_minutes: 20,
            directory: directory.clone(),
        };
        let world = crate::world::World::new_with_play_options(
            record.seed,
            record.spec,
            crate::world::PlayOptions {
                difficulty: 1,
                mode: record.mode,
                score_minutes: 20,
                tutorials: true,
                quests: true,
                show_quests: true,
                custom_skin: false,
            },
        );
        save_state(&record, &world).unwrap();
        save_state(&record, &world).unwrap();
        std::fs::write(directory.join("state.json"), "broken").unwrap();
        assert!(load_state(&record).unwrap().is_some());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
