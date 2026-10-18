use super::*;
use std::{
    io::Read,
    sync::atomic::{AtomicU64, Ordering},
};

struct Owned(PathBuf);
impl Owned {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "minicraft-infdev-v2-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn world(&self, capacity: usize) -> World {
        World::create(&self.0, "World", 42, capacity).unwrap()
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn edited(data: u16) -> Cell {
    Cell {
        ground: Ground::Flower,
        data,
    }
}
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn add(root: &Path, map: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in fs::read_dir(root).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                add(&p, map);
            } else {
                if p.file_name().unwrap() == "session.lock" {
                    assert_eq!(fs::metadata(&p).unwrap().len(), 0);
                    map.insert(p, Vec::new());
                } else {
                    map.insert(p.clone(), fs::read(p).unwrap());
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    add(root, &mut out);
    out
}

#[test]
fn original_java_complete_surface_chunks_preserve_all_fields_and_classify_time_choices() {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(include_bytes!(
        "../../../../tests/fixtures/infdev4a/terrain-observation.zip"
    )))
    .unwrap();
    let mut order = String::new();
    zip.by_name("biome-order.txt")
        .unwrap()
        .read_to_string(&mut order)
        .unwrap();
    assert_eq!(
        order,
        "Rock\nForest\nOcean\nRiver\nDesert\nRiverBank\nSurface\n"
    );
    let mut cases = String::new();
    zip.by_name("terrain-cases.tsv")
        .unwrap()
        .read_to_string(&mut cases)
        .unwrap();
    let mut cells = 0;
    let mut biomes = std::collections::BTreeSet::new();
    let mut eligible = 0;
    let mut differences = 0;
    for line in cases.lines() {
        let v: Vec<_> = line.split('\t').collect();
        let mut bytes = Vec::new();
        zip.by_name(v[3]).unwrap().read_to_end(&mut bytes).unwrap();
        biomes.extend(bytes[3..].iter().step_by(5).copied());
        let report = terrain::compare_observation(
            &bytes,
            v[0].parse().unwrap(),
            v[1].parse().unwrap(),
            v[2].parse().unwrap(),
        )
        .unwrap();
        cells += report.cells;
        eligible += report.time_eligible;
        differences += report.time_choice_differences;
    }
    assert_eq!(cells, 15 * 4096);
    assert_eq!(biomes.len(), 7);
    assert!(eligible > 1000);
    println!(
        "Java complete chunks: {cells} cells, {eligible} time-RNG eligible, {differences} actual time-choice differences (not equality claims)"
    );
}

#[test]
fn java_generation_is_deterministic_order_independent_and_crosses_i32_and_i64_boundaries() {
    let a = terrain::generate(42, -1, 0);
    let _ = terrain::generate(-1, 0, -1);
    assert_eq!(a, terrain::generate(42, -1, 0));
    for x in [-65, -64, -1, 0, 63, 64, i64::MIN, i64::MAX] {
        let s = terrain::sample(42, x, -1);
        let c = terrain::generate(42, Key::at(x, -1).x, -1);
        assert_eq!(s.cell, c[Key::local(x, -1)]);
    }
    let root = Owned::new();
    let mut w = root.world(2);
    w.visit(i64::from(i32::MAX) + 1, -65).unwrap();
    w.set_cell(i64::from(i32::MAX) + 1, -65, edited(9)).unwrap();
    w.step(1, 0).unwrap();
    let directory = w.directory.clone();
    w.unload_all().unwrap();
    drop(w);
    let mut w = World::open(&directory, 1).unwrap();
    assert_eq!(w.position(), (i64::from(i32::MAX) + 2, -65));
    assert_eq!(w.cell(i64::from(i32::MAX) + 1, -65).unwrap(), edited(9));
    w.visit(i64::MAX, 0).unwrap();
    assert!(w.step(1, 0).is_err());
}

#[test]
fn legacy_upgrade_is_a_copy_preserves_source_and_keeps_unvisited_generation_contract() {
    let root = Owned::new();
    let mut old = super::super::InfdevWorld::create(&root.0, "Old", -1, 2).unwrap();
    old.set_cell(
        -1,
        -1,
        Cell {
            ground: Ground::Rock,
            data: 731,
        },
    )
    .unwrap();
    old.checkpoint().unwrap();
    let source = old.directory.clone();
    drop(old);
    let before = files(&source);
    let mut new = World::upgrade_legacy(&source, &root.0, "Converted", 2).unwrap();
    assert_eq!(new.generator(), Generator::LegacyHeightV1);
    assert_eq!(
        new.cell(-1, -1).unwrap(),
        Cell {
            ground: Ground::Rock,
            data: 731
        }
    );
    let expected =
        super::super::generation::generate(-1, super::super::ChunkCoord { x: 20, y: -20 });
    assert_eq!(new.cell(20 * 64, -20 * 64).unwrap(), expected[0]);
    assert_eq!(files(&source), before);
    let dir = new.directory.clone();
    new.checkpoint().unwrap();
    drop(new);
    assert_eq!(
        World::open(&dir, 1).unwrap().generator(),
        Generator::LegacyHeightV1
    );
    assert!(World::upgrade_legacy(&source, &root.0, "Converted", 2).is_err());
    fs::remove_file(source.join("session.lock")).unwrap();
    let before = files(&source);
    let refused = World::upgrade_legacy(&source, &root.0, "MissingLock", 2);
    assert!(refused.is_err());
    assert_eq!(files(&source), before);
    assert!(!root.0.join(NAMESPACE).join("MissingLock").exists());
}

#[test]
fn complete_but_unsynced_objects_must_be_synced_again_before_reuse() {
    let root = Owned::new();
    let w = root.world(1);
    for domain in [objects::Domain::Nodes, objects::Domain::Chunks] {
        let bytes = b"owned unreferenced retry probe";
        let mut syncs = 0;
        for _ in 0..2 {
            let result = objects::write_with_sync(&w.directory, domain, bytes, |_| {
                syncs += 1;
                Err(std::io::Error::other("injected sync failure"))
            });
            assert!(result.unwrap_err().contains("injected sync failure"));
        }
        // First call created complete bytes; the second reused them and STILL
        // propagated the failed sync. Only a successful sync permits reuse.
        assert_eq!(syncs, 2);
        let object = objects::write_with_sync(&w.directory, domain, bytes, |f| {
            syncs += 1;
            f.sync_all()
        })
        .unwrap();
        assert_eq!(syncs, 3);
        assert_eq!(objects::read(&w.directory, domain, object).unwrap(), bytes);
    }
}

#[test]
fn gc_retains_current_backup_and_unknown_files_reclaims_only_unreachable_objects() {
    let root = Owned::new();
    let mut w = root.world(2);
    let dir = w.directory.clone();
    for value in 1..=4 {
        w.set_cell(0, 0, edited(value)).unwrap();
        w.checkpoint().unwrap();
    }
    let primary = fs::read(dir.join("checkpoint.json")).unwrap();
    let backup = fs::read(dir.join("checkpoint.json.bak")).unwrap();
    let protected = w.validate_root(w.saved.root).unwrap();
    let old = World::parse_checkpoint(std::str::from_utf8(&backup).unwrap()).unwrap();
    let backup_objects = w.validate_root(old.root).unwrap();
    assert!(
        w.collect_with_hook(|removed| if removed == 1 {
            Err("simulated stop during sweep".into())
        } else {
            Ok(())
        })
        .is_err()
    );
    w.validate_root(w.saved.root).unwrap();
    w.validate_root(old.root).unwrap();
    let note = dir.join("chunks").join("personal-notes.txt");
    fs::write(&note, b"keep").unwrap();
    let report = w.collect_garbage().unwrap();
    assert!(report.removed_files > 0);
    assert_eq!(report.unknown_files, 1);
    assert_eq!(fs::read(&note).unwrap(), b"keep");
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), primary);
    assert_eq!(fs::read(dir.join("checkpoint.json.bak")).unwrap(), backup);
    for (d, o) in protected.union(&backup_objects) {
        assert!(objects::read(&dir, *d, *o).is_ok());
    }
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.cell(0, 0).unwrap(), edited(4));
    drop(w);
    fs::write(dir.join("checkpoint.json"), b"broken").unwrap();
    let mut recovered = World::open(&dir, 1).unwrap();
    assert!(recovered.recovered_backup());
    assert_eq!(recovered.cell(0, 0).unwrap(), edited(3));
    let before = files(&dir);
    assert!(recovered.collect_garbage().is_err());
    assert_eq!(files(&dir), before);
}

