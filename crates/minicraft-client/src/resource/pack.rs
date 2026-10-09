//! Folder/ZIP resource packs shared by the client and root compatibility host.
use crate::gfx::Image;

use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use serde_json::Value;

const MAX_PACK_ENTRY_BYTES: u64 = 16 * 1024 * 1024;

// Check the directory counts before ZipArchive allocates its entry table.
// These bounds cover ordinary packs and small ZIP64 archives alike.
fn check_zip_directory(file: &mut File) -> Result<(), String> {
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    let length = metadata.len();
    if !metadata.is_file() || !(22..=64 * 1024 * 1024).contains(&length) {
        return Err("resource-pack ZIP must be a regular file of at most 64 MiB".to_owned());
    }
    let tail_length = length.min(65_557) as usize;
    file.seek(SeekFrom::End(-(tail_length as i64)))
        .map_err(|error| error.to_string())?;
    let mut tail = vec![0; tail_length];
    file.read_exact(&mut tail)
        .map_err(|error| error.to_string())?;
    let end = (0..=tail_length - 22)
        .rev()
        .find(|&offset| {
            tail[offset..offset + 4] == *b"PK\x05\x06"
                && offset
                    + 22
                    + usize::from(u16::from_le_bytes([tail[offset + 20], tail[offset + 21]]))
                    == tail_length
        })
        .ok_or("invalid resource-pack ZIP footer")?;
    let mut entries = u64::from(u16::from_le_bytes([tail[end + 10], tail[end + 11]])).max(
        u64::from(u16::from_le_bytes([tail[end + 8], tail[end + 9]])),
    );
    let mut directory_bytes = u64::from(u32::from_le_bytes(
        tail[end + 12..end + 16]
            .try_into()
            .map_err(|_| "invalid ZIP directory size")?,
    ));
    let absolute_end = length - tail_length as u64 + end as u64;
    let mut locator = [0; 20];
    if let Some(offset) = absolute_end.checked_sub(20) {
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        file.read_exact(&mut locator)
            .map_err(|error| error.to_string())?;
    }
    let requires_zip64 = [4, 6, 8, 10]
        .into_iter()
        .any(|offset| u16::from_le_bytes([tail[end + offset], tail[end + offset + 1]]) == u16::MAX)
        || [12, 16]
            .into_iter()
            .any(|offset| tail[end + offset..end + offset + 4] == u32::MAX.to_le_bytes());
    if requires_zip64 || locator[..4] == *b"PK\x06\x07" {
        if locator[..4] != *b"PK\x06\x07" {
            return Err("invalid resource-pack ZIP64 locator".to_owned());
        }
        let record_offset = u64::from_le_bytes(
            locator[8..16]
                .try_into()
                .map_err(|_| "invalid ZIP64 offset")?,
        );
        if record_offset > length.saturating_sub(56) {
            return Err("invalid resource-pack ZIP64 directory offset".to_owned());
        }
        file.seek(SeekFrom::Start(record_offset))
            .map_err(|error| error.to_string())?;
        let mut record = [0; 56];
        file.read_exact(&mut record)
            .map_err(|error| error.to_string())?;
        if record[..4] != *b"PK\x06\x06" {
            return Err("invalid resource-pack ZIP64 directory".to_owned());
        }
        entries = u64::from_le_bytes(
            record[32..40]
                .try_into()
                .map_err(|_| "invalid ZIP64 count")?,
        );
        entries = entries.max(u64::from_le_bytes(
            record[24..32]
                .try_into()
                .map_err(|_| "invalid ZIP64 disk count")?,
        ));
        directory_bytes = u64::from_le_bytes(
            record[40..48]
                .try_into()
                .map_err(|_| "invalid ZIP64 size")?,
        );
    }
    if entries > 4096 || directory_bytes > 4 * 1024 * 1024 {
        return Err("resource-pack ZIP directory exceeds 4096 entries or 4 MiB".to_owned());
    }
    file.rewind().map_err(|error| error.to_string())
}

