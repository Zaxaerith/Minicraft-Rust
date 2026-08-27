use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::Value;

#[derive(Debug, Clone)]
enum Source {
    Folder(PathBuf),
    Zip(PathBuf),
}

#[derive(Debug, Clone)]
pub struct ResourcePack {
    pub id: String,
    pub name: String,
    source: Source,
}

impl ResourcePack {
    pub fn read(&self, relative_path: &str) -> Result<Option<Vec<u8>>, String> {
        match &self.source {
            Source::Folder(root) => {
                let path = root.join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
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
        packs.push(ResourcePack {
            id: candidate.id,
            name: value["name"].as_str().unwrap_or(&fallback_name).to_owned(),
            source: candidate.source,
        });
    }
    (packs, warnings)
}

fn has_zip_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
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
            r#"{"pack_format":1,"name":"Folder"}"#,
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
        assert_eq!(warnings.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
