//! Read-only Java source -> isolated snapshot -> validated native world ->
//! exclusive directory publication. No in-place Java conversion or ZIP input.
use super::{
    load,
    storage::{atomic_write, read_text_limited},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_FILE: u64 = 16 * 1024 * 1024;
const MAX_TOTAL: u64 = 128 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        // Only the directory exclusively created by this invocation is owned.
        if let Err(e) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "cannot remove import staging directory {}: {e}",
                self.0.display()
            );
        }
    }
}
fn known_file(name: &str) -> bool {
    matches!(
        name,
        "Game.miniplussave"
            | "Player.miniplussave"
            | "Inventory.miniplussave"
            | "Entities.miniplussave"
            | "advancements.json"
            | "signs.json"
    ) || (0..6).any(|i| {
        name == format!("Level{i}.miniplussave") || name == format!("Level{i}data.miniplussave")
    })
}
fn snapshot(source: &Path) -> Result<BTreeMap<String, String>, String> {
    let metadata =
        fs::symlink_metadata(source).map_err(|e| format!("cannot open Java source: {e}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(
            "Java import requires a real world directory; archives and symlinks are not accepted"
                .into(),
        );
    }
    let mut result = BTreeMap::new();
    let mut total = 0_u64;
    for (index, entry) in fs::read_dir(source).map_err(|e| e.to_string())?.enumerate() {
        if index >= 32 {
            return Err("too many Java source files".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 source filename")?;
        if !known_file(&name) {
            return Err(format!(
                "unrecognized Java source file {name:?}; import refused rather than dropping it"
            ));
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if !kind.is_file() || kind.is_symlink() {
            return Err(format!("Java source contains link/subdirectory {name:?}"));
        }
        let canonical = fs::canonicalize(entry.path()).map_err(|e| e.to_string())?;
        if canonical.parent() != Some(source) {
            return Err("Java source path escapes selected world".into());
        }
        let text = read_text_limited(&canonical, MAX_FILE)?;
        total = total
            .checked_add(text.len() as u64)
            .ok_or("input size overflow")?;
        if total > MAX_TOTAL {
            return Err("Java source exceeds total input budget".into());
        }
        result.insert(name, text);
    }
    let game = result
        .get("Game.miniplussave")
        .ok_or("missing Java Game file")?;
    if game.split(',').next() != Some("2.3.0-dev2") {
        return Err("this transactional importer supports exactly Java 2.3.0-dev2".into());
    }
    Ok(result)
}

/// Destination parent must exist and destination itself must not. Optional
/// global Unlocks.json is selected explicitly; never traverse source parents.
pub fn import_to(source: &Path, destination: &Path, unlocks: Option<&Path>) -> Result<(), String> {
    import_with_hook(source, destination, unlocks, || Ok(()))
}
fn import_with_hook(
    source: &Path,
    destination: &Path,
    unlocks: Option<&Path>,
    before_publish: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if destination
        .as_os_str()
        .to_string_lossy()
        .split(['/', '\\'])
        .any(|part| part == "..")
    {
        return Err("parent traversal in destination path".into());
    }
    if fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("symlink Java source refused".into());
    }
    let source = fs::canonicalize(source).map_err(|e| e.to_string())?;
    let name = destination
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("destination world name missing")?;
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("invalid destination world name".into());
    }
    if destination
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("parent traversal in destination path".into());
    }
    let parent = fs::canonicalize(destination.parent().ok_or("destination parent missing")?)
        .map_err(|e| format!("destination parent must already exist: {e}"))?;
    if parent.starts_with(&source) {
        return Err("destination cannot be inside the read-only Java world".into());
    }
    let destination = parent.join(name);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(format!(
            "destination already exists: {}",
            destination.display()
        ));
    }
    let files = snapshot(&source)?;
    let global = unlocks
        .map(|path| {
            let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("Unlocks must be a regular file".into());
            }
            read_text_limited(path, 1024 * 1024)
        })
        .transpose()?;
    let stage = loop {
        let path = parent.join(format!(
            ".java-import-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => break Stage(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    };
    let input = stage.0.join("saves/source");
    fs::create_dir_all(&input).map_err(|e| e.to_string())?;
    for (name, text) in files {
        fs::write(input.join(name), text).map_err(|e| e.to_string())?;
    }
    if let Some(global) = global {
        fs::write(stage.0.join("Unlocks.json"), global).map_err(|e| e.to_string())?;
    }
    let world = load::import_java_save(&input)?;
    let state = world.to_save_string()?;
    // Parse and validate exactly the bytes to be installed, before publication.
    crate::world::World::from_save_string(&state)?;
    let native = stage.0.join("native");
    fs::create_dir(&native).map_err(|e| e.to_string())?;
    atomic_write(&native.join("state.json"), state.as_bytes())?;
    let info = load::probe_java_save(&input)?.ok_or("missing Java metadata")?;
    let metadata = serde_json::json!({"format":1,"game_version":"2.2.4-rust","name":name,"seed":info.seed,"size":info.size,"mode":info.mode.index(),"score_minutes":info.score_minutes,
        "generation_version":"Java230Flora","state":"imported-java","source_version":"2.3.0-dev2"});
    atomic_write(
        &native.join("world.json"),
        serde_json::to_string_pretty(&metadata)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )?;
    before_publish()?;
    publish_exclusive(&native, &destination)
        .map_err(|e| format!("cannot publish imported world without replacing destination: {e}"))?;
    #[cfg(unix)]
    fs::File::open(&parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("world installed, but parent directory sync failed: {e}"))?;
    Ok(())
}

// std::fs::rename on Unix may replace an existing EMPTY directory. Use an
// exclusive OS operation instead; unsupported filesystems fail safely.
#[cfg(windows)]
pub(crate) fn publish_exclusive(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileW(from: *const u16, to: *const u16) -> i32;
    }
    // SAFETY: pointers are live NUL-terminated UTF16 strings for this call.
    // MoveFileW fails if the destination exists, including an empty directory.
    if unsafe { MoveFileW(from.as_ptr(), to.as_ptr()) } != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn publish_exclusive(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let from =
        CString::new(from.as_os_str().as_bytes()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    let to =
        CString::new(to.as_os_str().as_bytes()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    #[cfg(target_os = "linux")]
    unsafe extern "C" {
        fn renameat2(
            fromfd: i32,
            from: *const std::ffi::c_char,
            tofd: i32,
            to: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn renamex_np(
            from: *const std::ffi::c_char,
            to: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    // SAFETY: both pointers reference live NUL-terminated C strings; calls do
    // not retain them. Linux AT_FDCWD=-100, RENAME_NOREPLACE=1; macOS EXCL=4.
    #[cfg(target_os = "linux")]
    let result = unsafe { renameat2(-100, from.as_ptr(), -100, to.as_ptr(), 1) };
    #[cfg(target_os = "macos")]
    let result = unsafe { renamex_np(from.as_ptr(), to.as_ptr(), 4) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn publish_exclusive(_from: &Path, _to: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "exclusive import publication unavailable on this platform",
    ))
}

#[cfg(test)]
pub(crate) mod tests;