#[test]
fn automatic_collection_and_avl_rotations_keep_live_index_balanced_and_old_chunks_available() {
    let root = Owned::new();
    let mut w = root.world(2);
    let mut collected = false;
    for i in 1..=140 {
        if w.saved.sequence == 127 {
            let path = w.directory.clone();
            drop(w);
            w = World::open(&path, 2).unwrap();
        }
        let key = Key {
            x: i * 37 % 257 + 1,
            y: if i % 2 == 0 { -1 } else { 1 },
        };
        w.resident.clear();
        w.resident.insert(
            key,
            Cached {
                cells: vec![edited((i % 12) as u16); 4096],
                dirty: true,
                touched: 0,
            },
        );
        let before = if w.saved.sequence % 128 == 127 {
            Some(files(&w.directory).len())
        } else {
            None
        };
        w.checkpoint().unwrap();
        if let Some(n) = before {
            assert!(files(&w.directory).len() < n);
            assert!(w.collection_warning().is_none());
            collected = true;
        }
    }
    assert!(collected);
    assert_eq!(w.saved_chunk_count().unwrap(), 141);
    w.validate_root(w.saved.root).unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    for i in [1, 2, 63, 127, 140] {
        assert_eq!(
            w.cell((i * 37 % 257 + 1) * 64, if i % 2 == 0 { -64 } else { 64 })
                .unwrap(),
            edited((i % 12) as u16)
        );
    }
}

