use super::*;
use std::{
    io::Read,
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) struct Owned(pub(super) PathBuf);
impl Owned {
    pub(super) fn new() -> Self {
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
            depth: 0,
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
    let object = w.find(w.saved.root, Key::at(0, 0)).unwrap().unwrap();
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
    let oldref = w.find(cp.root, Key::at(0, 0)).unwrap().unwrap();
    drop(w);
    fs::remove_file(objects::path(&dir, objects::Domain::Chunks, oldref)).unwrap();
    assert!(World::open(&dir, 1).is_err());
}

#[test]
fn collection_revalidates_distinct_backup_data_before_removing_any_object() {
    let root = Owned::new();
    let mut w = root.world(2);
    let initial = w.find(w.saved.root, Key::at(0, 0)).unwrap().unwrap();
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
    let entries: Vec<_> = (0..8205)
        .map(|x| (Key { depth: 0, x, y: 0 }, chunk))
        .collect();
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

#[test]
fn layered_chunks_keep_depth_identity_across_eviction_collection_and_disk_restart() {
    let root = Owned::new();
    let mut w = World::create_layers(&root.0, "Layers", 42, 2).unwrap();
    assert_eq!(
        w.directory.parent().unwrap().file_name().unwrap(),
        LAYERED_NAMESPACE
    );
    assert_eq!(w.generator(), Generator::JavaLayersV1);
    assert_eq!(w.depth(), 0);
    for depth in -4..=0 {
        w.set_cell_at(
            depth,
            -65,
            64,
            Cell {
                ground: Ground::Dirt,
                data: (100 - depth) as u16,
            },
        )
        .unwrap();
        assert_eq!(
            w.depth(),
            0,
            "off-screen chunk access cannot move the explorer"
        );
        assert!(w.resident_count() <= 2);
    }
    w.visit_layer(-4, -65, 64).unwrap();
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
    let dir = w.directory.clone();
    let cp = fs::read_to_string(dir.join("checkpoint.json")).unwrap();
    assert!(cp.contains(LAYERED_FORMAT));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&cp).unwrap()["depth"],
        -4
    );
    w.unload_all().unwrap();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.depth(), -4);
    assert_eq!(w.position(), (-65, 64));
    for depth in -4..=0 {
        assert_eq!(
            w.cell_at(depth, -65, 64).unwrap(),
            Cell {
                ground: Ground::Dirt,
                data: (100 - depth) as u16
            }
        );
        assert_eq!(w.depth(), -4);
        let reference = w
            .find(w.saved.root, Key::at_depth(depth, -65, 64))
            .unwrap()
            .unwrap();
        assert_eq!(
            &objects::read(&dir, objects::Domain::Chunks, reference).unwrap()[..8],
            b"MCIFCH03"
        );
    }
    let before = (w.depth(), w.position());
    for invalid in [i8::MIN, -5, 1, i8::MAX] {
        assert!(w.visit_layer(invalid, 0, 0).is_err());
        assert!(w.cell_at(invalid, 0, 0).is_err());
        assert!(w.set_cell_at(invalid, 0, 0, edited(0)).is_err());
        assert_eq!((w.depth(), w.position()), before);
    }
    w.visit_layer(0, -65, 64).unwrap();
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.depth(), 0);
    assert_eq!(w.cell_at(-4, -65, 64).unwrap().data, 104);
}

