pub(crate) mod display;

use crate::gfx::Image;

use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::Value;

const MAX_PACK_ENTRY_BYTES: u64 = 16 * 1024 * 1024;

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
    pub(crate) fn parity_fixture(id: &str, name: &str, logo: Option<Image>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: "No description".into(),
            logo,
            source: Source::Fixture,
        }
    }

    pub fn read(&self, relative_path: &str) -> Result<Option<Vec<u8>>, String> {
        match &self.source {
            Source::Fixture => Ok(None),
            Source::Folder(root) => {
                let path = root.join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
                match fs::metadata(&path) {
                    Ok(metadata) if metadata.len() > MAX_PACK_ENTRY_BYTES => {
                        return Err(format!(
                            "resource-pack entry {} exceeds 16 MiB",
                            path.display()
                        ));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(error) => {
                        return Err(format!("cannot inspect {}: {error}", path.display()));
                    }
                }
                match fs::read(&path) {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(error) => Err(format!("cannot read {}: {error}", path.display())),
                }
            }
            Source::Zip(path) => {
                let file = File::open(path)
                    .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
                let mut archive = zip::ZipArchive::new(file)
                    .map_err(|error| format!("invalid zip {}: {error}", path.display()))?;
                match archive.by_name(relative_path) {
                    Ok(mut entry) => {
                        if entry.size() > MAX_PACK_ENTRY_BYTES {
                            return Err(format!(
                                "resource-pack entry {relative_path} in {} exceeds 16 MiB",
                                path.display()
                            ));
                        }
                        let mut bytes = Vec::new();
                        entry.read_to_end(&mut bytes).map_err(|error| {
                            format!("cannot read {relative_path} in {}: {error}", path.display())
                        })?;
                        Ok(Some(bytes))
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
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
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
        io::Write,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn logo_dimensions_follow_java_limits_before_pixel_decode() {
        let logo = super::decode_logo(include_bytes!("../assets/pack.png")).unwrap();
        assert!(logo.width <= 288 && logo.height <= 32);
        assert!(
            super::decode_logo(include_bytes!("../assets/assets/textures/gui/font.png")).is_err()
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
