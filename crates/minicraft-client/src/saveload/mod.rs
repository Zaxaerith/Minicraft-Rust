//! Versioned save/load, legacy migration, Java import/export, and atomic persistence.
//!
//! Layout mirrors Java `saveload/*`:
//! - `version` ↔ `Version.java`
//! - `load` ↔ `Load.java` (`.miniplussave` → `World`)
//! - `save` ↔ `Save.java` (`World` → `.miniplussave` field lists)
//! - `legacy_load` ↔ `LegacyLoad.java` + `Load.subOldName`
//! - `storage` — atomic write / backup / bounded reads (native `state.json`)

pub mod legacy_load;
pub mod load;
pub mod migration;
pub mod save;
pub mod storage;
pub mod version;

pub use load::{JavaSaveInfo, import_java_save, probe_java_save};
pub use storage::{atomic_write, backup_path, read_candidates, read_candidates_limited};
pub use version::Version;