#[test]
fn original_v2_bytes_and_contract_remain_isolated_from_layered_worlds() {
    let root = Owned::new();
    let mut old = root.world(1);
    old.set_cell(-1, -1, edited(9)).unwrap();
    old.visit(-1, -1).unwrap();
    old.checkpoint().unwrap();
    let directory = old.directory.clone();
    let primary = fs::read(directory.join("checkpoint.json")).unwrap();
    assert!(!std::str::from_utf8(&primary).unwrap().contains("depth"));
    assert_eq!(
        serde_json::to_string(&Key::at(0, 0)).unwrap(),
        r#"{"x":0,"y":0}"#
    );
    let reference = old.find(old.saved.root, Key::at(-1, -1)).unwrap().unwrap();
    assert_eq!(
        &objects::read(&directory, objects::Domain::Chunks, reference).unwrap()[..8],
        b"MCIFCH02"
    );
    let before = files(&directory);
    assert!(old.visit_layer(-1, -1, -1).is_err());
    assert!(
        old.set_cell(
            -1,
            -1,
            Cell {
                ground: Ground::Dirt,
                data: 4
            }
        )
        .is_err()
    );
    assert_eq!(old.cell(-1, -1).unwrap(), edited(9));
    assert_eq!(files(&directory), before);
    drop(old);
    let mut reopened = World::open(&directory, 1).unwrap();
    assert!(!reopened.is_layered());
    assert_eq!(reopened.depth(), 0);
    assert_eq!(reopened.cell(-1, -1).unwrap(), edited(9));
    reopened.checkpoint().unwrap();
    assert_eq!(
        fs::read(directory.join("checkpoint.json")).unwrap(),
        primary
    );
    drop(reopened);
    for (field, value) in [
        ("depth", serde_json::json!(-1)),
        ("generator", serde_json::json!("JavaLayersV1")),
        ("format", serde_json::json!(LAYERED_FORMAT)),
    ] {
        let mut invalid: serde_json::Value = serde_json::from_slice(&primary).unwrap();
        invalid[field] = value;
        let bytes = serde_json::to_vec(&invalid).unwrap();
        fs::write(directory.join("checkpoint.json"), &bytes).unwrap();
        assert!(
            World::open(&directory, 1).is_err(),
            "cannot reinterpret v2 {field}"
        );
        assert_eq!(fs::read(directory.join("checkpoint.json")).unwrap(), bytes);
    }
    fs::write(directory.join("checkpoint.json"), primary).unwrap();
    // A checksummed index page with an unsupported depth is still invalid.
    let mut old = World::open(&directory, 1).unwrap();
    let node_reference = old.saved.root.unwrap();
    let mut node = old.node(node_reference).unwrap();
    node.key.depth = -1;
    let incompatible = objects::write(
        &directory,
        objects::Domain::Nodes,
        &serde_json::to_vec(&node).unwrap(),
    )
    .unwrap();
    assert!(old.node(incompatible).is_err());
    let v2_chunk = objects::read(&directory, objects::Domain::Chunks, reference).unwrap();
    let layered = World::create_layers(&root.0, "World", 42, 1).unwrap();
    let layered_dir = layered.directory.clone();
    let wrong_header = objects::write(&layered_dir, objects::Domain::Chunks, &v2_chunk).unwrap();
    assert!(layered.read_chunk(wrong_header).is_err());
    let valid = fs::read(layered_dir.join("checkpoint.json")).unwrap();
    drop(layered);
    for (field, value) in [
        ("depth", serde_json::json!(-5)),
        ("depth", serde_json::json!(1)),
        ("generator", serde_json::json!("JavaSurfaceV1")),
    ] {
        let mut invalid: serde_json::Value = serde_json::from_slice(&valid).unwrap();
        invalid[field] = value;
        let bytes = serde_json::to_vec(&invalid).unwrap();
        fs::write(layered_dir.join("checkpoint.json"), &bytes).unwrap();
        assert!(World::open(&layered_dir, 1).is_err());
        assert_eq!(
            fs::read(layered_dir.join("checkpoint.json")).unwrap(),
            bytes
        );
    }
    let mut missing_root: serde_json::Value = serde_json::from_slice(&valid).unwrap();
    missing_root["root"] = serde_json::json!({"hash": 0, "slot": 0});
    let wrong_primary = serde_json::to_vec(&missing_root).unwrap();
    fs::write(layered_dir.join("checkpoint.json"), &wrong_primary).unwrap();
    fs::write(
        layered_dir.join("checkpoint.json.bak"),
        fs::read(directory.join("checkpoint.json")).unwrap(),
    )
    .unwrap();
    assert!(
        World::open(&layered_dir, 1)
            .err()
            .unwrap()
            .contains("different world format")
    );
    assert_eq!(
        fs::read(layered_dir.join("checkpoint.json")).unwrap(),
        wrong_primary
    );
}

