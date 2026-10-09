use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::{
    saveload::storage::{atomic_write, read_candidates, read_candidates_limited, restore_primary},
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
        if let Ok(candidates) = read_candidates_limited(&path, 1024 * 1024)
            && let Some(record) = candidates
                .iter()
                .find_map(|(_, text)| parse_at(text, directory.clone()))
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

pub fn create_named(
    game_dir: &Path,
    requested_name: &str,
    seed: i64,
    spec: WorldSpec,
    mode: GameMode,
    score_minutes: usize,
) -> Result<WorldRecord, String> {
    let name = requested_name.trim();
    validate_new_name(game_dir, name, None)?;
    let saves = game_dir.join("saves");
    let directory = saves.join(name);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let record = WorldRecord {
        name: name.to_owned(),
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
    atomic_write(&directory.join("world.json"), text.as_bytes())?;
    Ok(record)
}

pub fn copy_record(
    game_dir: &Path,
    source: &WorldRecord,
    requested_name: &str,
) -> Result<(), String> {
    let name = requested_name.trim();
    validate_new_name(game_dir, name, None)?;
    ensure_record_in_save_root(game_dir, source)?;
    let destination = game_dir.join("saves").join(name);
    copy_directory(&source.directory, &destination)?;
    update_record_name(&destination, name)
}

pub fn rename_record(
    game_dir: &Path,
    source: &WorldRecord,
    requested_name: &str,
) -> Result<(), String> {
    let name = requested_name.trim();
    validate_new_name(game_dir, name, Some(&source.name))?;
    ensure_record_in_save_root(game_dir, source)?;
    if name == source.name {
        return Ok(());
    }
    let destination = game_dir.join("saves").join(name);
    fs::rename(&source.directory, &destination).map_err(|error| {
        format!(
            "cannot rename {} to {}: {error}",
            source.directory.display(),
            destination.display()
        )
    })?;
    update_record_name(&destination, name)
}

pub fn delete_record(game_dir: &Path, source: &WorldRecord) -> Result<(), String> {
    ensure_record_in_save_root(game_dir, source)?;
    fs::remove_dir_all(&source.directory)
        .map_err(|error| format!("cannot delete {}: {error}", source.directory.display()))
}

fn validate_new_name(
    game_dir: &Path,
    name: &str,
    ignored_existing: Option<&str>,
) -> Result<(), String> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
    {
        return Err("world name is not a valid save-folder name".to_owned());
    }
    // A corrupt/unknown world may be absent from the menu. Its directory is
    // still user data and must not be reused by create_dir_all + world.json.
    let destination = game_dir.join("saves").join(name);
    if fs::symlink_metadata(&destination).is_ok()
        && ignored_existing.is_none_or(|ignored| !name.eq_ignore_ascii_case(ignored))
    {
        return Err(format!("save directory {name} already exists"));
    }
    if load(game_dir)?.iter().any(|world| {
        world.name.eq_ignore_ascii_case(name)
            && ignored_existing.is_none_or(|ignored| !world.name.eq_ignore_ascii_case(ignored))
    }) {
        return Err(format!("a world named {name} already exists"));
    }
    Ok(())
}

fn ensure_record_in_save_root(game_dir: &Path, record: &WorldRecord) -> Result<(), String> {
    let expected_parent = game_dir.join("saves");
    if record.directory.parent() != Some(expected_parent.as_path()) {
        return Err("world record is outside the configured save directory".to_owned());
    }
    let root = fs::canonicalize(&expected_parent).map_err(|error| error.to_string())?;
    let target = fs::canonicalize(&record.directory).map_err(|error| error.to_string())?;
    if target.parent() != Some(root.as_path()) {
        return Err("world record resolves outside the configured save directory".to_owned());
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)
                .map_err(|error| format!("cannot copy {}: {error}", entry.path().display()))?;
        }
    }
    Ok(())
}

