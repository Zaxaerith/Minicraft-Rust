//! Immutable chunk objects + one atomic manifest: old and new manifests always
//! reference complete objects. Interrupted writes leave unreachable objects,
//! never half a visible chunk. No automatic object deletion in this prototype.
use super::*;
use crate::saveload::storage::{atomic_write, read_candidates_limited, restore_primary};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

const FORMAT: &str = "MinicraftRust.Experimental.Infdev4A.v1";
const GENERATOR: &str = "InfdevHeightPrototypeV1";
const MAX_CHUNKS: usize = 8192;
const MAX_MANIFEST: u64 = 2 * 1024 * 1024;
const HEADER: usize = 24;
const OBJECT_SIZE: usize = HEADER + CHUNK_SIZE * CHUNK_SIZE * 3;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    coord: ChunkCoord,
    object: ChunkObject,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    generator: String,
    seed: i64,
    position: (i32, i32),
    chunks: Vec<Reference>,
}

pub(super) fn real_path(path: &Path, directory: bool) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let reparse = metadata.file_type().is_symlink();
    if reparse || (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(format!(
            "not a regular {}: {}",
            if directory { "directory" } else { "file" },
            path.display()
        ));
    }
    Ok(())
}

pub(super) fn lock(directory: &Path) -> Result<File, String> {
    lock_mode(directory, true)
}

fn lock_mode(directory: &Path, create: bool) -> Result<File, String> {
    let path = directory.join("session.lock");
    if path.try_exists().map_err(|e| e.to_string())? {
        real_path(&path, false)?;
    }
    let mut options = fs::OpenOptions::new();
    options
        .create(create)
        .truncate(false)
        .read(true)
        .write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0).custom_flags(0x0020_0000); // OPEN_REPARSE_POINT
    }
    #[cfg(not(windows))]
    return Err("Infdev4A requires Windows file sharing locks".into());
    #[cfg(windows)]
    options.open(&path).map_err(|e| {
        format!(
            "cannot exclusively lock experimental world {}: {e}",
            directory.display()
        )
    })
}

/// Noncryptographic corruption checksum, NOT an authenticity/security signature.
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    })
}

fn object_name(coord: ChunkCoord, object: ChunkObject) -> String {
    format!(
        "c.{}.{}.{:016x}.{}.bin",
        coord.x, coord.y, object.hash, object.slot
    )
}

fn encode(seed: i64, coord: ChunkCoord, cells: &[Cell]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(OBJECT_SIZE);
    bytes.extend_from_slice(b"MCIF4A01");
    bytes.extend_from_slice(&seed.to_le_bytes());
    bytes.extend_from_slice(&coord.x.to_le_bytes());
    bytes.extend_from_slice(&coord.y.to_le_bytes());
    for cell in cells {
        bytes.push(cell.ground as u8);
        bytes.extend_from_slice(&cell.data.to_le_bytes());
    }
    bytes
}

