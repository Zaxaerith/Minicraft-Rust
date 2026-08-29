use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub fn backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!("{name}.bak"))
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!("{name}.new"))
}

pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let temporary = temporary_path(path);
    let backup = backup_path(path);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("cannot create {}: {error}", temporary.display()))?;
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("cannot flush {}: {error}", temporary.display()))?;
    drop(file);

    if backup.exists() {
        fs::remove_file(&backup)
            .map_err(|error| format!("cannot replace {}: {error}", backup.display()))?;
    }
    let had_primary = path.exists();
    if had_primary {
        fs::rename(path, &backup).map_err(|error| {
            format!(
                "cannot move {} to backup {}: {error}",
                path.display(),
                backup.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if had_primary {
            let _ = fs::rename(&backup, path);
        }
        return Err(format!(
            "cannot install atomic file {} from {}: {error}",
            path.display(),
            temporary.display()
        ));
    }
    Ok(())
}

pub fn read_candidates(path: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    read_candidates_limited(path, 256 * 1024 * 1024)
}

pub fn read_candidates_limited(
    path: &Path,
    maximum_bytes: u64,
) -> Result<Vec<(PathBuf, String)>, String> {
    let mut candidates = Vec::new();
    for candidate in [path.to_owned(), backup_path(path)] {
        match fs::metadata(&candidate) {
            Ok(metadata) if metadata.len() > maximum_bytes => {
                return Err(format!(
                    "{} exceeds the {} byte read limit",
                    candidate.display(),
                    maximum_bytes
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!("cannot inspect {}: {error}", candidate.display()));
            }
        }
        match fs::read_to_string(&candidate) {
            Ok(text) => candidates.push((candidate, text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("cannot read {}: {error}", candidate.display()));
            }
        }
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::{atomic_write, backup_path, read_candidates, read_candidates_limited};

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