#[test]
fn layered_failed_transition_publication_and_corrupt_current_chunk_recover_complete_backup() {
    let root = Owned::new();
    let mut w = World::create_layers(&root.0, "Recover", 42, 2).unwrap();
    w.visit_layer(-1, -65, -1).unwrap();
    w.set_cell(-65, -1, edited(7)).unwrap();
    w.checkpoint().unwrap();
    let directory = w.directory.clone();
    let committed = fs::read(directory.join("checkpoint.json")).unwrap();
    w.visit_layer(-2, -65, -1).unwrap();
    w.set_cell(-65, -1, edited(8)).unwrap();
    assert!(
        w.commit_with_hook(|| Err("termination before layered root publication".into()))
            .is_err()
    );
    assert_eq!(
        fs::read(directory.join("checkpoint.json")).unwrap(),
        committed
    );
    drop(w);
    let mut w = World::open(&directory, 2).unwrap();
    assert_eq!(w.depth(), -1);
    assert_eq!(w.cell(-65, -1).unwrap(), edited(7));
    w.collect_garbage().unwrap();
    w.set_cell_at(-2, -65, -1, edited(10)).unwrap();
    w.checkpoint().unwrap();
    let invalid = w
        .find(w.saved.root, Key::at_depth(-2, -65, -1))
        .unwrap()
        .unwrap();
    w.resident.clear();
    fs::write(
        objects::path(&directory, objects::Domain::Chunks, invalid),
        b"torn underground chunk",
    )
    .unwrap();
    let before = (w.depth(), w.position());
    assert!(w.visit_layer(-2, -65, -1).is_err());
    assert_eq!((w.depth(), w.position()), before);
    drop(w);
    let mut recovered = World::open(&directory, 1).unwrap();
    assert!(recovered.recovered_backup());
    assert_eq!(recovered.depth(), -1);
    assert_eq!(recovered.position(), (-65, -1));
    assert_eq!(recovered.cell(-65, -1).unwrap(), edited(7));
    let preserved = files(&directory);
    assert!(recovered.collect_garbage().is_err());
    assert_eq!(files(&directory), preserved);
}

#[test]
fn survival_publication_failure_and_corrupt_player_recover_player_and_map_together() {
    use crate::item::ItemId;
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "AtomicPlayer", 42, 1).unwrap();
    w.survival
        .as_mut()
        .unwrap()
        .player
        .inventory
        .add(ItemId::Wood, 23);
    w.set_cell_at(
        -1,
        -65,
        -1,
        Cell {
            ground: Ground::Dirt,
            data: 731,
        },
    )
    .unwrap();
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    let old = fs::read(dir.join("checkpoint.json")).unwrap();
    w.survival
        .as_mut()
        .unwrap()
        .player
        .inventory
        .remove(ItemId::Wood, 10);
    w.set_cell_at(
        -1,
        -65,
        -1,
        Cell {
            ground: Ground::Dirt,
            data: 23,
        },
    )
    .unwrap();
    assert!(
        w.commit_with_hook(|| Err("termination before player/map publication".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), old);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Wood),
        23
    );
    assert_eq!(w.cell_at(-1, -65, -1).unwrap().data, 731);
    w.survival.as_mut().unwrap().player.health = 9;
    w.checkpoint().unwrap();
    let prior_session = w.saved.session.unwrap();
    w.survival.as_mut().unwrap().player.health = 8;
    w.set_cell_at(
        -1,
        -65,
        -1,
        Cell {
            ground: Ground::Dirt,
            data: 41,
        },
    )
    .unwrap();
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
    assert!(
        objects::read(&dir, objects::Domain::Sessions, prior_session).is_ok(),
        "GC protects backup player"
    );
    let damaged = w.saved.session.unwrap();
    drop(w);
    fs::write(
        objects::path(&dir, objects::Domain::Sessions, damaged),
        b"torn player",
    )
    .unwrap();
    let mut recovered = World::open(&dir, 1).unwrap();
    assert!(recovered.recovered_backup());
    assert_eq!(recovered.survival().unwrap().player.health, 9);
    assert_eq!(recovered.cell_at(-1, -65, -1).unwrap().data, 731);
    assert!(recovered.collect_garbage().is_err());
}