#[test]
fn new_storage_rejects_future_versions_and_preserves_checkpoints_on_sharing_and_gc_failures() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = Owned::new();
    let mut w = root.world(1);
    let dir = w.directory.clone();
    assert!(World::open(&dir, 1).is_err());
    assert!(World::create(&root.0, "World", 0, 1).is_err());
    assert!(World::create(&root.0, "../escape", 0, 1).is_err());
    assert!(World::create(&root.0, "Zero", 0, 0).is_err());
    let before = fs::read(dir.join("checkpoint.json")).unwrap();
    w.set_cell(0, 0, edited(3)).unwrap();
    let deny = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(dir.join("checkpoint.json"))
        .unwrap();
    assert!(w.checkpoint().is_err());
    assert_eq!(w.cell(0, 0).unwrap(), edited(3));
    drop(deny);
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), before);
    w.checkpoint().unwrap();
    let older_backup = fs::read(dir.join("checkpoint.json.bak")).unwrap();
    w.set_cell(0, 0, edited(5)).unwrap();
    let deny_backup = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(dir.join("checkpoint.json.bak"))
        .unwrap();
    assert!(w.checkpoint().is_err());
    drop(deny_backup);
    // Primary installation succeeded; retry must acknowledge it while keeping
    // the last older backup, rather than rotating the duplicate new root.
    w.checkpoint().unwrap();
    assert_eq!(
        fs::read(dir.join("checkpoint.json.bak")).unwrap(),
        older_backup
    );
    let good = fs::read(dir.join("checkpoint.json")).unwrap();
    let backup = fs::read(dir.join("checkpoint.json.bak")).unwrap();
    drop(w);
    let mut future: serde_json::Value = serde_json::from_slice(&good).unwrap();
    future["generator"] = "UnknownFuture".into();
    let unknown = serde_json::to_vec(&future).unwrap();
    fs::write(dir.join("checkpoint.json"), &unknown).unwrap();
    assert!(World::open(&dir, 1).is_err());
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), unknown);
    fs::write(dir.join("checkpoint.json"), &good).unwrap();
    let mut w = World::open(&dir, 1).unwrap();
    fs::write(dir.join("checkpoint.json.bak"), b"ambiguous backup").unwrap();
    let chunks = files(&dir.join("chunks"));
    let nodes = files(&dir.join("nodes"));
    assert!(w.collect_garbage().is_err());
    assert_eq!(files(&dir.join("chunks")), chunks);
    assert_eq!(files(&dir.join("nodes")), nodes);
    assert!(dir.join("gc-recovery.hold").is_file());
    fs::write(dir.join("checkpoint.json.bak"), backup).unwrap();
    w.set_cell(0, 0, edited(4)).unwrap();
    w.checkpoint().unwrap();
    assert!(w.collect_garbage().is_err());
}

