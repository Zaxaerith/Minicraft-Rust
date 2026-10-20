//! Immutable AVL index pages + chunk objects, constant-sized checkpoint root.
//! Windows writer lock covers saving, validation and reachability collection.
#[doc(hidden)]
pub mod diagnostics;
mod dynamic;
#[cfg(test)]
mod dynamic_tests;
mod entities;
mod objects;
mod passive;
#[cfg(test)]
mod passive_tests;
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
pub const LAYERED_NAMESPACE: &str = "infdev-v3";
const FORMAT: &str = "MinicraftRust.Experimental.Infdev.v2";
pub const SURVIVAL_NAMESPACE: &str = "infdev-survival-v4";
const SURVIVAL_FORMAT: &str = "MinicraftRust.Experimental.Infdev.Survival.v4";
const LAYERED_FORMAT: &str = "MinicraftRust.Experimental.Infdev.v3";
const MAX_CHECKPOINT: u64 = 4096;
const AUTO_GC_COMMITS: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Generator {
    LegacyHeightV1,
    JavaSurfaceV1,
    JavaLayersV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    #[serde(default, skip_serializing_if = "is_surface")]
    pub depth: i8,
    pub x: i64,
    pub y: i64,
}
impl Key {
    pub fn at(x: i64, y: i64) -> Self {
        Self::at_depth(0, x, y)
    }
    pub fn at_depth(depth: i8, x: i64, y: i64) -> Self {
        Self {
            depth,
            x: x.div_euclid(64),
            y: y.div_euclid(64),
        }
    }
    pub(super) fn valid(self, layered: bool) -> bool {
        let bounds = i64::MIN / 64..=i64::MAX / 64;
        bounds.contains(&self.x)
            && bounds.contains(&self.y)
            && if layered {
                (-4..=0).contains(&self.depth)
            } else {
                self.depth == 0
            }
    }
    fn local(x: i64, y: i64) -> usize {
        (x.rem_euclid(64) + y.rem_euclid(64) * 64) as usize
    }
}