#[test]
fn survival_format_and_unknown_instance_fields_cannot_downgrade_into_exploration() {
    let root = Owned::new();
    let mut old = World::create_layers(&root.0, "Separate", 42, 2).unwrap();
    let before = files(&old.directory);
    assert!(
        old.set_cell(
            0,
            0,
            Cell {
                ground: Ground::Hole,
                data: 0
            }
        )
        .is_err()
    );
    assert_eq!(files(&old.directory), before);
    let old_checkpoint = fs::read(old.directory.join("checkpoint.json")).unwrap();
    let w = World::create_survival(&root.0, "Separate", 42, 2).unwrap();
    assert_ne!(old.directory, w.directory);
    let dir = w.directory.clone();
    let checkpoint = fs::read(dir.join("checkpoint.json")).unwrap();
    let original_session = w.saved.session.unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(
        &objects::read(&dir, objects::Domain::Sessions, original_session).unwrap(),
    )
    .unwrap();
    value["player"]["future_owned_item"] = serde_json::json!(731);
    let unknown = objects::write(
        &dir,
        objects::Domain::Sessions,
        &serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let mut bad = w.saved.clone();
    bad.session = Some(unknown);
    drop(w);
    fs::write(
        dir.join("checkpoint.json"),
        serde_json::to_vec(&bad).unwrap(),
    )
    .unwrap();
    assert!(
        World::open(&dir, 2).is_err(),
        "unknown player fields cannot be silently removed"
    );
    // A valid exploration backup is not a substitute for a Survival session.
    fs::write(dir.join("checkpoint.json.bak"), old_checkpoint).unwrap();
    assert!(
        World::open(&dir, 2)
            .err()
            .unwrap()
            .contains("different world format")
    );
    fs::write(dir.join("checkpoint.json"), &checkpoint).unwrap();
    fs::write(
        objects::path(&dir, objects::Domain::Sessions, original_session),
        b"damaged",
    )
    .unwrap();
    assert!(World::open(&dir, 2).is_err());
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), checkpoint);
}

#[test]
fn survival_shovel_place_and_real_stairs_keep_materials_player_and_tools_on_disk() {
    use crate::{
        experimental::infdev::survival::Action,
        item::{ItemId, ToolItem, ToolKind, ToolTier},
        world::ActiveItem,
    };
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Placement", 42, 2).unwrap();
    // Constructed dirt/tool fixture isolates placement conservation; the
    // natural-resource and window tests separately earn materials from zero.
    w.set_cell(
        -65,
        -1,
        Cell {
            ground: Ground::Dirt,
            data: 0,
        },
    )
    .unwrap();
    w.visit(-65, -2).unwrap();
    w.survival
        .as_mut()
        .unwrap()
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Shovel, ToolTier::Wood))
        .unwrap();
    w.survival.as_mut().unwrap().player.active_item = Some(ActiveItem::Tool(0));
    w.survival_action(Action::Attack).unwrap();
    assert_eq!(w.cell(-65, -1).unwrap().ground, Ground::Hole);
    for _ in 0..60 {
        w.survival_action(Action::Tick).unwrap();
    }
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Dirt),
        1
    );
    let durability = w.survival().unwrap().player.inventory.tools()[0].durability;
    w.survival_action(Action::Inventory).unwrap();
    w.survival_action(Action::Confirm).unwrap();
    w.survival_action(Action::Attack).unwrap();
    assert_eq!(w.cell(-65, -1).unwrap().ground, Ground::Dirt);
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Dirt),
        0
    );
    assert!(w.survival().unwrap().drops.is_empty());
    assert_eq!(
        w.survival().unwrap().player.inventory.tools()[0].durability,
        durability
    );
    w.visit(-32, -20).unwrap();
    assert_eq!(w.cell(-32, -20).unwrap().ground, Ground::StairsDown);
    w.survival_action(Action::Pickup).unwrap();
    assert_eq!(w.depth(), -1);
    w.survival_action(Action::Pickup).unwrap();
    assert_eq!(w.depth(), 0);
    w.survival.as_mut().unwrap().player.health = 7;
    w.survival.as_mut().unwrap().player.stamina = 3;
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.position(), (-32, -20));
    assert_eq!(w.depth(), 0);
    assert_eq!(w.survival().unwrap().player.health, 7);
    assert_eq!(w.survival().unwrap().player.stamina, 3);
    assert_eq!(
        w.survival().unwrap().player.inventory.tools()[0].durability,
        durability
    );
    assert_eq!(w.cell(-65, -1).unwrap().ground, Ground::Dirt);
}

