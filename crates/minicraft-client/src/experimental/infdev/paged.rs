//! Immutable AVL index pages + chunk objects, constant-sized checkpoint root.
//! Windows writer lock covers saving, validation and reachability collection.
mod objects;
#[cfg(test)]
mod tests;

use super::{
    Cell, Ground,
    store::{lock, real_path},
    terrain,
};
use crate::saveload::storage::{atomic_write, read_candidates_limited, restore_primary};
use objects::Object;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    path::{Path, PathBuf},
};

pub const NAMESPACE: &str = "infdev-v2";
const FORMAT: &str = "MinicraftRust.Experimental.Infdev.v2";
const MAX_CHECKPOINT: u64 = 4096;
const AUTO_GC_COMMITS: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Generator {
    LegacyHeightV1,
    JavaSurfaceV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub x: i64,
    pub y: i64,
}
impl Key {
    pub fn at(x: i64, y: i64) -> Self {
        Self {
            x: x.div_euclid(64),
            y: y.div_euclid(64),
        }
    }
    fn valid(self) -> bool {
        let bounds = i64::MIN / 64..=i64::MAX / 64;
        bounds.contains(&self.x) && bounds.contains(&self.y)
    }
    fn local(x: i64, y: i64) -> usize {
        (x.rem_euclid(64) + y.rem_euclid(64) * 64) as usize
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    key: Key,
    chunk: Object,
    left: Option<Object>,
    right: Option<Object>,
    height: u8,
    count: u64,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    format: String,
    generator: Generator,
    seed: i64,
    position: (i64, i64),
    root: Option<Object>,
    sequence: u64,
}

struct Cached {
    cells: Vec<Cell>,
    dirty: bool,
    touched: u64,
}

pub struct World {
    directory: PathBuf,
    _lock: File,
    saved: Checkpoint,
    resident: BTreeMap<Key, Cached>,
    nodes: BTreeMap<Object, (Node, u64)>,
    capacity: usize,
    clock: u64,
    recovered: bool,
    last_gc_issue: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Collection {
    pub removed_files: u64,
    pub removed_bytes: u64,
    pub retained_objects: u64,
    pub unknown_files: u64,
}

impl World {
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn seed(&self) -> i64 {
        self.saved.seed
    }
    pub fn generator(&self) -> Generator {
        self.saved.generator
    }
    pub fn collection_warning(&self) -> Option<&str> {
        self.last_gc_issue.as_deref()
    }
    pub fn position(&self) -> (i64, i64) {
        self.saved.position
    }
    pub fn resident_count(&self) -> usize {
        self.resident.len()
    }
    pub fn recovered_backup(&self) -> bool {
        self.recovered
    }
    pub fn saved_chunk_count(&mut self) -> Result<u64, String> {
        self.count(self.saved.root)
    }

    fn check_capacity(capacity: usize) -> Result<(), String> {
        if !(1..=64).contains(&capacity) {
            return Err("resident capacity must be 1..64".into());
        }
        Ok(())
    }

    pub fn create(root: &Path, name: &str, seed: i64, capacity: usize) -> Result<Self, String> {
        Self::create_inner(root, name, seed, capacity, None)
    }

    /// Explicit copy upgrade; source is locked/read-only, including on recovery.
    /// Future unvisited chunks KEEP the old generator contract.
    pub fn upgrade_legacy(
        source: &Path,
        root: &Path,
        name: &str,
        capacity: usize,
    ) -> Result<Self, String> {
        let legacy = super::InfdevWorld::open_snapshot(source)?;
        Self::create_inner(root, name, legacy.seed(), capacity, Some(&legacy))
    }