/// Use the same portable relative-path grammar for ZIP and folder packs.
/// Backslashes, drive/ADS syntax, empty components and dot components are
/// rejected even on Unix so a pack cannot acquire a different meaning on Windows.
pub(crate) fn validate_resource_path(path: &str) -> Result<(), String> {
    if path.contains(['<', '>', '"', '|', '?', '*'])
        || path.split('/').any(|part| {
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .trim_end()
                .to_ascii_uppercase();
            matches!(
                stem.as_str(),
                "CON"
                    | "PRN"
                    | "AUX"
                    | "NUL"
                    | "COM1"
                    | "COM2"
                    | "COM3"
                    | "COM4"
                    | "COM5"
                    | "COM6"
                    | "COM7"
                    | "COM8"
                    | "COM9"
                    | "LPT1"
                    | "LPT2"
                    | "LPT3"
                    | "LPT4"
                    | "LPT5"
                    | "LPT6"
                    | "LPT7"
                    | "LPT8"
                    | "LPT9"
            )
        })
    {
        return Err(format!("invalid resource-pack path: {path:?}"));
    }
    if path.is_empty()
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
        || path.split('/').any(|part| {
            part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' '])
        })
        || std::path::Path::new(path)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(format!("invalid resource-pack path: {path:?}"));
    }
    Ok(())
}

pub(crate) fn read_entry(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_PACK_ENTRY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read resource-pack entry: {error}"))?;
    if bytes.len() as u64 > MAX_PACK_ENTRY_BYTES {
        return Err("resource-pack entry exceeds 16 MiB".to_owned());
    }
    Ok(bytes)
}

#[derive(Debug, Clone)]
enum Source {
    Fixture,
    Folder(PathBuf),
    Zip(PathBuf),
}

#[derive(Debug, Clone)]
pub struct ResourcePack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub logo: Option<Image>,
    source: Source,
}

impl ResourcePack {
    pub fn parity_fixture(id: &str, name: &str, logo: Option<Image>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: "No description".into(),
            logo,
            source: Source::Fixture,
        }
    }

    pub fn read(&self, relative_path: &str) -> Result<Option<Vec<u8>>, String> {
        validate_resource_path(relative_path)?;
        match &self.source {
            Source::Fixture => Ok(None),
            Source::Folder(root) => {
                let root = fs::canonicalize(root)
                    .map_err(|error| format!("cannot resolve pack root: {error}"))?;
                let path = root.join(relative_path);
                let path = match fs::canonicalize(&path) {
                    Ok(path) if path.starts_with(&root) => path,
                    Ok(_) => {
                        return Err(format!("resource-pack path escapes root: {relative_path}"));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(error) => {
                        return Err(format!("cannot resolve {}: {error}", path.display()));
                    }
                };
                // Read metadata and bytes through the same opened handle.
                let file = File::open(&path)
                    .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
                match file.metadata() {
                    Ok(metadata) if metadata.len() > MAX_PACK_ENTRY_BYTES => {
                        return Err(format!(
                            "resource-pack entry {} exceeds 16 MiB",
                            path.display()
                        ));
                    }
                    Ok(metadata) if !metadata.is_file() => {
                        return Err(format!("{} is not a regular file", path.display()));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(error) => {
                        return Err(format!("cannot inspect {}: {error}", path.display()));
                    }
                }
                read_entry(file).map(Some)
            }
            Source::Zip(path) => {
                let mut file = File::open(path)
                    .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
                check_zip_directory(&mut file)?;
                let mut archive = zip::ZipArchive::new(file)
                    .map_err(|error| format!("invalid zip {}: {error}", path.display()))?;
                match archive.by_name(relative_path) {
                    Ok(entry) => {
                        if entry.is_dir()
                            || entry
                                .unix_mode()
                                .is_some_and(|mode| mode & 0o170000 == 0o120000)
                        {
                            return Err(format!(
                                "resource-pack entry {relative_path} is not a regular file"
                            ));
                        }
                        if entry.size() > MAX_PACK_ENTRY_BYTES {
                            return Err(format!(
                                "resource-pack entry {relative_path} in {} exceeds 16 MiB",
                                path.display()
                            ));
                        }
                        read_entry(entry).map(Some)
                    }
                    Err(zip::result::ZipError::FileNotFound) => Ok(None),
                    Err(error) => Err(format!(
                        "cannot inspect {relative_path} in {}: {error}",
                        path.display()
                    )),
                }
            }
        }
    }
}

pub fn discover(game_dir: &Path) -> (Vec<ResourcePack>, Vec<String>) {
    let directory = game_dir.join("resourcepacks");
    let mut warnings = Vec::new();
    let mut paths = match fs::read_dir(&directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() || has_zip_extension(path))
            .collect::<Vec<_>>(),
        Err(error) => {
            warnings.push(format!("cannot list {}: {error}", directory.display()));
            return (Vec::new(), warnings);
        }
    };
    paths.sort_by_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    });

    let mut packs = Vec::new();
    for path in paths {
        let source = if path.is_dir() {
            Source::Folder(path.clone())
        } else {
            Source::Zip(path.clone())
        };
        let fallback_name = path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unnamed Pack".to_owned());
        let candidate = ResourcePack {
            id: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| fallback_name.clone()),
            name: fallback_name.clone(),
            description: "No description".to_owned(),
            logo: None,
            source,
        };
        let metadata = match candidate.read("pack.json") {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                warnings.push(format!("{} has no pack.json; skipped", path.display()));
                continue;
            }
            Err(error) => {
                warnings.push(error);
                continue;
            }
        };
        let value: Value = match serde_json::from_slice(&metadata) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("invalid pack.json in {}: {error}", path.display()));
                continue;
            }
        };
        if value["pack_format"].as_u64() != Some(1) {
            warnings.push(format!(
                "{} uses unsupported pack_format; skipped",
                path.display()
            ));
            continue;
        }
        let logo = match candidate
            .read("pack.png")
            .and_then(|bytes| bytes.map(|bytes| decode_logo(&bytes)).transpose())
        {
            Ok(logo) => logo,
            Err(error) => {
                warnings.push(format!(
                    "{}: invalid pack logo; using default: {error}",
                    path.display()
                ));
                None
            }
        };
        packs.push(ResourcePack {
            id: candidate.id,
            name: value["name"].as_str().unwrap_or(&fallback_name).to_owned(),
            description: value["description"]
                .as_str()
                .unwrap_or("No description")
                .to_owned(),
            logo,
            source: candidate.source,
        });
    }
    (packs, warnings)
}