#[test]
fn survival_legacy_session_and_smelting_interruption_preserve_exact_ownership() {
    use crate::{
        experimental::infdev::survival::{Action, FoodRules, Panel, Station, StationKind, recipes},
        item::{ItemId, ToolItem, ToolKind, ToolTier},
    };
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "LegacyFood", 42, 1).unwrap();
    let position = w.position();
    let s = w.survival.as_mut().unwrap();
    s.environment = None;
    s.entity_rules = crate::experimental::infdev::survival::EntityRules::Legacy;
    s.food_rules = FoodRules::Legacy;
    s.death_rule = crate::experimental::infdev::survival::DeathRule::KeepInventory;
    s.station_rules = super::super::survival::StationRules::Session;
    s.stations.push(Station {
        depth: 0,
        x: position.0 + 1,
        y: position.1,
        kind: StationKind::Workbench,
    });
    s.player.inventory.add(ItemId::IronOre, 31);
    s.player.inventory.add(ItemId::Coal, 11);
    let mut tool = ToolItem::new(ToolKind::Pickaxe, ToolTier::Rock);
    tool.durability = 23;
    s.player.inventory.add_tool(tool).unwrap();
    w.checkpoint().unwrap();
    let old = objects::read(
        &w.directory,
        objects::Domain::Sessions,
        w.saved.session.unwrap(),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&old).unwrap();
    assert!(value.get("food_rules").is_none());
    assert!(value["stations"][0].get("kind").is_none());
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.survival().unwrap().food_rules == FoodRules::Legacy);
    assert_eq!(serde_json::to_vec(w.survival().unwrap()).unwrap(), old);
    // Test a new-rules checkpoint's atomic material/fuel/terrain publication.
    let s = w.survival.as_mut().unwrap();
    s.food_rules = FoodRules::JavaNormal;
    s.stations[0].kind = StationKind::Furnace;
    w.cell_at(-1, -65, -1).unwrap(); // preload before the transaction, even at LRU=1
    w.checkpoint().unwrap();
    let checkpoint = fs::read(dir.join("checkpoint.json")).unwrap();
    let s = w.survival.as_mut().unwrap();
    s.panel = Panel::Crafting(Some(StationKind::Furnace));
    s.selection = recipes(Some(StationKind::Furnace))
        .iter()
        .position(|r| r.product_name() == "Iron")
        .unwrap();
    w.survival_action(Action::Confirm).unwrap();
    w.survival_action(Action::Close).unwrap();
    w.set_cell_at(
        -1,
        -65,
        -1,
        Cell {
            ground: Ground::Dirt,
            data: 731,
        },
    )
    .unwrap();
    assert!(
        w.commit_with_hook(|| Err("interrupted smelt before publication".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), checkpoint);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    let s = w.survival().unwrap();
    assert_eq!(s.player.inventory.count(ItemId::IronOre), 31);
    assert_eq!(s.player.inventory.count(ItemId::Coal), 11);
    assert_eq!(s.player.inventory.count(ItemId::IronIngot), 0);
    assert_eq!(s.player.inventory.tools()[0].durability, 23);
    assert!(s.stations[0].kind == StationKind::Furnace);
    assert_ne!(w.cell_at(-1, -65, -1).unwrap().data, 731);
    w.collect_garbage().unwrap();
}

fn death_fixture(root: &Owned, name: &str) -> (World, crate::item::Inventory) {
    use crate::item::{ItemId, ItemStack, ToolItem, ToolKind, ToolTier};
    let mut w = World::create_survival(&root.0, name, 42, 1).unwrap();
    w.visit_layer(-1, -65, -65).unwrap();
    w.set_cell(
        -65,
        -65,
        Cell {
            ground: Ground::Dirt,
            data: 0,
        },
    )
    .unwrap();
    w.survival.as_mut().unwrap().environment = None;
    w.survival.as_mut().unwrap().entity_rules = super::super::survival::EntityRules::Legacy;
    w.survival.as_mut().unwrap().station_rules = super::super::survival::StationRules::Session;
    let p = &mut w.survival.as_mut().unwrap().player;
    p.inventory.add(ItemId::Wood, 731);
    p.inventory.add_stack(ItemStack::with_water(731));
    p.inventory.add_stack(ItemStack::with_water(23));
    p.inventory.add_stack(ItemStack {
        item: ItemId::WateringCan,
        count: 1,
        watering_content: None,
    });
    let mut t = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
    t.durability = 23;
    p.inventory.add_tool(t).unwrap();
    p.active_item = Some(crate::world::ActiveItem::Tool(0));
    let inventory = p.inventory.clone();
    w.checkpoint().unwrap();
    (w, inventory)
}