#[test]
fn interrupted_publication_missing_objects_and_gc_preflight_never_drop_live_data() {
    let root = Owned::new();
    let mut w = root.world(2);
    let dir = w.directory.clone();
    let old = w.cell(0, 0).unwrap();
    let before = fs::read(dir.join("checkpoint.json")).unwrap();
    w.set_cell(0, 0, edited(11)).unwrap();
    assert!(
        w.commit_with_hook(|| Err("simulated termination before root publication".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), before);
    drop(w);
    let mut w = World::open(&dir, 2).unwrap();
    assert_eq!(w.cell(0, 0).unwrap(), old);
    assert!(w.collect_garbage().unwrap().removed_files > 0);
    w.set_cell(0, 0, edited(7)).unwrap();
    w.checkpoint().unwrap();
    let object = w.find(w.saved.root, Key { x: 0, y: 0 }).unwrap().unwrap();
    drop(w);
    let target = objects::path(&dir, objects::Domain::Chunks, object);
    fs::write(&target, b"torn").unwrap();
    let mut recovered = World::open(&dir, 1).unwrap();
    assert!(recovered.recovered_backup());
    assert_eq!(recovered.cell(0, 0).unwrap(), old);
    drop(recovered);
    let cp =
        World::parse_checkpoint(&fs::read_to_string(dir.join("checkpoint.json")).unwrap()).unwrap();
    let mut w = World::open(&dir, 1).unwrap();
    let oldref = w.find(cp.root, Key { x: 0, y: 0 }).unwrap().unwrap();
    drop(w);
    fs::remove_file(objects::path(&dir, objects::Domain::Chunks, oldref)).unwrap();
    assert!(World::open(&dir, 1).is_err());
}

#[test]
fn collection_revalidates_distinct_backup_data_before_removing_any_object() {
    let root = Owned::new();
    let mut w = root.world(2);
    let initial = w.find(w.saved.root, Key { x: 0, y: 0 }).unwrap().unwrap();
    w.set_cell(0, 0, edited(12)).unwrap();
    w.checkpoint().unwrap();
    // First successful GC validates both roots. A later GC must not reuse
    // that validation after the backup-only data has become corrupt.
    w.collect_garbage().unwrap();
    let orphan = objects::write(
        &w.directory,
        objects::Domain::Chunks,
        b"owned unreferenced collection probe",
    )
    .unwrap();
    let orphan_path = objects::path(&w.directory, objects::Domain::Chunks, orphan);
    fs::write(
        objects::path(&w.directory, objects::Domain::Chunks, initial),
        b"corrupt backup-only chunk",
    )
    .unwrap();
    let before = files(&w.directory.join("chunks"));
    assert!(w.collect_garbage().is_err());
    assert!(orphan_path.is_file());
    assert_eq!(files(&w.directory.join("chunks")), before);
    assert!(w.directory.join("gc-recovery.hold").is_file());
    assert_eq!(w.cell(0, 0).unwrap(), edited(12));
}

#[test]
fn paged_index_exceeds_old_8192_limit_without_manifest_growth_and_keeps_lru_bounded() {
    let root = Owned::new();
    let mut w = root.world(2);
    let dir = w.directory.clone();
    // Real disk index fixture: 8205 edited chunks with deduplicated known data,
    // not a claim that Java naturally generates identical terrain at each key.
    let cells = vec![edited(6); 4096];
    let chunk = w.write_chunk(&cells).unwrap();
    let entries: Vec<_> = (0..8205).map(|x| (Key { x, y: 0 }, chunk)).collect();
    w.saved.root = w.build_balanced(&entries).unwrap();
    w.resident.clear();
    w.commit_with_hook(|| Ok(())).unwrap();
    assert_eq!(w.saved_chunk_count().unwrap(), 8205);
    assert!(fs::metadata(dir.join("checkpoint.json")).unwrap().len() < 512);
    for x in 0..8205 {
        assert_eq!(w.cell(x * 64, 0).unwrap(), edited(6));
        assert!(w.resident_count() <= 2);
    }
    assert!(w.nodes.len() <= 512);
    for x in [0, 1, 63, 64, 8192, 8204] {
        w.set_cell(x * 64, 0, edited(9)).unwrap();
        w.checkpoint().unwrap();
    }
    w.collect_garbage().unwrap();
    drop(w);
    let mut w = World::open(&dir, 2).unwrap();
    assert_eq!(w.saved_chunk_count().unwrap(), 8205);
    assert_eq!(w.cell(8204 * 64, 0).unwrap(), edited(9));
    assert!(fs::metadata(dir.join("checkpoint.json")).unwrap().len() < 512);
}