fn update_record_name(directory: &Path, name: &str) -> Result<(), String> {
    let path = directory.join("world.json");
    let candidates = read_candidates_limited(&path, 1024 * 1024)?;
    if candidates.is_empty() {
        return Ok(());
    }
    let (candidate, original, mut value): (_, _, Value) = candidates
        .iter()
        .find_map(|(candidate, text)| {
            parse_at(text, directory.to_owned())?;
            serde_json::from_str(text)
                .ok()
                .map(|value| (candidate, text, value))
        })
        .ok_or_else(|| format!("cannot parse {} or its backup", path.display()))?;
    if *candidate != path {
        restore_primary(&path, original.as_bytes())?;
    }
    value["name"] = Value::String(name.to_owned());
    let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    atomic_write(&path, text.as_bytes())
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
            Ok(world) => {
                if candidate != path {
                    // Read-only worlds may still be loaded from backup. Report
                    // repair failure, but never discard the recovered state.
                    if let Err(error) = restore_primary(&path, text.as_bytes()) {
                        eprintln!(
                            "world recovered from {} but primary repair failed: {error}",
                            candidate.display()
                        );
                    }
                }
                return Ok(Some(world));
            }
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
    use super::{
        WorldRecord, copy_record, create_named, delete_record, load, load_state, parse,
        rename_record, save_state,
    };

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
    fn world_metadata_recovers_backup_and_hidden_directory_is_not_overwritten() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-metadata-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let record = create_named(
            &root,
            "world",
            42,
            crate::world::WorldSpec::default(),
            crate::world::GameMode::Survival,
            20,
        )
        .unwrap();
        let path = record.directory.join("world.json");
        std::fs::copy(&path, record.directory.join("world.json.bak")).unwrap();
        std::fs::write(&path, "broken").unwrap();
        assert_eq!(load(&root).unwrap()[0].seed, 42);
        rename_record(&root, &record, "renamed").unwrap();
        assert_eq!(load(&root).unwrap()[0].name, "renamed");
        let directory = root.join("saves/renamed");
        let backup = directory.join("world.json.bak");
        assert!(
            super::parse_at(
                &std::fs::read_to_string(&backup).unwrap(),
                directory.clone()
            )
            .is_some()
        );
        std::fs::write(directory.join("world.json"), "null").unwrap();
        super::update_record_name(&directory, "renamed").unwrap();
        assert_eq!(load(&root).unwrap()[0].seed, 42);
        std::fs::create_dir_all(root.join("saves/hidden")).unwrap();
        std::fs::write(root.join("saves/hidden/state.json"), b"protected data").unwrap();
        assert!(
            create_named(
                &root,
                "hidden",
                1,
                crate::world::WorldSpec::default(),
                crate::world::GameMode::Survival,
                20
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(root.join("saves/hidden/state.json")).unwrap(),
            b"protected data"
        );
        std::fs::remove_dir_all(root).unwrap();
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
        let recovered = load_state(&record).unwrap().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&recovered.to_save_string().unwrap())
                .unwrap(),
            serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap()).unwrap()
        );
        assert!(
            crate::world::World::from_save_string(
                &std::fs::read_to_string(directory.join("state.json")).unwrap()
            )
            .is_ok()
        );
        save_state(&record, &recovered).unwrap();
        assert!(
            crate::world::World::from_save_string(
                &std::fs::read_to_string(directory.join("state.json.bak")).unwrap()
            )
            .is_ok()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rust_state_round_trips_through_format_envelope() {
        let directory = std::env::temp_dir().join(format!(
            "minicraft-rust-state-fmt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let record = WorldRecord {
            name: "FMT".to_owned(),
            seed: 11,
            spec: crate::world::WorldSpec::default(),
            mode: crate::world::GameMode::Survival,
            score_minutes: 20,
            directory: directory.clone(),
        };
        let world = crate::world::World::new_with_play_options(
            record.seed,
            record.spec,
            crate::world::PlayOptions::survival(1),
        );
        save_state(&record, &world).unwrap();
        let text = std::fs::read_to_string(directory.join("state.json")).unwrap();
        assert!(text.contains("format"));
        assert!(text.contains('2'));
        assert!(text.contains("2.2.4-rust"));
        let loaded = load_state(&record).unwrap().unwrap();
        assert!(!loaded.to_save_string().unwrap().is_empty());
        // Format 1 is rejected rather than silently accepted.
        let bad = text
            .replace("\"format\": 2", "\"format\": 1")
            .replace("\"format\":2", "\"format\":1");
        assert!(crate::world::World::from_save_string(&bad).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn java_style_world_copy_rename_and_delete_actions_update_records() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-rust-world-actions-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let original = create_named(
            &root,
            "ORIGINAL",
            42,
            crate::world::WorldSpec::default(),
            crate::world::GameMode::Survival,
            20,
        )
        .unwrap();
        let nested = original.directory.join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("marker.txt"), "copied").unwrap();

        copy_record(&root, &original, "COPY").unwrap();
        let copy = load(&root)
            .unwrap()
            .into_iter()
            .find(|record| record.name == "COPY")
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(copy.directory.join("nested/marker.txt")).unwrap(),
            "copied"
        );

        rename_record(&root, &copy, "RENAMED").unwrap();
        let records = load(&root).unwrap();
        assert!(records.iter().any(|record| record.name == "ORIGINAL"));
        let renamed = records
            .into_iter()
            .find(|record| record.name == "RENAMED")
            .unwrap();
        delete_record(&root, &original).unwrap();
        delete_record(&root, &renamed).unwrap();
        assert!(load(&root).unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