#[test]
fn death_chest_cross_depth_full_bag_instances_duplicate_touch_and_disk_gc() {
    use crate::{
        experimental::infdev::{entities::Kind, survival::Action},
        item::{Inventory, ToolItem, ToolKind, ToolTier},
    };
    let root = Owned::new();
    let (mut w, owned) = death_fixture(&root, "Death");
    w.survival_action(Action::DiagnosticDeath).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    assert!(w.survival().unwrap().player.active_item.is_none());
    let key = Key::at_depth(-1, -65, -65);
    for _ in 0..5 {
        w.survival_action(Action::Tick).unwrap();
    }
    let page = w.entity_page(key).unwrap();
    assert_eq!(page.entities.len(), 1);
    let Kind::DeathChest(chest) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(chest.inventory, owned);
    // Enter respawns, but must NOT retrieve an underground chest remotely.
    w.survival_action(Action::Interact).unwrap();
    assert_eq!(w.depth(), 0);
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    let spawn = w.position();
    w.visit_layer(-1, -65, -65).unwrap();
    for _ in 0..27 {
        w.survival
            .as_mut()
            .unwrap()
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
            .unwrap();
    }
    w.survival_action(Action::Tick).unwrap();
    let page = w.entity_page(key).unwrap();
    let Kind::DeathChest(chest) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(
        chest.inventory, owned,
        "full bag leaves every instance in the chest"
    );
    // Remove only the explicit capacity fixture, not any of the source items.
    w.survival.as_mut().unwrap().player.inventory = Inventory::new(27);
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    assert!(w.entity_page(key).unwrap().entities.is_empty());
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    w.visit_layer(0, spawn.0, spawn.1).unwrap();
    w.unload_all().unwrap();
    assert!(w.entity_cache.is_empty());
    w.collect_garbage().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    assert!(w.entity_page(key).unwrap().entities.is_empty());
}

#[test]
fn death_chest_partial_stack_pickup_never_loses_remainders() {
    use crate::{
        experimental::infdev::{entities::Kind, survival::Action},
        item::{ItemId, ToolItem, ToolKind, ToolTier},
    };
    let root = Owned::new();
    let (mut w, _) = death_fixture(&root, "PartialDeath");
    w.survival_action(Action::DiagnosticDeath).unwrap();
    w.survival_action(Action::Confirm).unwrap();
    w.visit_layer(-1, -65, -65).unwrap();
    let inv = &mut w.survival.as_mut().unwrap().player.inventory;
    inv.add(ItemId::Wood, crate::item::MAX_STACK - 1);
    for _ in 0..26 {
        inv.add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
            .unwrap();
    }
    w.survival_action(Action::Tick).unwrap();
    let page = w.entity_page(Key::at_depth(-1, -65, -65)).unwrap();
    let Kind::DeathChest(chest) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(chest.inventory.count(ItemId::Wood), 730);
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Wood),
        crate::item::MAX_STACK
    );
    assert_eq!(
        chest
            .inventory
            .slots()
            .iter()
            .filter(|s| s.item == ItemId::WateringCan)
            .count(),
        3
    );
    for _ in 0..10 {
        w.survival_action(Action::Tick).unwrap();
    }
    let page = w.entity_page(Key::at_depth(-1, -65, -65)).unwrap();
    let Kind::DeathChest(chest) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(chest.inventory.count(ItemId::Wood), 730);
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Wood),
        crate::item::MAX_STACK
    );
    let page = w.entity_page(Key::at_depth(-1, -65, -65)).unwrap();
    let Kind::DeathChest(chest) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(chest.inventory.count(ItemId::Wood), 730);
}