    fn create_inner(
        root: &Path,
        name: &str,
        seed: i64,
        capacity: usize,
        legacy: Option<&super::InfdevWorld>,
    ) -> Result<Self, String> {
        Self::check_capacity(capacity)?;
        if name.is_empty()
            || name.len() > 64
            || name.starts_with('.')
            || name.ends_with(['.', ' '])
            || name
                .chars()
                .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        {
            return Err("invalid experimental world name".into());
        }
        real_path(root, true)?;
        let namespace = root.join(NAMESPACE);
        match fs::create_dir(&namespace) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.to_string()),
        }
        real_path(&namespace, true)?;
        let destination = namespace.join(name);
        if fs::symlink_metadata(&destination).is_ok() {
            return Err("destination already exists; never replaced".into());
        }
        let stage = objects::Stage::create(&namespace)?;
        fs::create_dir(stage.0.join("chunks")).map_err(|e| e.to_string())?;
        fs::create_dir(stage.0.join("nodes")).map_err(|e| e.to_string())?;
        let handle = lock(&stage.0)?;
        let mut w = Self {
            directory: stage.0.clone(),
            _lock: handle,
            saved: Checkpoint {
                format: FORMAT.into(),
                generator: if legacy.is_some() {
                    Generator::LegacyHeightV1
                } else {
                    Generator::JavaSurfaceV1
                },
                seed,
                position: (0, 0),
                root: None,
                sequence: 0,
            },
            resident: BTreeMap::new(),
            nodes: BTreeMap::new(),
            capacity,
            clock: 0,
            recovered: false,
            last_gc_issue: None,
        };
        if let Some(old) = legacy {
            let mut entries = Vec::with_capacity(old.committed.len());
            for (&key, &object) in &old.committed {
                let cells = old.read_chunk(key, object)?;
                let reference = w.write_chunk(&cells)?;
                entries.push((
                    Key {
                        x: i64::from(key.x),
                        y: i64::from(key.y),
                    },
                    reference,
                ));
            }
            w.saved.root = w.build_balanced(&entries)?;
            w.saved.position = (i64::from(old.position.0), i64::from(old.position.1));
        } else {
            w.ensure(Key::at(0, 0))?;
        }
        w.checkpoint()?;
        w.validate_root(w.saved.root)?;
        drop(w); // Close Windows child handle before publishing its directory.
        crate::saveload::migration::publish_exclusive(&stage.0, &destination)
            .map_err(|e| format!("cannot exclusively publish experimental world: {e}"))?;
        let mut converted = Self::open(&destination, capacity)?;
        converted.recovered |= legacy.is_some_and(|old| old.recovered_backup());
        Ok(converted)
    }

