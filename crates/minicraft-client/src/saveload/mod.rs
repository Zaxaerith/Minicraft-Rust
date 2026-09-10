//! Versioned save/load, legacy migration, Java import, and atomic persistence.
pub mod storage;

pub use storage::{atomic_write, backup_path, read_candidates, read_candidates_limited};