fn has_zip_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn decode_logo(bytes: &[u8]) -> Result<Image, String> {
    // Reject dimensions before allocating decoded pixels (Java accepts <=288x32, 8px aligned).
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_limits(png::Limits {
        bytes: 32 * 1024 * 1024,
    });
    let reader = decoder.read_info().map_err(|error| error.to_string())?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > 288
        || info.height > 32
        || !info.width.is_multiple_of(8)
        || !info.height.is_multiple_of(8)
    {
        return Err(format!(
            "unacceptable logo size: {}x{}",
            info.width, info.height
        ));
    }
    Image::from_png(bytes)
}

#[cfg(test)]
mod tests {
    use super::{discover, has_zip_extension};
    use std::{
        fs::{self, File},
        io::{Read, Write},
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn zip_metadata_is_bounded_before_entry_table_allocation() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-zip-bounds-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("pack.zip");
        let mut file = File::create(&path).unwrap();
        file.set_len(64 * 1024 * 1024 + 1).unwrap();
        assert!(super::check_zip_directory(&mut file).is_err());
        drop(file);
        let mut footer = [0_u8; 22];
        footer[..4].copy_from_slice(b"PK\x05\x06");
        footer[10..12].copy_from_slice(&4097_u16.to_le_bytes());
        fs::write(&path, footer).unwrap();
        assert!(
            super::check_zip_directory(&mut File::open(&path).unwrap())
                .unwrap_err()
                .contains("4096")
        );
        let mut zip64 = vec![0_u8; 56 + 20 + 22];
        zip64[..4].copy_from_slice(b"PK\x06\x06");
        zip64[4..12].copy_from_slice(&44_u64.to_le_bytes());
        zip64[32..40].copy_from_slice(&u64::MAX.to_le_bytes());
        zip64[56..60].copy_from_slice(b"PK\x06\x07");
        footer[10..12].copy_from_slice(&u16::MAX.to_le_bytes());
        zip64[76..].copy_from_slice(&footer);
        fs::write(&path, &zip64).unwrap();
        assert!(
            super::check_zip_directory(&mut File::open(&path).unwrap())
                .unwrap_err()
                .contains("4096")
        );
        zip64[32..40].copy_from_slice(&1_u64.to_le_bytes());
        fs::write(&path, &zip64).unwrap();
        super::check_zip_directory(&mut File::open(&path).unwrap()).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_and_zip_share_portable_path_rejection() {
        for source in [
            super::Source::Folder("unused".into()),
            super::Source::Zip("unused.zip".into()),
        ] {
            let mut pack = super::ResourcePack::parity_fixture("bad", "bad", None);
            pack.source = source;
            for path in [
                "",
                "/etc/passwd",
                "../escape",
                "a/../../escape",
                "C:/escape",
                "a\\..\\escape",
                "//server/share",
                "a//b",
                "a/./b",
                "a/b:stream",
                "a/b.",
                "a/b ",
                "a/\0b",
                "assets/NUL.png",
                "assets/COM1.png",
                "assets/a?.png",
                "assets/a|b.png",
            ] {
                assert!(
                    pack.read(path)
                        .unwrap_err()
                        .contains("invalid resource-pack path"),
                    "{path:?}"
                );
            }
        }
        assert!(super::validate_resource_path("assets/textures/tile/water.png").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn folder_symlink_cannot_escape_pack_root() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-pack-link-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("pack")).unwrap();
        fs::write(root.join("outside.png"), b"private").unwrap();
        std::os::unix::fs::symlink(root.join("outside.png"), root.join("pack/escape.png")).unwrap();
        let mut pack = super::ResourcePack::parity_fixture("link", "link", None);
        pack.source = super::Source::Folder(root.join("pack"));
        assert!(
            pack.read("escape.png")
                .unwrap_err()
                .contains("escapes root")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn actual_entry_read_is_bounded_without_metadata() {
        assert!(
            super::read_entry(std::io::repeat(b'x').take(super::MAX_PACK_ENTRY_BYTES + 1)).is_err()
        );
    }

    #[test]
    fn logo_dimensions_follow_java_limits_before_pixel_decode() {
        let logo = super::decode_logo(include_bytes!("../../../../assets/pack.png")).unwrap();
        assert!(logo.width <= 288 && logo.height <= 32);
        assert!(
            super::decode_logo(include_bytes!(
                "../../../../assets/assets/textures/gui/font.png"
            ))
            .is_err()
        );
        assert!(super::decode_logo(b"not png").is_err());
    }

    #[test]
    fn zip_extension_is_case_insensitive() {
        assert!(has_zip_extension(Path::new("pack.ZIP")));
        assert!(!has_zip_extension(Path::new("pack.jar")));
    }

    #[test]
    fn malformed_pack_is_skipped_while_folder_and_zip_load() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "minicraft-rust-pack-test-{}-{suffix}",
            std::process::id()
        ));
        let packs = root.join("resourcepacks");
        let folder = packs.join("folder-pack");
        let broken = packs.join("broken-pack");
        fs::create_dir_all(&folder).unwrap();
        fs::create_dir_all(&broken).unwrap();
        fs::write(
            folder.join("pack.json"),
            r#"{"pack_format":1,"name":"Folder","description":"Folder description"}"#,
        )
        .unwrap();
        fs::write(broken.join("pack.json"), "not json").unwrap();

        let zip_path = packs.join("zip-pack.zip");
        let mut archive = zip::ZipWriter::new(File::create(&zip_path).unwrap());
        archive
            .start_file("pack.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive
            .write_all(br#"{"pack_format":1,"name":"Zip"}"#)
            .unwrap();
        archive.finish().unwrap();

        let (loaded, warnings) = discover(&root);
        let names = loaded
            .iter()
            .map(|pack| pack.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["Folder", "Zip"]);
        assert_eq!(loaded[0].description, "Folder description");
        assert_eq!(loaded[1].description, "No description");
        assert!(loaded.iter().all(|pack| pack.logo.is_none()));
        assert_eq!(warnings.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_resource_pack_metadata_is_rejected_before_allocation() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-rust-large-pack-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pack = root.join("resourcepacks").join("oversized");
        fs::create_dir_all(&pack).unwrap();
        File::create(pack.join("pack.json"))
            .unwrap()
            .set_len(super::MAX_PACK_ENTRY_BYTES + 1)
            .unwrap();
        let (loaded, warnings) = discover(&root);
        assert!(loaded.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("exceeds 16 MiB"));
        fs::remove_dir_all(root).unwrap();
    }
}