    pub fn open(directory: &Path, capacity: usize) -> Result<Self, String> {
        Self::check_capacity(capacity)?;
        real_path(directory, true)?;
        if directory
            .join("state.json")
            .try_exists()
            .map_err(|e| e.to_string())?
            || directory
                .join("world.json")
                .try_exists()
                .map_err(|e| e.to_string())?
        {
            return Err("fixed-world save refused".into());
        }
        real_path(&directory.join("nodes"), true)?;
        real_path(&directory.join("chunks"), true)?;
        let handle = lock(directory)?;
        let path = directory.join("checkpoint.json");
        let candidates = objects::checkpoints(directory)?;
        let mut errors = Vec::new();
        for (candidate, text) in candidates {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
                && (value
                    .get("format")
                    .and_then(|v| v.as_str())
                    .is_some_and(|f| f != FORMAT)
                    || value
                        .get("generator")
                        .and_then(|v| v.as_str())
                        .is_some_and(|g| g != "LegacyHeightV1" && g != "JavaSurfaceV1"))
            {
                return Err(
                    "unknown checkpoint/generator version; downgrade recovery refused".into(),
                );
            }
            let saved = match Self::parse_checkpoint(&text) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };
            let mut w = Self {
                directory: directory.into(),
                _lock: handle.try_clone().map_err(|e| e.to_string())?,
                saved,
                resident: BTreeMap::new(),
                nodes: BTreeMap::new(),
                capacity,
                clock: 0,
                recovered: candidate != path,
                last_gc_issue: None,
            };
            let valid: Result<(), String> = (|| {
                w.validate_root(w.saved.root)?;
                let p = w.position();
                if w.find(w.saved.root, Key::at(p.0, p.1))?.is_none() {
                    return Err("explorer chunk missing from checkpoint".into());
                }
                Ok(())
            })();
            match valid {
                Ok(()) => {
                    if w.recovered {
                        // Ambiguous previous root/data may still be valuable. Pin the
                        // entire store against GC until explicitly investigated.
                        objects::hold_recovery(directory)?;
                        restore_primary(&path, text.as_bytes())?;
                    }
                    return Ok(w);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }
        Err(format!(
            "no valid complete checkpoint: {}",
            errors.join("; ")
        ))
    }

    fn parse_checkpoint(text: &str) -> Result<Checkpoint, String> {
        let saved: Checkpoint = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if saved.format != FORMAT {
            return Err("unknown Infdev checkpoint format".into());
        }
        Ok(saved)
    }

    fn ensure(&mut self, key: Key) -> Result<(), String> {
        if !key.valid() {
            return Err("chunk outside signed 64-bit tile domain".into());
        }
        if !self.resident.contains_key(&key) {
            let reference = self.find(self.saved.root, key)?;
            let cells = match reference {
                Some(r) => self.read_chunk(r)?,
                None => match self.generator() {
                    Generator::JavaSurfaceV1 => terrain::generate(self.seed(), key.x, key.y),
                    Generator::LegacyHeightV1 => {
                        // Match the old contract at all old representable coordinates.
                        let coord = super::ChunkCoord::at(
                            key.x.wrapping_mul(64) as i32,
                            key.y.wrapping_mul(64) as i32,
                        );
                        super::generation::generate(self.seed(), coord)
                    }
                },
            };
            if self.resident.len() == self.capacity {
                let oldest = *self
                    .resident
                    .iter()
                    .min_by_key(|(_, c)| c.touched)
                    .expect("nonzero capacity")
                    .0;
                if self.resident.values().any(|c| c.dirty) {
                    self.checkpoint()?;
                }
                self.resident.remove(&oldest);
            }
            self.resident.insert(
                key,
                Cached {
                    cells,
                    dirty: reference.is_none(),
                    touched: 0,
                },
            );
        }
        self.clock = self.clock.wrapping_add(1);
        self.resident.get_mut(&key).expect("loaded").touched = self.clock;
        Ok(())
    }

    pub fn cell(&mut self, x: i64, y: i64) -> Result<Cell, String> {
        let key = Key::at(x, y);
        self.ensure(key)?;
        Ok(self.resident[&key].cells[Key::local(x, y)])
    }
    pub fn set_cell(&mut self, x: i64, y: i64, cell: Cell) -> Result<(), String> {
        let key = Key::at(x, y);
        self.ensure(key)?;
        let c = self.resident.get_mut(&key).expect("loaded");
        if c.cells[Key::local(x, y)] != cell {
            c.cells[Key::local(x, y)] = cell;
            c.dirty = true;
        }
        Ok(())
    }
    pub fn visit(&mut self, x: i64, y: i64) -> Result<(), String> {
        self.ensure(Key::at(x, y))?;
        self.saved.position = (x, y);
        Ok(())
    }
    pub fn step(&mut self, dx: i32, dy: i32) -> Result<(), String> {
        if !matches!((dx, dy), (-1 | 1, 0) | (0, -1 | 1)) {
            return Err("one cardinal step required".into());
        }
        self.visit(
            self.position()
                .0
                .checked_add(i64::from(dx))
                .ok_or("x overflow")?,
            self.position()
                .1
                .checked_add(i64::from(dy))
                .ok_or("y overflow")?,
        )
    }
    pub fn unload_all(&mut self) -> Result<(), String> {
        self.checkpoint()?;
        self.resident.clear();
        Ok(())
    }

    pub fn checkpoint(&mut self) -> Result<(), String> {
        let previous = self.saved.sequence;
        self.commit_with_hook(|| Ok(()))?;
        if self.saved.sequence != previous
            && self
                .saved
                .sequence
                .is_multiple_of(u64::from(AUTO_GC_COMMITS))
        {
            self.last_gc_issue = self.collect_inner().err();
        }
        Ok(())
    }

    fn commit_with_hook(
        &mut self,
        before_publish: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        // Never rotate a corrupt primary over its good backup during a live
        // session. Ambiguous backup metadata pins objects before replacement.
        let primary = self.directory.join("checkpoint.json");
        if primary.try_exists().map_err(|e| e.to_string())? {
            let text = crate::saveload::storage::read_text_limited(&primary, MAX_CHECKPOINT)?;
            let existing = Self::parse_checkpoint(&text)?;
            if existing.seed != self.seed() || existing.generator != self.generator() {
                return Err("live checkpoint identity changed".into());
            }
        }
        let backup = crate::saveload::backup_path(&primary);
        if backup.try_exists().map_err(|e| e.to_string())? {
            let valid = crate::saveload::storage::read_text_limited(&backup, MAX_CHECKPOINT)
                .and_then(|text| Self::parse_checkpoint(&text));
            if valid.is_err() {
                objects::hold_recovery(&self.directory)?;
            }
        }
        let dirty: Vec<_> = self
            .resident
            .iter()
            .filter(|(_, c)| c.dirty)
            .map(|(&k, c)| (k, c.cells.clone()))
            .collect();
        if dirty.is_empty() && self.directory.join("checkpoint.json").exists() {
            let old = crate::saveload::storage::read_text_limited(
                &self.directory.join("checkpoint.json"),
                MAX_CHECKPOINT,
            )?;
            if Self::parse_checkpoint(&old)? == self.saved {
                before_publish()?;
                return Ok(());
            }
        }
        let mut root = self.saved.root;
        for (key, cells) in dirty {
            let reference = self.write_chunk(&cells)?;
            root = Some(self.insert(root, key, reference, 0)?);
        }
        let mut next = self.saved.clone();
        next.root = root;
        next.sequence = next
            .sequence
            .checked_add(1)
            .ok_or("checkpoint sequence exhausted")?;
        let bytes = serde_json::to_vec(&next).map_err(|e| e.to_string())?;
        before_publish()?;
        // A previous atomic write may have installed the primary but failed to
        // rotate its backup. Acknowledge identical committed bytes on retry;
        // do not overwrite the last older backup with a duplicate new root.
        let already_installed =
            crate::saveload::storage::read_text_limited(&primary, MAX_CHECKPOINT)
                .is_ok_and(|text| text.as_bytes() == bytes);
        if !already_installed {
            atomic_write(&primary, &bytes)?;
        }
        self.saved = next;
        for chunk in self.resident.values_mut() {
            chunk.dirty = false;
        }
        Ok(())
    }
}
