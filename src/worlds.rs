use std::{fs, path::Path};

use serde_json::{Value, json};

use crate::world::WorldSpec;

#[derive(Debug, Clone)]
pub struct WorldRecord {
    pub name: String,
    pub seed: i64,
    pub spec: WorldSpec,
}

pub fn load(game_dir: &Path) -> Result<Vec<WorldRecord>, String> {
    let saves = game_dir.join("saves");
    fs::create_dir_all(&saves)
        .map_err(|error| format!("cannot create {}: {error}", saves.display()))?;
    let mut records = Vec::new();
    let entries = fs::read_dir(&saves)
        .map_err(|error| format!("cannot read {}: {error}", saves.display()))?;
    for entry in entries.flatten() {
        let path = entry.path().join("world.json");
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if let Some(record) = parse(&text) {
            records.push(record);
        }
    }
    records.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(records)
}

pub fn create(game_dir: &Path, seed: i64, spec: WorldSpec) -> Result<WorldRecord, String> {
    let existing = load(game_dir)?;
    let mut number = 1;
    loop {
        let name = format!("WORLD {number}");
        if existing.iter().all(|world| world.name != name) {
            let directory = game_dir.join("saves").join(format!("world-{number}"));
            fs::create_dir_all(&directory)
                .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
            let record = WorldRecord { name, seed, spec };
            let value = json!({
                "format": 1,
                "game_version": "2.2.4-rust",
                "name": record.name,
                "seed": record.seed,
                "size": record.spec.size,
                "theme": record.spec.theme.index(),
                "terrain_type": record.spec.terrain.index(),
                "state": "seed-only",
            });
            let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
            let path = directory.join("world.json");
            fs::write(&path, text)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
            return Ok(record);
        }
        number += 1;
    }
}

fn parse(text: &str) -> Option<WorldRecord> {
    let value: Value = serde_json::from_str(text).ok()?;
    Some(WorldRecord {
        name: value["name"].as_str()?.to_owned(),
        seed: value["seed"].as_i64()?,
        spec: WorldSpec::new(
            value["size"].as_u64().unwrap_or(128) as usize,
            value["theme"].as_u64().unwrap_or(0) as usize,
            value["terrain_type"].as_u64().unwrap_or(0) as usize,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn malformed_world_records_are_ignored() {
        assert!(parse("not json").is_none());
        let world = parse(r#"{"name":"WORLD 1","seed":42}"#).unwrap();
        assert_eq!(world.name, "WORLD 1");
        assert_eq!(world.seed, 42);
        assert_eq!(world.spec, crate::world::WorldSpec::default());
    }
}
