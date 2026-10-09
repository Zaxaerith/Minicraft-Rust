use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!("{name}.bak"))
}

// Each writer owns its staging file; never truncate another writer's file or
// follow a pre-existing `.new` symlink. Drop removes only our own staging file.
struct StagedFile(PathBuf);

impl StagedFile {
    fn create(path: &Path) -> Result<(Self, std::fs::File), String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let name = path.file_name().ok_or("atomic file has no name")?;
        loop {
            let mut temporary = name.to_os_string();
            temporary.push(format!(
                ".{}.{}.new",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let temporary = path.with_file_name(temporary);
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
            {
                Ok(file) => return Ok((Self(temporary), file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("cannot create {}: {error}", temporary.display()));
                }
            }
        }
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn sync_directory(path: &Path) -> Result<(), String> {
    // Windows does not support opening directories through std::fs::File.
    #[cfg(unix)]
    std::fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("cannot sync {}: {error}", path.display()))?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    write_atomic(path, contents, true)
}

/// Restore a validated backup without replacing it with the corrupt primary.
pub fn restore_primary(path: &Path, contents: &[u8]) -> Result<(), String> {
    write_atomic(path, contents, false)
}

fn write_atomic(path: &Path, contents: &[u8], rotate_backup: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let backup = backup_path(path);
    let (temporary, mut file) = StagedFile::create(path)?;
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("cannot flush {}: {error}", temporary.0.display()))?;
    drop(file);
    let previous = if rotate_backup {
        match std::fs::File::open(path) {
            Ok(mut source) => {
                let (staged, mut destination) = StagedFile::create(&backup)?;
                std::io::copy(&mut source, &mut destination)
                    .and_then(|_| destination.sync_all())
                    .map_err(|error| {
                        format!("cannot stage backup {}: {error}", backup.display())
                    })?;
                Some(staged)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("cannot read previous {}: {error}", path.display())),
        }
    } else {
        None
    };
    // Replace the primary while the original primary AND backup still exist.
    // A recovered backup must survive even if the current primary is corrupt
    // and installation fails. Never delete the last backup before installation.
    fs::rename(&temporary.0, path)
        .map_err(|error| format!("cannot install atomic file {}: {error}", path.display()))?;
    sync_directory(parent)?;
    if let Some(previous) = previous {
        fs::rename(&previous.0, &backup).map_err(|error| {
            format!(
                "primary saved, but cannot replace backup {}: {error}",
                backup.display()
            )
        })?;
        sync_directory(parent)?;
    }
    Ok(())
}

/// Bound the actual read, including files that grow after the metadata check.
pub fn read_text_limited(path: &Path, maximum_bytes: u64) -> Result<String, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let limit_error = || {
        format!(
            "{} exceeds the {} byte read limit",
            path.display(),
            maximum_bytes
        )
    };
    if metadata.len() > maximum_bytes {
        return Err(limit_error());
    }
    let mut bytes = Vec::new();
    file.take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() as u64 > maximum_bytes {
        return Err(limit_error());
    }
    String::from_utf8(bytes).map_err(|error| format!("cannot decode {}: {error}", path.display()))
}

pub fn read_candidates(path: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    read_candidates_limited(path, 256 * 1024 * 1024)
}

pub fn read_candidates_limited(
    path: &Path,
    maximum_bytes: u64,
) -> Result<Vec<(PathBuf, String)>, String> {
    let mut candidates = Vec::new();
    let mut errors = Vec::new();
    for candidate in [path.to_owned(), backup_path(path)] {
        match fs::metadata(&candidate) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                errors.push(format!("cannot inspect {}: {error}", candidate.display()));
                continue;
            }
        }
        match read_text_limited(&candidate, maximum_bytes) {
            Ok(text) => candidates.push((candidate, text)),
            Err(error) => errors.push(error),
        }
    }
    if candidates.is_empty() && !errors.is_empty() {
        Err(errors.join("; "))
    } else {
        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::{atomic_write, backup_path, read_candidates, read_candidates_limited};

    fn test_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "minicraft-storage-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn unreadable_or_oversized_primary_does_not_hide_backup() {
        let root = test_root();
        let path = root.join("state.json");
        let backup = backup_path(&path);
        std::fs::write(&backup, "good backup").unwrap();
        for bytes in [vec![0xff], vec![b'x'; 33]] {
            std::fs::write(&path, bytes).unwrap();
            assert_eq!(
                read_candidates_limited(&path, 32).unwrap(),
                vec![(backup.clone(), "good backup".to_owned())]
            );
        }
        std::fs::remove_file(&path).unwrap();
        assert_eq!(read_candidates_limited(&path, 32).unwrap().len(), 1);
        std::fs::write(&backup, [0xff]).unwrap();
        assert!(read_candidates_limited(&path, 32).is_err());
        std::fs::remove_file(&backup).unwrap();
        assert!(read_candidates_limited(&path, 32).unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_primary_preserves_backup_and_stale_staging_file() {
        let root = test_root();
        let path = root.join("state.json");
        std::fs::write(backup_path(&path), b"recovery").unwrap();
        std::fs::write(root.join("state.json.new"), b"interrupted").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(std::fs::read(backup_path(&path)).unwrap(), b"recovery");
        assert_eq!(
            std::fs::read(root.join("state.json.new")).unwrap(),
            b"interrupted"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_backup_rotation_keeps_installed_primary_and_old_backup() {
        let root = test_root();
        let path = root.join("state.json");
        std::fs::write(&path, b"previous").unwrap();
        std::fs::create_dir(backup_path(&path)).unwrap();
        std::fs::write(backup_path(&path).join("protected"), b"keep").unwrap();
        let error = atomic_write(&path, b"new").unwrap_err();
        assert!(error.contains("primary saved"));
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(
            std::fs::read(backup_path(&path).join("protected")).unwrap(),
            b"keep"
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn sharing_denial_preserves_backup_on_read_and_write_failure() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = test_root();
        let path = root.join("state.json");
        std::fs::write(&path, b"primary").unwrap();
        std::fs::write(backup_path(&path), b"backup").unwrap();
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert_eq!(read_candidates_limited(&path, 32).unwrap()[0].1, "backup");
        assert!(atomic_write(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(backup_path(&path)).unwrap(), b"backup");
        drop(locked);
        assert_eq!(std::fs::read(&path).unwrap(), b"primary");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atomic_write_keeps_the_previous_file_as_backup() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-rust-atomic-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("state.json");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert_eq!(
            std::fs::read_to_string(backup_path(&path)).unwrap(),
            "first"
        );
        assert_eq!(read_candidates(&path).unwrap().len(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bounded_reads_reject_sparse_oversized_files() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-rust-bounded-read-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.json");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(1_025)
            .unwrap();
        let error = read_candidates_limited(&path, 1_024).unwrap_err();
        assert!(error.contains("read limit"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