#[test]
fn death_and_retrieval_publication_faults_keep_a_single_inventory_owner() {
    use crate::experimental::infdev::{entities::Kind, survival::Action};
    let root = Owned::new();
    let (mut w, owned) = death_fixture(&root, "FaultDeath");
    let dir = w.directory.clone();
    let before = fs::read(dir.join("checkpoint.json")).unwrap();
    assert!(w.act_with_entities(Action::DiagnosticDeath).unwrap());
    w.set_cell(
        -65,
        -65,
        Cell {
            ground: Ground::Dirt,
            data: 731,
        },
    )
    .unwrap();
    assert!(
        w.commit_with_hook(|| Err("kill before death pointer publish".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), before);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    assert_eq!(w.cell(-65, -65).unwrap().data, 0);
    let key = Key::at_depth(-1, -65, -65);
    assert!(w.entity_page(key).unwrap().entities.is_empty());
    w.survival_action(Action::DiagnosticDeath).unwrap();
    w.survival_action(Action::Confirm).unwrap();
    w.visit_layer(-1, -65, -65).unwrap();
    let before = fs::read(dir.join("checkpoint.json")).unwrap();
    assert!(w.act_with_entities(Action::Tick).unwrap());
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    assert!(
        w.commit_with_hook(|| Err("kill before retrieval pointer publish".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), before);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    let page = w.entity_page(key).unwrap();
    let Kind::DeathChest(chest) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(chest.inventory, owned);
    w.collect_garbage().unwrap();
}

#[test]
fn corrupt_death_entity_rolls_back_entire_player_root_and_pins_gc() {
    use crate::experimental::infdev::survival::Action;
    let root = Owned::new();
    let (mut w, owned) = death_fixture(&root, "CorruptDeath");
    w.survival_action(Action::DiagnosticDeath).unwrap();
    let key = Key::at_depth(-1, -65, -65);
    let object = w.find(w.saved.entity_root, key).unwrap().unwrap();
    let dir = w.directory.clone();
    drop(w);
    fs::write(
        objects::path(&dir, objects::Domain::Entities, object),
        b"torn chest object",
    )
    .unwrap();
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.recovered_backup());
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    assert!(w.entity_page(key).unwrap().entities.is_empty());
    assert!(w.collect_garbage().is_err());
    assert!(dir.join("gc-recovery.hold").exists());
}

#[test]
fn blocked_full_expired_and_legacy_deaths_keep_all_valid_items() {
    use crate::{
        experimental::infdev::{
            entities::{Kind, Page, Position},
            survival::{DeathRule, State},
        },
        item::ItemId,
    };
    let at = Position {
        depth: -1,
        x: -65,
        y: -65,
    };
    let mut page = Page::empty(at.key());
    let mut s = State::new(42);
    s.player.inventory.add(ItemId::Wood, 731);
    s.player.health = 0;
    page.record_death(&mut s, at, false);
    assert!(s.death_retained);
    assert_eq!(s.player.inventory.count(ItemId::Wood), 731);
    assert!(page.entities.is_empty());
    s.death_recorded = false;
    s.death_retained = false;
    s.death_rule = DeathRule::KeepInventory;
    page.record_death(&mut s, at, true);
    assert!(!s.death_recorded);
    assert_eq!(s.player.inventory.count(ItemId::Wood), 731);
    s.death_rule = DeathRule::ChestNormal;
    for _ in 0..64 {
        s.death_recorded = false;
        s.player.inventory.add(ItemId::Wood, 1);
        page.record_death(&mut s, at, true);
    }
    s.death_recorded = false;
    s.player.inventory.add(ItemId::Wood, 23);
    page.record_death(&mut s, at, true);
    assert!(s.death_retained);
    assert_eq!(s.player.inventory.count(ItemId::Wood), 23);
    assert_eq!(page.entities.len(), 64);
    for e in &mut page.entities {
        let Kind::DeathChest(c) = &mut e.kind else {
            panic!("expected death chest")
        };
        c.remaining = 1;
    }
    let other = Position { x: at.x - 1, ..at };
    page.tick(other, &mut s.player.inventory);
    assert_eq!(page.entities.len(), 64);
    assert!(page.entities.iter().all(|e| {
        let Kind::DeathChest(c) = &e.kind else {
            panic!("expected death chest")
        };
        c.remaining == 0
    }));
    page.interact(at, &mut s.player.inventory);
    assert!(page.entities.is_empty());
    assert_eq!(s.player.inventory.count(ItemId::Wood), 818);
    // 732 in first chest + 63 other chests + 23 retained on player.
}

#[test]
fn entity_pages_evict_across_signed_depth_keys_and_gc_retains_backups() {
    use crate::{
        experimental::infdev::{
            entities::{Kind, Page, Position},
            survival::State,
        },
        item::ItemId,
    };
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "EntityPages", 42, 1).unwrap();
    let mut places = Vec::new();
    for i in 0..128_i64 {
        let at = Position {
            depth: -(i % 5) as i8,
            x: if i == 0 {
                i64::MIN
            } else if i == 1 {
                i64::MAX
            } else {
                -i * 64 - 1
            },
            y: i * 64,
        };
        let key = at.key();
        let mut page = w.entity_page(key).unwrap();
        assert!(page.entities.is_empty());
        let mut s = State::new(42);
        s.player.inventory.add(ItemId::Wood, (i + 1) as u16);
        page.record_death(&mut s, at, true);
        let cached = w.entity_cache.get_mut(&key).unwrap();
        cached.page = page;
        cached.dirty = true;
        w.survival.as_mut().unwrap().entity_pages_present = true;
        places.push((at, (i + 1) as u16));
        assert_eq!(w.entity_cache.len(), 1);
    }
    w.unload_all().unwrap();
    w.collect_garbage().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    for (at, count) in places {
        let page = w.entity_page(at.key()).unwrap();
        let Kind::DeathChest(c) = &page.entities[0].kind else {
            panic!("expected death chest")
        };
        assert_eq!(c.inventory.count(ItemId::Wood), count);
        assert_eq!(page.position(&page.entities[0]), at);
    }
    assert!(
        Page::empty(Key {
            depth: 1,
            x: 0,
            y: 0
        })
        .validate()
        .is_err()
    );
    w.collect_garbage().unwrap();
}