fn read_object(
    directory: &Path,
    seed: i64,
    coord: ChunkCoord,
    object: ChunkObject,
) -> Result<Vec<Cell>, String> {
    let path = directory.join("chunks").join(object_name(coord, object));
    real_path(&path, false)?;
    let mut bytes = Vec::with_capacity(OBJECT_SIZE);
    File::open(&path)
        .map_err(|e| e.to_string())?
        .take((OBJECT_SIZE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let invalid = || {
        format!(
            "corrupt committed chunk ({}, {}): {}",
            coord.x,
            coord.y,
            path.display()
        )
    };
    if bytes.len() != OBJECT_SIZE
        || checksum(&bytes) != object.hash
        || &bytes[..8] != b"MCIF4A01"
        || bytes[8..16] != seed.to_le_bytes()
        || bytes[16..20] != coord.x.to_le_bytes()
        || bytes[20..24] != coord.y.to_le_bytes()
    {
        return Err(invalid());
    }
    bytes[HEADER..]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|b| {
            let ground = match b[0] {
                0 => Ground::Grass,
                1 => Ground::Water,
                2 => Ground::Sand,
                3 => Ground::Rock,
                _ => return Err(invalid()),
            };
            Ok(Cell {
                ground,
                data: u16::from_le_bytes([b[1], b[2]]),
            })
        })
        .collect()
}

impl InfdevWorld {
    /// Creates ROOT/infdev4a-v1/NAME exclusively. Never accepts/relabels a fixed
    /// world or replaces an existing directory, even if its manifest is corrupt.
    pub fn create(root: &Path, name: &str, seed: i64, capacity: usize) -> Result<Self, String> {
        if !(1..=64).contains(&capacity) {
            return Err("resident capacity must be 1..64".into());
        }
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
        let directory = namespace.join(name);
        fs::create_dir(&directory).map_err(|e| {
            format!("cannot create new experimental world (never overwritten): {e}")
        })?;
        fs::create_dir(directory.join("chunks")).map_err(|e| e.to_string())?;
        let _lock = lock(&directory)?;
        let mut world = Self {
            directory,
            _lock,
            seed,
            position: (0, 0),
            committed: BTreeMap::new(),
            resident: BTreeMap::new(),
            capacity,
            clock: 0,
            recovered: false,
        };
        world.ensure(ChunkCoord::at(0, 0))?;
        world.checkpoint()?;
        Ok(world)
    }

    pub fn open(directory: &Path, capacity: usize) -> Result<Self, String> {
        Self::open_mode(directory, capacity, true)
    }

    pub(super) fn open_snapshot(directory: &Path) -> Result<Self, String> {
        Self::open_mode(directory, 1, false)
    }

    fn open_mode(directory: &Path, capacity: usize, restore: bool) -> Result<Self, String> {
        if !(1..=64).contains(&capacity) {
            return Err("resident capacity must be 1..64".into());
        }
        real_path(directory, true)?;
        // Refuse the fixed-world namespace before even creating the lock file.
        if directory
            .join("state.json")
            .try_exists()
            .map_err(|e| e.to_string())?
            || directory
                .join("world.json")
                .try_exists()
                .map_err(|e| e.to_string())?
        {
            return Err("fixed-world saves cannot be opened as Infdev4A".into());
        }
        real_path(&directory.join("chunks"), true)?;
        // A copy upgrade must never create a file in its read-only source.
        let _lock = lock_mode(directory, restore)?;
        let path = directory.join("manifest.json");
        for candidate in [&path, &crate::saveload::backup_path(&path)] {
            if fs::symlink_metadata(candidate).is_ok() {
                real_path(candidate, false)?;
            }
        }
        let mut errors = Vec::new();
        for (candidate, text) in read_candidates_limited(&path, MAX_MANIFEST)? {
            let checked: Result<_, String> = (|| {
                let saved: Manifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                if saved.format != FORMAT || saved.generator != GENERATOR {
                    return Err(
                        "unknown experimental format/generator; no regeneration performed".into(),
                    );
                }
                if saved.chunks.len() > MAX_CHUNKS {
                    return Err("experimental chunk index limit exceeded".into());
                }
                let mut committed = BTreeMap::new();
                for reference in saved.chunks {
                    if !reference.coord.valid()
                        || reference.object.slot >= 1024
                        || committed
                            .insert(reference.coord, reference.object)
                            .is_some()
                    {
                        return Err("invalid or duplicate committed chunk coordinate".into());
                    }
                    // Validate the whole checkpoint, not just the player's chunk.
                    read_object(directory, saved.seed, reference.coord, reference.object)?;
                }
                if !committed.contains_key(&ChunkCoord::at(saved.position.0, saved.position.1)) {
                    return Err("saved explorer has no committed destination chunk".into());
                }
                Ok((saved.seed, saved.position, committed))
            })();
            match checked {
                Ok((seed, position, committed)) => {
                    let recovered = candidate != path;
                    // Restore only an entirely validated checkpoint. Never rotate
                    // a corrupt primary over the validated backup.
                    if recovered && restore {
                        restore_primary(&path, text.as_bytes())?;
                    }
                    return Ok(Self {
                        directory: directory.to_path_buf(),
                        _lock,
                        seed,
                        position,
                        committed,
                        resident: BTreeMap::new(),
                        capacity,
                        clock: 0,
                        recovered,
                    });
                }
                Err(error) => errors.push(format!("{}: {error}", candidate.display())),
            }
        }
        Err(format!(
            "no complete valid Infdev4A checkpoint; {}",
            errors.join("; ")
        ))
    }

    pub(super) fn read_chunk(
        &self,
        coord: ChunkCoord,
        object: ChunkObject,
    ) -> Result<Vec<Cell>, String> {
        read_object(&self.directory, self.seed, coord, object)
    }

    pub fn checkpoint(&mut self) -> Result<(), String> {
        self.checkpoint_with_hook(|| Ok(()))
    }

    /// All new objects are fsynced BEFORE the single manifest commit. Failure
    /// before that point preserves the previous complete checkpoint and dirty RAM.
    pub(super) fn checkpoint_with_hook(
        &mut self,
        before_commit: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let mut next = self.committed.clone();
        for (&coord, chunk) in &self.resident {
            if !chunk.dirty {
                continue;
            }
            let bytes = encode(self.seed, coord, &chunk.cells);
            let hash = checksum(&bytes);
            let mut written = None;
            for slot in 0..1024 {
                let object = ChunkObject { hash, slot };
                let path = self
                    .directory
                    .join("chunks")
                    .join(object_name(coord, object));
                match fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)
                {
                    Ok(mut file) => {
                        file.write_all(&bytes)
                            .and_then(|_| file.sync_all())
                            .map_err(|e| {
                                format!("chunk write failed; checkpoint unchanged: {e}")
                            })?;
                        written = Some(object);
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        if let Ok(existing) = read_object(&self.directory, self.seed, coord, object)
                        {
                            if existing != chunk.cells {
                                return Err("chunk checksum collision; refusing replacement".into());
                            }
                            // Reuse can follow a failed sync from an earlier
                            // attempt; require durable bytes before publication.
                            fs::OpenOptions::new()
                                .read(true)
                                .write(true)
                                .open(&path)
                                .and_then(|f| f.sync_all())
                                .map_err(|e| e.to_string())?;
                            written = Some(object);
                            break;
                        }
                        // A torn, unreachable object from a crash is preserved.
                        // Use another immutable slot; never replace it in place.
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            next.insert(coord, written.ok_or("too many conflicting chunk objects")?);
        }
        if next.len() > MAX_CHUNKS {
            return Err("experimental chunk index limit exceeded; checkpoint unchanged".into());
        }
        let saved = Manifest {
            format: FORMAT.into(),
            generator: GENERATOR.into(),
            seed: self.seed,
            position: self.position,
            chunks: next
                .iter()
                .map(|(&coord, &object)| Reference { coord, object })
                .collect(),
        };
        let text = serde_json::to_vec(&saved).map_err(|e| e.to_string())?;
        if text.len() as u64 > MAX_MANIFEST {
            return Err("experimental manifest too large".into());
        }
        before_commit()?;
        atomic_write(&self.directory.join("manifest.json"), &text)?;
        self.committed = next;
        for chunk in self.resident.values_mut() {
            chunk.dirty = false;
        }
        Ok(())
    }
}
