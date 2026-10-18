use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct TestRoot(PathBuf);
impl TestRoot {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "minicraft-infdev4a-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn world(&self, capacity: usize) -> InfdevWorld {
        InfdevWorld::create(&self.0, "Test", 42, capacity).unwrap()
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn marker(data: u16) -> Cell {
    Cell {
        ground: Ground::Rock,
        data,
    }
}

#[test]
fn coordinates_follow_java_floor_div_mod_and_extreme_steps_cannot_overflow() {
    for x in [
        i32::MIN,
        -129,
        -128,
        -65,
        -64,
        -63,
        -1,
        0,
        1,
        63,
        64,
        65,
        i32::MAX,
    ] {
        for y in [-65, -1, 0, 64, i32::MAX] {
            let c = ChunkCoord::at(x, y);
            let index = ChunkCoord::local_index(x, y);
            assert!(c.valid());
            assert!(index < 4096);
            assert_eq!(i64::from(c.x) * 64 + (index % 64) as i64, i64::from(x));
            assert_eq!(i64::from(c.y) * 64 + (index / 64) as i64, i64::from(y));
        }
    }
    let root = TestRoot::new();
    let mut w = root.world(1);
    assert!(w.step(i32::MIN, i32::MIN).is_err());
    w.position = (i32::MAX, 0);
    assert!(w.step(1, 0).is_err());
    assert_eq!(w.position(), (i32::MAX, 0));
}

#[test]
fn java_infdev_original_level_noise_and_negative_coordinates_match_oracle() {
    assert_eq!(
        include_str!("../../../tests/fixtures/infdev4a/noise-coordinates.tsv")
            .lines()
            .filter(|s| !s.starts_with('#') && !s.is_empty())
            .count(),
        50
    );
    for row in include_str!("../../../tests/fixtures/infdev4a/noise-coordinates.tsv")
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
    {
        let v: Vec<_> = row.split('\t').collect();
        let seed = v[0].parse().unwrap();
        let x = v[1].parse().unwrap();
        let y = v[2].parse().unwrap();
        assert_eq!(
            ChunkCoord::at(x, y),
            ChunkCoord {
                x: v[3].parse().unwrap(),
                y: v[4].parse().unwrap()
            }
        );
        assert_eq!(
            ChunkCoord::local_index(x, y),
            v[5].parse::<usize>().unwrap()
        );
        assert_eq!(
            height(seed, x, y).to_bits(),
            u64::from_str_radix(v[6], 16).unwrap(),
            "seed={seed} x={x} y={y}"
        );
    }
}

#[test]
fn generation_is_order_independent_and_chunk_edges_use_global_noise_coordinates() {
    let a = ChunkCoord { x: -1, y: 0 };
    let b = ChunkCoord { x: 0, y: -1 };
    let first = generation::generate(-1, a);
    let _ = generation::generate(42, b);
    assert_eq!(first, generation::generate(-1, a));
    assert_ne!(height(-1, -64, 0).to_bits(), height(42, -64, 0).to_bits());
    assert_eq!(first.len(), 4096);
    for (x, y) in [(-64, 0), (-1, 0), (-1, 63)] {
        let h = height(-1, x, y);
        let expected = if h < -0.25 {
            Ground::Water
        } else if h < -0.18 {
            Ground::Sand
        } else if h > 0.3 {
            Ground::Rock
        } else {
            Ground::Grass
        };
        assert_eq!(first[ChunkCoord::local_index(x, y)].ground, expected);
    }
}

#[test]
fn real_movement_eviction_edit_save_unload_and_reopen_preserve_chunks_and_player() {
    let root = TestRoot::new();
    let mut w = root.world(2);
    let directory = w.directory().to_path_buf();
    for _ in 0..65 {
        w.step(-1, 0).unwrap();
    }
    w.step(0, -1).unwrap();
    assert_eq!(w.position(), (-65, -1));
    w.set_cell(-65, -1, marker(731)).unwrap();
    w.set_cell(64, 64, marker(23)).unwrap();
    // Far access forces dirty eviction without allocating the intervening map.
    let far = w.cell(1_000_000, -1_000_000).unwrap();
    assert!(w.resident_count() <= 2);
    assert_eq!(w.cell(-65, -1).unwrap(), marker(731));
    w.unload_all().unwrap();
    assert_eq!(w.resident_count(), 0);
    assert!(w.saved_chunk_count() >= 6);
    drop(w);
    let mut loaded = InfdevWorld::open(&directory, 1).unwrap();
    assert_eq!(loaded.seed(), 42);
    assert_eq!(loaded.position(), (-65, -1));
    assert_eq!(loaded.cell(-65, -1).unwrap(), marker(731));
    assert_eq!(loaded.cell(64, 64).unwrap(), marker(23));
    assert_eq!(loaded.cell(1_000_000, -1_000_000).unwrap(), far);
    assert_eq!(loaded.resident_count(), 1);
}

#[test]
fn interrupted_manifest_commit_and_torn_orphan_never_replace_old_checkpoint() {
    let root = TestRoot::new();
    let mut w = root.world(2);
    let directory = w.directory().to_path_buf();
    let old = w.cell(0, 0).unwrap();
    let manifest = fs::read(directory.join("manifest.json")).unwrap();
    let old_files: Vec<_> = fs::read_dir(directory.join("chunks"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    w.set_cell(0, 0, marker(731)).unwrap();
    assert!(
        w.checkpoint_with_hook(|| Err("simulated power loss before manifest commit".into()))
            .is_err()
    );
    assert_eq!(fs::read(directory.join("manifest.json")).unwrap(), manifest);
    let orphan = fs::read_dir(directory.join("chunks"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| !old_files.contains(p))
        .unwrap();
    fs::write(&orphan, b"torn").unwrap();
    drop(w);
    let mut w = InfdevWorld::open(&directory, 2).unwrap();
    assert_eq!(w.cell(0, 0).unwrap(), old);
    w.set_cell(0, 0, marker(731)).unwrap();
    w.checkpoint().unwrap();
    drop(w);
    assert_eq!(fs::read(&orphan).unwrap(), b"torn");
    assert_eq!(
        InfdevWorld::open(&directory, 2)
            .unwrap()
            .cell(0, 0)
            .unwrap(),
        marker(731)
    );
}

#[test]
fn corrupt_committed_chunk_rolls_back_whole_checkpoint_or_refuses_without_regeneration() {
    let root = TestRoot::new();
    let mut w = root.world(2);
    let directory = w.directory().to_path_buf();
    let old = w.cell(0, 0).unwrap();
    w.set_cell(0, 0, marker(23)).unwrap();
    w.checkpoint().unwrap();
    drop(w);
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    let hash = saved["chunks"][0]["object"]["hash"].as_u64().unwrap();
    let latest = fs::read_dir(directory.join("chunks"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains(&format!("{hash:016x}"))
        })
        .unwrap();
    fs::write(&latest, b"bad committed data").unwrap();
    let mut restored = InfdevWorld::open(&directory, 2).unwrap();
    assert!(restored.recovered_backup());
    assert_eq!(restored.cell(0, 0).unwrap(), old);
    drop(restored);
    // Losing every valid referenced object must fail, even though seed is known.
    for file in fs::read_dir(directory.join("chunks")).unwrap() {
        fs::write(file.unwrap().path(), b"bad").unwrap();
    }
    let before = fs::read(directory.join("manifest.json")).unwrap();
    assert!(InfdevWorld::open(&directory, 2).is_err());
    assert_eq!(fs::read(directory.join("manifest.json")).unwrap(), before);
}

#[test]
fn duplicate_targets_unknown_metadata_traversal_and_fixed_worlds_are_refused() {
    let root = TestRoot::new();
    let w = root.world(2);
    let directory = w.directory().to_path_buf();
    assert!(InfdevWorld::open(&directory, 2).is_err()); // real Windows sharing lock
    assert!(InfdevWorld::create(&root.0, "Test", 43, 2).is_err());
    assert!(InfdevWorld::create(&root.0, "../escape", 43, 2).is_err());
    assert!(InfdevWorld::create(&root.0, "Other", 43, 0).is_err());
    drop(w);
    let mut saved: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    saved["generator"] = "UnknownFuture".into();
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(&saved).unwrap(),
    )
    .unwrap();
    assert!(
        InfdevWorld::open(&directory, 2)
            .unwrap_err_string()
            .contains("unknown experimental")
    );
    saved["generator"] = "InfdevHeightPrototypeV1".into();
    saved["chunks"][0]["coord"]["x"] = i32::MAX.into();
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(&saved).unwrap(),
    )
    .unwrap();
    assert!(InfdevWorld::open(&directory, 2).is_err());
    fs::write(directory.join("state.json"), b"original native save").unwrap();
    assert!(InfdevWorld::open(&directory, 2).is_err());
    assert_eq!(
        fs::read(directory.join("state.json")).unwrap(),
        b"original native save"
    );
}

// Avoid requiring Debug on the exclusive, file-backed World just to inspect Err.
trait ErrorText {
    fn unwrap_err_string(self) -> String;
}
impl ErrorText for Result<InfdevWorld, String> {
    fn unwrap_err_string(self) -> String {
        match self {
            Err(e) => e,
            Ok(_) => panic!("expected rejection"),
        }
    }
}

#[test]
fn sharing_denial_keeps_dirty_ram_and_previous_checkpoint_retryable() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = TestRoot::new();
    let mut w = root.world(1);
    let directory = w.directory().to_path_buf();
    let before = fs::read(directory.join("manifest.json")).unwrap();
    w.set_cell(0, 0, marker(731)).unwrap();
    let denial = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(directory.join("manifest.json"))
        .unwrap();
    assert!(w.checkpoint().is_err());
    assert_eq!(w.cell(0, 0).unwrap(), marker(731));
    drop(denial);
    assert_eq!(fs::read(directory.join("manifest.json")).unwrap(), before);
    w.checkpoint().unwrap();
    drop(w);
    assert_eq!(
        InfdevWorld::open(&directory, 1)
            .unwrap()
            .cell(0, 0)
            .unwrap(),
        marker(731)
    );
}

#[test]
fn oversized_manifest_recovers_backup_and_missing_committed_objects_never_regenerate() {
    let root = TestRoot::new();
    let mut w = root.world(2);
    let directory = w.directory().to_path_buf();
    w.set_cell(-1, -1, marker(23)).unwrap();
    w.checkpoint().unwrap();
    drop(w);
    let manifest = directory.join("manifest.json");
    fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&manifest)
        .unwrap()
        .set_len(2 * 1024 * 1024 + 1)
        .unwrap();
    let w = InfdevWorld::open(&directory, 2).unwrap();
    assert!(w.recovered_backup());
    drop(w);
    // Delete only fixture-owned objects to simulate a lost disk file.
    for file in fs::read_dir(directory.join("chunks")).unwrap() {
        fs::remove_file(file.unwrap().path()).unwrap();
    }
    let before = fs::read(&manifest).unwrap();
    assert!(InfdevWorld::open(&directory, 2).is_err());
    assert_eq!(fs::read(&manifest).unwrap(), before);
    assert_eq!(fs::read_dir(directory.join("chunks")).unwrap().count(), 0);
}