#[test]
fn missing_entity_root_or_write_failure_cannot_silently_empty_player_inventory() {
    use crate::experimental::infdev::{entities::Kind, survival::Action};
    let root = Owned::new();
    let (mut w, owned) = death_fixture(&root, "EntityWriteFailure");
    let dir = w.directory.clone();
    let previous = fs::read(dir.join("checkpoint.json")).unwrap();
    // A controlled I/O obstruction belongs exclusively to this test directory.
    fs::write(dir.join("entities"), b"not a directory").unwrap();
    assert!(w.survival_action(Action::DiagnosticDeath).is_err());
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), previous);
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    let key = Key::at_depth(-1, -65, -65);
    let page = w.entity_page(key).unwrap();
    let Kind::DeathChest(c) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(
        c.inventory, owned,
        "RAM still has exactly one owner after save failure"
    );
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    fs::remove_file(dir.join("entities")).unwrap();
    w.survival_action(Action::DiagnosticDeath).unwrap();
    let complete = fs::read(dir.join("checkpoint.json")).unwrap();
    let mut missing: serde_json::Value = serde_json::from_slice(&complete).unwrap();
    missing.as_object_mut().unwrap().remove("entity_root");
    drop(w);
    fs::write(
        dir.join("checkpoint.json"),
        serde_json::to_vec(&missing).unwrap(),
    )
    .unwrap();
    let w = World::open(&dir, 1).unwrap();
    assert!(w.recovered_backup());
    assert_eq!(
        w.survival().unwrap().player.inventory,
        owned,
        "missing root is corruption, not an empty entity world"
    );
}

#[test]
fn gc_retains_pre_retrieval_chest_and_unknown_inventory_fields_are_refused() {
    use crate::experimental::infdev::{entities::Kind, survival::Action};
    let root = Owned::new();
    let (mut w, owned) = death_fixture(&root, "EntityBackup");
    w.survival_action(Action::DiagnosticDeath).unwrap();
    let key = Key::at_depth(-1, -65, -65);
    let chest = w.find(w.saved.entity_root, key).unwrap().unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(
        &objects::read(&w.directory, objects::Domain::Entities, chest).unwrap(),
    )
    .unwrap();
    value["entities"][0]["kind"]["DeathChest"]["inventory"]["future_owned_instance"] =
        serde_json::json!(731);
    let unknown = objects::write(
        &w.directory,
        objects::Domain::Entities,
        &serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    assert!(
        w.read_entity_object(key, unknown).is_err(),
        "nested inventory data must never be silently ignored"
    );
    w.survival_action(Action::Confirm).unwrap();
    w.visit_layer(-1, -65, -65).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    w.collect_garbage().unwrap();
    assert!(
        objects::read(&w.directory, objects::Domain::Entities, chest).is_ok(),
        "backup still owns its pre-retrieval chest object"
    );
    let current = w.find(w.saved.entity_root, key).unwrap().unwrap();
    let dir = w.directory.clone();
    drop(w);
    fs::write(
        objects::path(&dir, objects::Domain::Entities, current),
        b"corrupt empty page",
    )
    .unwrap();
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.recovered_backup());
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    let page = w.entity_page(key).unwrap();
    let Kind::DeathChest(c) = &page.entities[0].kind else {
        panic!("expected death chest")
    };
    assert_eq!(c.inventory, owned);
}
