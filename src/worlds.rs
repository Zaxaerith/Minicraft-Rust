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
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let mut value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
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
