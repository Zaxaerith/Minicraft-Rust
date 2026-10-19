//! Bounded immutable object I/O; filenames never come from save strings.
use super::*;
use std::{
    io::{Read, Write},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Object {
    hash: u64,
    slot: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Domain {
    Nodes,
    Chunks,
    Sessions,
}
impl Domain {
    fn name(self) -> &'static str {
        match self {
            Self::Nodes => "nodes",
            Self::Chunks => "chunks",
            Self::Sessions => "sessions",
        }
    }
    fn suffix(self) -> &'static str {
        match self {
            Self::Nodes => "node",
            Self::Chunks => "chunk",
            Self::Sessions => "session",
        }
    }
    fn limit(self) -> usize {
        match self {
            Self::Nodes => 1024,
            Self::Sessions => 256 * 1024,
            Self::Chunks => 8 + 4096 * 3,
        }
    }
}
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    })
}
fn filename(domain: Domain, object: Object) -> String {
    format!(
        "{:016x}-{:04x}.{}",
        object.hash,
        object.slot,
        domain.suffix()
    )
}
pub(super) fn path(root: &Path, domain: Domain, object: Object) -> PathBuf {
    root.join(domain.name())
        .join(format!("{:02x}", object.hash >> 56))
        .join(filename(domain, object))
}