fn is_surface(depth: &i8) -> bool {
    *depth == 0
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
    #[serde(default, skip_serializing_if = "is_surface")]
    depth: i8,
    root: Option<Object>,
    sequence: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session: Option<Object>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entity_root: Option<Object>,
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
    survival: Option<Box<super::survival::State>>,
    entity_cache: BTreeMap<Key, entities::CachedEntities>,
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
    pub fn depth(&self) -> i8 {
        self.saved.depth
    }
    pub fn is_layered(&self) -> bool {
        self.saved.format == LAYERED_FORMAT || self.is_survival()
    }
    pub fn is_survival(&self) -> bool {
        self.saved.format == SURVIVAL_FORMAT
    }
    pub(crate) fn survival(&self) -> Option<&super::survival::State> {
        self.survival.as_deref()
    }
    pub(crate) fn survival_action(
        &mut self,
        action: super::survival::Action,
    ) -> Result<(), String> {
        if self.act_with_entities(action)? {
            self.checkpoint()?;
        }
        self.trim_entities()?;
        Ok(())
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
        Self::create_inner(root, name, seed, capacity, None, FORMAT)
    }

    pub fn create_layers(
        root: &Path,
        name: &str,
        seed: i64,
        capacity: usize,
    ) -> Result<Self, String> {
        Self::create_inner(root, name, seed, capacity, None, LAYERED_FORMAT)
    }

    pub fn create_survival(
        root: &Path,
        name: &str,
        seed: i64,
        capacity: usize,
    ) -> Result<Self, String> {
        Self::create_inner(root, name, seed, capacity, None, SURVIVAL_FORMAT)
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
        Self::create_inner(root, name, legacy.seed(), capacity, Some(&legacy), FORMAT)
    }

    fn create_inner(
        root: &Path,
        name: &str,
        seed: i64,
        capacity: usize,
        legacy: Option<&super::InfdevWorld>,
        format: &str,
    ) -> Result<Self, String> {
        let layered = format != FORMAT;
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
        let namespace = root.join(if format == SURVIVAL_FORMAT {
            SURVIVAL_NAMESPACE
        } else if layered {
            LAYERED_NAMESPACE
        } else {
            NAMESPACE
        });
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
        if format == SURVIVAL_FORMAT {
            fs::create_dir(stage.0.join("sessions")).map_err(|e| e.to_string())?;
        }
        let handle = lock(&stage.0)?;
        let mut w = Self {
            directory: stage.0.clone(),
            _lock: handle,
            survival: (format == SURVIVAL_FORMAT)
                .then(|| Box::new(super::survival::State::new(seed))),
            saved: Checkpoint {
                format: format.into(),
                generator: if layered {
                    Generator::JavaLayersV1
                } else if legacy.is_some() {
                    Generator::LegacyHeightV1
                } else {
                    Generator::JavaSurfaceV1
                },
                seed,
                position: (0, 0),
                depth: 0,
                root: None,
                sequence: 0,
                session: None,
                entity_root: None,
            },
            entity_cache: BTreeMap::new(),
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
                        depth: 0,
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
        if w.is_survival() {
            let spawn = super::survival::find_spawn(&mut w)?;
            w.visit(spawn.0, spawn.1)?;
            w.survival.as_mut().expect("new Survival").spawn = spawn;
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
        let mut identity = None;
        for (candidate, text) in candidates {
            Self::refuse_version_downgrade(&text)?;
            let saved = match Self::parse_checkpoint(&text) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };
            let candidate_identity = (saved.format.clone(), saved.generator, saved.seed);
            if identity
                .as_ref()
                .is_some_and(|expected| expected != &candidate_identity)
            {
                return Err(
                    "backup checkpoint belongs to a different world format/generator/seed".into(),
                );
            }
            identity = Some(candidate_identity);
            let mut w = Self {
                directory: directory.into(),
                _lock: handle.try_clone().map_err(|e| e.to_string())?,
                saved,
                survival: None,
                entity_cache: BTreeMap::new(),
                resident: BTreeMap::new(),
                nodes: BTreeMap::new(),
                capacity,
                clock: 0,
                recovered: candidate != path,
                last_gc_issue: None,
            };
            let valid: Result<(), String> = (|| {
                w.validate_root(w.saved.root)?;
                w.validate_entity_root(w.saved.entity_root)?;
                w.survival = w.read_session(&w.saved)?;
                let p = w.position();
                if w.find(w.saved.root, Key::at_depth(w.depth(), p.0, p.1))?
                    .is_none()
                {
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
        let contract = match (saved.format.as_str(), saved.generator) {
            (FORMAT, Generator::LegacyHeightV1 | Generator::JavaSurfaceV1) => {
                saved.depth == 0 && saved.session.is_none() && saved.entity_root.is_none()
            }
            (LAYERED_FORMAT, Generator::JavaLayersV1) => {
                (-4..=0).contains(&saved.depth)
                    && saved.session.is_none()
                    && saved.entity_root.is_none()
            }
            (SURVIVAL_FORMAT, Generator::JavaLayersV1) => {
                (-4..=0).contains(&saved.depth) && saved.session.is_some()
            }
            _ => false,
        };
        if !contract {
            return Err("invalid Infdev format/generator/depth contract".into());
        }
        Ok(saved)
    }

    fn refuse_version_downgrade(text: &str) -> Result<(), String> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return Ok(());
        };
        let format = value.get("format").and_then(|v| v.as_str());
        let generator = value.get("generator").and_then(|v| v.as_str());
        if (format != Some(SURVIVAL_FORMAT)
            && value.get("entity_root").is_some_and(|v| !v.is_null()))
            || (format == Some(SURVIVAL_FORMAT) && value.get("session").is_none_or(|v| v.is_null()))
            || (matches!(format, Some(FORMAT | LAYERED_FORMAT))
                && value.get("session").is_some_and(|v| !v.is_null()))
            || format.is_some_and(|f| f != FORMAT && f != LAYERED_FORMAT && f != SURVIVAL_FORMAT)
            || generator
                .is_some_and(|g| !matches!(g, "LegacyHeightV1" | "JavaSurfaceV1" | "JavaLayersV1"))
            || matches!((format, generator), (Some(FORMAT), Some("JavaLayersV1")))
            || matches!(
                (format, generator),
                (
                    Some(LAYERED_FORMAT | SURVIVAL_FORMAT),
                    Some("LegacyHeightV1" | "JavaSurfaceV1")
                )
            )
            || value.get("depth").is_some_and(|depth| {
                depth.as_i64().is_none_or(|d| {
                    if matches!(format, Some(LAYERED_FORMAT | SURVIVAL_FORMAT)) {
                        !(-4..=0).contains(&d)
                    } else {
                        d != 0
                    }
                })
            })
        {
            return Err("unknown or incompatible checkpoint/generator/depth version; downgrade recovery refused".into());
        }
        Ok(())
    }

    fn ensure(&mut self, key: Key) -> Result<(), String> {
        if !key.valid(self.is_layered()) {
            return Err("chunk outside this world's depth/signed 64-bit tile domain".into());
        }
        if !self.resident.contains_key(&key) {
            let reference = self.find(self.saved.root, key)?;
            let cells = match reference {
                Some(r) => self.read_chunk(r)?,
                None => match self.generator() {
                    Generator::JavaSurfaceV1 => terrain::generate(self.seed(), key.x, key.y),
                    Generator::JavaLayersV1 => {
                        super::stairs::generate(self.seed(), key.depth, key.x, key.y)?
                    }
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
        self.cell_at(self.depth(), x, y)
    }
    pub fn cell_at(&mut self, depth: i8, x: i64, y: i64) -> Result<Cell, String> {
        let key = Key::at_depth(depth, x, y);
        self.ensure(key)?;
        Ok(self.resident[&key].cells[Key::local(x, y)])
    }
    pub fn set_cell(&mut self, x: i64, y: i64, cell: Cell) -> Result<(), String> {
        self.set_cell_at(self.depth(), x, y, cell)
    }
    pub fn set_cell_at(&mut self, depth: i8, x: i64, y: i64, cell: Cell) -> Result<(), String> {
        if !self.is_layered() && cell.ground as u8 > Ground::StairsDown as u8 {
            return Err("layered ground cannot be written to a v2 world".into());
        }
        if !self.is_survival() && cell.ground as u8 > Ground::ObsidianDoor as u8 {
            return Err("Survival ground cannot be written to an exploration world".into());
        }
        let key = Key::at_depth(depth, x, y);
        self.ensure(key)?;
        let c = self.resident.get_mut(&key).expect("loaded");
        if c.cells[Key::local(x, y)] != cell {
            c.cells[Key::local(x, y)] = cell;
            c.dirty = true;
        }
        Ok(())
    }
    pub fn visit(&mut self, x: i64, y: i64) -> Result<(), String> {
        self.visit_layer(self.depth(), x, y)
    }
    pub fn visit_layer(&mut self, depth: i8, x: i64, y: i64) -> Result<(), String> {
        self.ensure(Key::at_depth(depth, x, y))?;
        self.saved.position = (x, y);
        self.saved.depth = depth;
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
        self.entity_cache.clear();
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
            if existing.seed != self.seed()
                || existing.generator != self.generator()
                || existing.format != self.saved.format
            {
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
        let session = self
            .survival
            .as_ref()
            .map(|state| {
                state.validate()?;
                objects::write(
                    &self.directory,
                    objects::Domain::Sessions,
                    &serde_json::to_vec(state).map_err(|e| e.to_string())?,
                )
            })
            .transpose()?;
        let dirty: Vec<_> = self
            .resident
            .iter()
            .filter(|(_, c)| c.dirty)
            .map(|(&k, c)| (k, c.cells.clone()))
            .collect();
        let entity_root = self.write_entity_pages()?;
        if dirty.is_empty() && self.directory.join("checkpoint.json").exists() {
            let old = crate::saveload::storage::read_text_limited(
                &self.directory.join("checkpoint.json"),
                MAX_CHECKPOINT,
            )?;
            let mut desired = self.saved.clone();
            desired.session = session;
            desired.entity_root = entity_root;
            if Self::parse_checkpoint(&old)? == desired {
                before_publish()?;
                for page in self.entity_cache.values_mut() {
                    page.dirty = false;
                }
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
        next.session = session;
        next.entity_root = entity_root;
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
        for page in self.entity_cache.values_mut() {
            page.dirty = false;
        }
        for chunk in self.resident.values_mut() {
            chunk.dirty = false;
        }
        Ok(())
    }
}