pub(super) fn read(root: &Path, domain: Domain, object: Object) -> Result<Vec<u8>, String> {
    if object.slot >= 4096 {
        return Err("invalid object slot".into());
    }
    let p = path(root, domain, object);
    real_path(p.parent().ok_or("object parent missing")?, true)?;
    real_path(&p, false)?;
    let mut bytes = Vec::new();
    File::open(&p)
        .map_err(|e| format!("{}: {e}", p.display()))?
        .take((domain.limit() + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > domain.limit() || checksum(&bytes) != object.hash {
        return Err(format!("corrupt immutable object: {}", p.display()));
    }
    Ok(bytes)
}

pub(super) fn write(root: &Path, domain: Domain, bytes: &[u8]) -> Result<Object, String> {
    write_with_sync(root, domain, bytes, File::sync_all)
}

pub(super) fn write_with_sync(
    root: &Path,
    domain: Domain,
    bytes: &[u8],
    mut sync: impl FnMut(&File) -> std::io::Result<()>,
) -> Result<Object, String> {
    if bytes.len() > domain.limit() {
        return Err("object size exceeds format bound".into());
    }
    let hash = checksum(bytes);
    let bucket = root.join(domain.name()).join(format!("{:02x}", hash >> 56));
    real_path(&root.join(domain.name()), true)?;
    match fs::create_dir(&bucket) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.to_string()),
    }
    real_path(&bucket, true)?;
    for slot in 0..4096 {
        let object = Object { hash, slot };
        let p = path(root, domain, object);
        match fs::OpenOptions::new().create_new(true).write(true).open(&p) {
            Ok(mut f) => {
                f.write_all(bytes)
                    .and_then(|_| sync(&f))
                    .map_err(|e| e.to_string())?;
                return Ok(object);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if let Ok(old) = read(root, domain, object) {
                    if old != bytes {
                        return Err("object checksum collision; replacement refused".into());
                    }
                    // A previous attempt may have written all bytes but failed
                    // to sync. Byte equality alone is not a durable commit.
                    let f = fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&p)
                        .map_err(|e| e.to_string())?;
                    sync(&f).map_err(|e| e.to_string())?;
                    return Ok(object);
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("object collision/torn-file slots exhausted".into())
}

pub(super) fn checkpoints(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let p = root.join("checkpoint.json");
    for candidate in [&p, &crate::saveload::backup_path(&p)] {
        if fs::symlink_metadata(candidate).is_ok() {
            real_path(candidate, false)?;
        }
    }
    read_candidates_limited(&p, MAX_CHECKPOINT)
}

pub(super) fn hold_recovery(root: &Path) -> Result<(), String> {
    let p = root.join("gc-recovery.hold");
    match fs::OpenOptions::new().create_new(true).write(true).open(&p) {
        Ok(mut f) => {
            let notice = b"Ambiguous recovery state: automatic collection disabled; preserve all objects for investigation.\n";
            f.write_all(notice)
                .and_then(|_| f.sync_all())
                .map_err(|e| e.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            real_path(&p, false)?;
            // Reattempt durability if an earlier marker sync failed.
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&p)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub(super) struct Stage(pub PathBuf);
impl Stage {
    pub fn create(parent: &Path) -> Result<Self, String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let p = parent.join(format!(
                ".new-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&p) {
                Ok(()) => return Ok(Self(p)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if self.0.exists() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

impl World {
    pub(super) fn write_chunk(&self, cells: &[Cell]) -> Result<Object, String> {
        if cells.len() != 4096 {
            return Err("invalid chunk cell count".into());
        }
        let mut bytes = Vec::with_capacity(Domain::Chunks.limit());
        bytes.extend_from_slice(if self.is_survival() {
            b"MCIFCH04"
        } else if self.is_layered() {
            b"MCIFCH03"
        } else {
            b"MCIFCH02"
        });
        for c in cells {
            if !self.is_layered() && c.ground as u8 > Ground::StairsDown as u8 {
                return Err("layered ground cannot be written to a v2 object".into());
            }
            if !self.is_survival() && c.ground as u8 > Ground::ObsidianDoor as u8 {
                return Err("Survival ground in old object".into());
            }
            bytes.push(c.ground as u8);
            bytes.extend_from_slice(&c.data.to_le_bytes());
        }
        write(&self.directory, Domain::Chunks, &bytes)
    }
    pub(super) fn read_chunk(&self, reference: Object) -> Result<Vec<Cell>, String> {
        let bytes = read(&self.directory, Domain::Chunks, reference)?;
        let header = if self.is_survival() {
            b"MCIFCH04"
        } else if self.is_layered() {
            b"MCIFCH03"
        } else {
            b"MCIFCH02"
        };
        if bytes.len() != Domain::Chunks.limit() || &bytes[..8] != header {
            return Err("invalid complete chunk object".into());
        }
        bytes[8..]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|b| {
                Ok(Cell {
                    ground: match b[0] {
                        0 => Ground::Grass,
                        1 => Ground::Water,
                        2 => Ground::Sand,
                        3 => Ground::Rock,
                        4 => Ground::Tree,
                        5 => Ground::Flower,
                        6 => Ground::Cactus,
                        7 => Ground::StairsDown,
                        8 if self.is_layered() => Ground::Dirt,
                        9 if self.is_layered() => Ground::IronOre,
                        10 if self.is_layered() => Ground::GoldOre,
                        11 if self.is_layered() => Ground::GemOre,
                        12 if self.is_layered() => Ground::Lapis,
                        13 if self.is_layered() => Ground::Lava,
                        14 if self.is_layered() => Ground::ObsidianWall,
                        15 if self.is_layered() => Ground::Obsidian,
                        16 if self.is_layered() => Ground::RawObsidian,
                        17 if self.is_layered() => Ground::OrnateObsidian,
                        18 if self.is_layered() => Ground::StairsUp,
                        19 if self.is_layered() => Ground::ObsidianDoor,
                        20 if self.is_survival() => Ground::Hole,
                        21 if self.is_survival() => Ground::WoodFloor,
                        22 if self.is_survival() => Ground::RawStone,
                        23 if self.is_survival() => Ground::StoneFloor,
                        _ => return Err("unknown ground ID".into()),
                    },
                    data: u16::from_le_bytes([b[1], b[2]]),
                })
            })
            .collect()
    }
    pub(super) fn node(&mut self, reference: Object) -> Result<Node, String> {
        self.clock = self.clock.wrapping_add(1);
        if let Some((n, touched)) = self.nodes.get_mut(&reference) {
            *touched = self.clock;
            return Ok(n.clone());
        }
        let bytes = read(&self.directory, Domain::Nodes, reference)?;
        let n: Node = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if !n.key.valid(self.is_layered()) || n.height == 0 || n.height > 128 || n.count == 0 {
            return Err("invalid index page".into());
        }
        if self.nodes.len() >= 512 {
            let oldest = *self
                .nodes
                .iter()
                .min_by_key(|(_, (_, t))| *t)
                .expect("cache full")
                .0;
            self.nodes.remove(&oldest);
        }
        self.nodes.insert(reference, (n.clone(), self.clock));
        Ok(n)
    }
    fn height(&mut self, r: Option<Object>) -> Result<u8, String> {
        r.map(|r| self.node(r).map(|n| n.height)).unwrap_or(Ok(0))
    }
    pub(super) fn count(&mut self, r: Option<Object>) -> Result<u64, String> {
        r.map(|r| self.node(r).map(|n| n.count)).unwrap_or(Ok(0))
    }
    fn write_node(&mut self, mut n: Node) -> Result<Object, String> {
        n.height = self
            .height(n.left)?
            .max(self.height(n.right)?)
            .checked_add(1)
            .ok_or("index height overflow")?;
        n.count = self
            .count(n.left)?
            .checked_add(self.count(n.right)?)
            .and_then(|n| n.checked_add(1))
            .ok_or("index count overflow")?;
        let bytes = serde_json::to_vec(&n).map_err(|e| e.to_string())?;
        let r = write(&self.directory, Domain::Nodes, &bytes)?;
        if self.nodes.len() >= 512 {
            let oldest = *self
                .nodes
                .iter()
                .min_by_key(|(_, (_, t))| *t)
                .expect("cache full")
                .0;
            self.nodes.remove(&oldest);
        }
        self.clock = self.clock.wrapping_add(1);
        self.nodes.insert(r, (n, self.clock));
        Ok(r)
    }
    fn rotate_right(&mut self, mut n: Node) -> Result<Object, String> {
        let mut l = self.node(n.left.ok_or("missing rotation child")?)?;
        n.left = l.right;
        l.right = Some(self.write_node(n)?);
        self.write_node(l)
    }
    fn rotate_left(&mut self, mut n: Node) -> Result<Object, String> {
        let mut r = self.node(n.right.ok_or("missing rotation child")?)?;
        n.right = r.left;
        r.left = Some(self.write_node(n)?);
        self.write_node(r)
    }
    fn balance(&mut self, mut n: Node) -> Result<Object, String> {
        let left = self.height(n.left)?;
        let right = self.height(n.right)?;
        if i16::from(left) - i16::from(right) > 1 {
            let l = self.node(n.left.ok_or("left branch missing")?)?;
            if self.height(l.left)? < self.height(l.right)? {
                n.left = Some(self.rotate_left(l)?);
            }
            self.rotate_right(n)
        } else if i16::from(right) - i16::from(left) > 1 {
            let r = self.node(n.right.ok_or("right branch missing")?)?;
            if self.height(r.right)? < self.height(r.left)? {
                n.right = Some(self.rotate_right(r)?);
            }
            self.rotate_left(n)
        } else {
            self.write_node(n)
        }
    }
    pub(super) fn insert(
        &mut self,
        root: Option<Object>,
        key: Key,
        chunk: Object,
        depth: u8,
    ) -> Result<Object, String> {
        if depth >= 128 {
            return Err("index path too deep".into());
        }
        let Some(root) = root else {
            return self.write_node(Node {
                key,
                chunk,
                left: None,
                right: None,
                height: 1,
                count: 1,
            });
        };
        let mut n = self.node(root)?;
        match key.cmp(&n.key) {
            std::cmp::Ordering::Less => {
                n.left = Some(self.insert(n.left, key, chunk, depth + 1)?)
            }
            std::cmp::Ordering::Greater => {
                n.right = Some(self.insert(n.right, key, chunk, depth + 1)?)
            }
            std::cmp::Ordering::Equal => {
                if chunk == n.chunk {
                    return Ok(root);
                }
                n.chunk = chunk;
            }
        }
        self.balance(n)
    }
    pub(super) fn build_balanced(
        &mut self,
        entries: &[(Key, Object)],
    ) -> Result<Option<Object>, String> {
        if entries.is_empty() {
            return Ok(None);
        }
        let middle = entries.len() / 2;
        let (key, chunk) = entries[middle];
        let left = self.build_balanced(&entries[..middle])?;
        let right = self.build_balanced(&entries[middle + 1..])?;
        self.write_node(Node {
            key,
            chunk,
            left,
            right,
            height: 1,
            count: 1,
        })
        .map(Some)
    }
    pub(super) fn find(
        &mut self,
        mut root: Option<Object>,
        key: Key,
    ) -> Result<Option<Object>, String> {
        for _ in 0..128 {
            let Some(r) = root else {
                return Ok(None);
            };
            let n = self.node(r)?;
            match key.cmp(&n.key) {
                std::cmp::Ordering::Equal => return Ok(Some(n.chunk)),
                std::cmp::Ordering::Less => root = n.left,
                std::cmp::Ordering::Greater => root = n.right,
            }
        }
        Err("index depth/cycle violation".into())
    }

    pub(super) fn validate_root(
        &mut self,
        root: Option<Object>,
    ) -> Result<HashSet<(Domain, Object)>, String> {
        let mut marked = HashSet::new();
        self.walk(root, None, None, 0, &mut marked)?;
        Ok(marked)
    }
    fn walk(
        &mut self,
        root: Option<Object>,
        lower: Option<Key>,
        upper: Option<Key>,
        depth: u8,
        marked: &mut HashSet<(Domain, Object)>,
    ) -> Result<(u8, u64), String> {
        let Some(r) = root else {
            return Ok((0, 0));
        };
        if depth >= 128 {
            return Err("index path too deep".into());
        }
        let n = self.node(r)?;
        if lower.is_some_and(|k| n.key <= k) || upper.is_some_and(|k| n.key >= k) {
            return Err("index key order/cycle violation".into());
        }
        marked.insert((Domain::Nodes, r));
        if marked.insert((Domain::Chunks, n.chunk)) {
            self.read_chunk(n.chunk)?;
        }
        let (lh, lc) = self.walk(n.left, lower, Some(n.key), depth + 1, marked)?;
        let (rh, rc) = self.walk(n.right, Some(n.key), upper, depth + 1, marked)?;
        let count = lc
            .checked_add(rc)
            .and_then(|n| n.checked_add(1))
            .ok_or("index count overflow")?;
        let height = lh.max(rh) + 1;
        if lh.abs_diff(rh) > 1 || height != n.height || count != n.count {
            return Err("unbalanced/corrupt index metadata".into());
        }
        Ok((height, count))
    }

    pub(super) fn read_session(
        &self,
        cp: &Checkpoint,
    ) -> Result<Option<Box<super::super::survival::State>>, String> {
        cp.session
            .map(|object| {
                let bytes = read(&self.directory, Domain::Sessions, object)?;
                let state: super::super::survival::State =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                state.validate()?;
                let source: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if source != serde_json::to_value(&state).map_err(|e| e.to_string())? {
                    return Err(
                        "noncanonical or unknown Survival session fields; conversion refused"
                            .into(),
                    );
                }
                Ok(Box::new(state))
            })
            .transpose()
    }
    pub fn collect_garbage(&mut self) -> Result<Collection, String> {
        if self.resident.values().any(|c| c.dirty) {
            return Err("save dirty chunks before collection".into());
        }
        if let Some(state) = &self.survival {
            let bytes = serde_json::to_vec(state).map_err(|e| e.to_string())?;
            let saved = read(
                &self.directory,
                Domain::Sessions,
                self.saved.session.ok_or("missing saved player")?,
            )?;
            if saved != bytes {
                return Err("save player changes before collection".into());
            }
        }
        self.collect_inner()
    }
    pub(super) fn collect_inner(&mut self) -> Result<Collection, String> {
        self.collect_with_hook(|_| Ok(()))
    }

    pub(super) fn collect_with_hook(
        &mut self,
        mut before_remove: impl FnMut(u64) -> Result<(), String>,
    ) -> Result<Collection, String> {
        if self
            .directory
            .join("gc-recovery.hold")
            .try_exists()
            .map_err(|e| e.to_string())?
        {
            return Err("GC disabled: ambiguous recovery objects are pinned".into());
        }
        // No object removal occurs until BOTH manifests and all reachable data
        // pass validation. Even an unreadable backup disables collection.
        self.nodes.clear();
        let primary = self.directory.join("checkpoint.json");
        let mut snapshots = Vec::new();
        let mut live = HashSet::new();
        // Immutable references need one complete validation per collection,
        // not once for primary, again for an identical backup and again for
        // RAM. Keep this cache local: later GC calls MUST re-read the data.
        let mut validated_roots = HashSet::new();
        for p in [&primary, &crate::saveload::backup_path(&primary)] {
            if !p.try_exists().map_err(|e| e.to_string())? {
                if p == &primary {
                    return Err("primary checkpoint missing".into());
                }
                continue;
            }
            real_path(p, false)?;
            let validation: Result<_, String> = (|| {
                let text = crate::saveload::storage::read_text_limited(p, MAX_CHECKPOINT)?;
                let saved = Self::parse_checkpoint(&text)?;
                if saved.seed != self.seed()
                    || saved.generator != self.generator()
                    || saved.format != self.saved.format
                {
                    return Err("checkpoint identity mismatch".into());
                }
                self.read_session(&saved)?;
                if let Some(session) = saved.session {
                    live.insert((Domain::Sessions, session));
                }
                if validated_roots.insert(saved.root) {
                    live.extend(self.validate_root(saved.root)?);
                }
                if self
                    .find(
                        saved.root,
                        Key::at_depth(saved.depth, saved.position.0, saved.position.1),
                    )?
                    .is_none()
                {
                    return Err("checkpoint explorer missing".into());
                }
                Ok(text)
            })();
            match validation {
                Ok(text) => {
                    snapshots.push((p.clone(), text));
                }
                Err(e) => {
                    hold_recovery(&self.directory)?;
                    return Err(format!("GC preflight failed; recovery objects pinned: {e}"));
                }
            }
        }
        // Also retain the live in-memory root, including publication-error state.
        if validated_roots.insert(self.saved.root) {
            live.extend(self.validate_root(self.saved.root)?);
        }
        self.read_session(&self.saved)?;
        if let Some(session) = self.saved.session {
            live.insert((Domain::Sessions, session));
        }
        let mut candidates = Vec::new();
        let mut report = Collection {
            retained_objects: live.len() as u64,
            ..Default::default()
        };
        let domains = if self.is_survival() {
            &[Domain::Nodes, Domain::Chunks, Domain::Sessions][..]
        } else {
            &[Domain::Nodes, Domain::Chunks][..]
        };
        for &domain in domains {
            let directory = self.directory.join(domain.name());
            real_path(&directory, true)?;
            for bucket in fs::read_dir(&directory).map_err(|e| e.to_string())? {
                let bucket = bucket.map_err(|e| e.to_string())?;
                let name = bucket.file_name().to_string_lossy().into_owned();
                if name.len() != 2
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                {
                    report.unknown_files += 1;
                    continue;
                }
                real_path(&bucket.path(), true)?;
                for file in fs::read_dir(bucket.path()).map_err(|e| e.to_string())? {
                    let file = file.map_err(|e| e.to_string())?;
                    let p = file.path();
                    real_path(&p, false)?;
                    let filename = file.file_name().to_string_lossy().into_owned();
                    let parsed = (|| {
                        let (hash, tail) = filename.split_once('-')?;
                        let (slot, suffix) = tail.split_once('.')?;
                        if hash.len() != 16 || slot.len() != 4 || suffix != domain.suffix() {
                            return None;
                        }
                        let o = Object {
                            hash: u64::from_str_radix(hash, 16).ok()?,
                            slot: u16::from_str_radix(slot, 16).ok()?,
                        };
                        if o.slot >= 4096
                            || format!("{:02x}", o.hash >> 56) != name
                            || self::filename(domain, o) != filename
                        {
                            return None;
                        }
                        Some(o)
                    })();
                    match parsed {
                        Some(o) if !live.contains(&(domain, o)) => candidates.push(p),
                        Some(_) => (),
                        None => report.unknown_files += 1,
                    }
                }
            }
        }
        for (p, text) in snapshots {
            if crate::saveload::storage::read_text_limited(&p, MAX_CHECKPOINT)? != text {
                return Err("checkpoint changed during GC; nothing removed".into());
            }
        }
        for p in candidates {
            before_remove(report.removed_files)?;
            let bytes = fs::metadata(&p).map_err(|e| e.to_string())?.len();
            fs::remove_file(&p).map_err(|e| format!("GC stopped safely: {e}"))?;
            report.removed_files += 1;
            report.removed_bytes += bytes;
        }
        self.nodes.clear();
        Ok(report)
    }
}
