use super::*;
use std::io::{Cursor, Read};

pub(crate) struct FixtureRoot(pub PathBuf);
impl Drop for FixtureRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
pub(crate) fn fixture(bytes: &[u8]) -> FixtureRoot {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parity/java230")
        .join(format!(
            "migration-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("source")).unwrap();
    fs::create_dir(root.join("saves")).unwrap();
    // Only checked-in official-Save fixtures are unpacked, never user archives.
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert!(archive.len() <= 32);
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        assert!(known_file(entry.name()));
        assert!(entry.size() <= MAX_FILE);
        let mut content = Vec::new();
        entry.read_to_end(&mut content).unwrap();
        fs::write(root.join("source").join(entry.name()), content).unwrap();
    }
    FixtureRoot(fs::canonicalize(root).unwrap())
}
const BASE: &[u8] = include_bytes!("../../../tests/fixtures/java230/migration-base.zip");
pub(crate) const COLORS: &[u8] =
    include_bytes!("../../../tests/fixtures/java230/migration-colors.zip");

#[test]
fn java230_migration_official_save_to_native_is_read_only_atomic_and_repeatable() {
    for bytes in [BASE, COLORS] {
        let root = fixture(bytes);
        let source = root.0.join("source");
        let target = root.0.join("saves/Imported");
        let before = snapshot(&fs::canonicalize(&source).unwrap()).unwrap();
        let unlocks = root.0.join("Unlocks.json");
        fs::write(
            &unlocks,
            include_str!("../../../tests/fixtures/java230/migration-unlocks.json"),
        )
        .unwrap();
        import_to(&source, &target, Some(&unlocks)).unwrap();
        assert_eq!(
            snapshot(&fs::canonicalize(&source).unwrap()).unwrap(),
            before
        );
        assert!(!source.join("state.json").exists());
        let records = crate::worlds::load(&root.0).unwrap();
        assert_eq!(records.len(), 1);
        let world = crate::worlds::load_state(&records[0]).unwrap().unwrap();
        assert_eq!(world.java_level_seeds, [42, 43, 44, 45, 46, 47]);
        assert_eq!(
            world.levels.iter().map(|l| l.depth).collect::<Vec<_>>(),
            [-3, -2, -1, 0, 1, -4]
        );
        assert!(world.achievement_unlocked("minicraft.achievement.planks"));
        assert_eq!(
            world.java_global_unlocks,
            Some(
                serde_json::from_str(include_str!(
                    "../../../tests/fixtures/java230/migration-unlocks.json"
                ))
                .unwrap()
            )
        );
        assert_eq!(world.respawn_tile, Some((6, 7)));
        assert_eq!(world.clock.get().day_tick, 48600);
        crate::worlds::save_state(&records[0], &world).unwrap();
        let restored = crate::worlds::load_state(&records[0]).unwrap().unwrap();
        for (a, b) in world.levels.iter().zip(&restored.levels) {
            assert_eq!(a.tiles, b.tiles);
            assert_eq!(a.data, b.data);
        }
        let saved = fs::read(target.join("state.json")).unwrap();
        assert!(
            import_to(&source, &target, None)
                .unwrap_err()
                .contains("already exists")
        );
        assert_eq!(fs::read(target.join("state.json")).unwrap(), saved);
        // A lost imported state must never revert to a seed-only world.
        fs::remove_file(target.join("state.json")).unwrap();
        fs::remove_file(target.join("state.json.bak")).unwrap();
        assert!(
            crate::worlds::load_state(&records[0])
                .err()
                .unwrap()
                .contains("refusing to regenerate")
        );
    }
}

#[test]
fn java230_migration_bad_inputs_leave_no_world_and_preserve_source() {
    let mutations: [(&str, &str, &str); 12] = [
        ("Game.miniplussave", "2.3.0-dev2", "2.3.0-infdev3"),
        ("Level0.miniplussave", "ROCK", "999"),
        ("Level0.miniplussave", "128,128,", "999,128,"),
        ("Level0.miniplussave", ",42,-3,", ",42,-2,"),
        ("Player.miniplussave", "104,120,", "999999,120,"),
        (
            "Inventory.miniplussave",
            "Iron Pickaxe_17",
            "Iron Pickaxe_bad",
        ),
        (
            "Inventory.miniplussave",
            "Iron Pickaxe_17",
            "Unknown Item_2",
        ),
        (
            "Entities.miniplussave",
            "Cyan Wool_4",
            "Unknown Chest Item_4",
        ),
        (
            "Entities.miniplussave",
            "Cow[248:136:2:3]",
            "Boat[248:136:2:3]",
        ),
        (
            "Entities.miniplussave",
            "Cow[248:136:2:3]",
            "Zombie[248:136:2:3:3]",
        ),
        (
            "Entities.miniplussave",
            "Sheep[232:136:4:true:3]",
            "Sheep[232:136:4:invalid:3]",
        ),
        (
            "Entities.miniplussave",
            "Cow[248:136:2:3]",
            "Cow[248:136:2:99]",
        ),
    ];
    for (file, old, new) in mutations {
        let root = fixture(COLORS);
        let source = root.0.join("source");
        let path = source.join(file);
        let original = fs::read_to_string(&path).unwrap();
        assert!(original.contains(old), "fixture token {file} {old}");
        fs::write(&path, original.replacen(old, new, 1)).unwrap();
        let before = snapshot(&fs::canonicalize(&source).unwrap());
        let target = root.0.join("saves/Rejected");
        assert!(
            import_to(&source, &target, None).is_err(),
            "accepted {file} {old}->{new}"
        );
        assert!(!target.exists());
        if let Ok(before) = before {
            assert_eq!(
                snapshot(&fs::canonicalize(&source).unwrap()).unwrap(),
                before
            );
        }
        assert_eq!(fs::read_dir(root.0.join("saves")).unwrap().count(), 0);
    }
    let root = fixture(include_bytes!(
        "../../../tests/fixtures/java230/migration-instances.zip"
    ));
    let error =
        import_to(&root.0.join("source"), &root.0.join("saves/Rejected"), None).unwrap_err();
    assert!(error.contains("Watering Can instance"), "{error}");
}

#[test]
fn java230_migration_truncation_missing_layers_limits_and_paths_are_rejected() {
    for case in 0..6 {
        let root = fixture(BASE);
        let source = root.0.join("source");
        let target = root.0.join("saves/Rejected");
        match case {
            0 => {
                fs::write(source.join("Level2data.miniplussave"), "0,1,").unwrap();
            }
            1 => {
                fs::remove_file(source.join("Level5.miniplussave")).unwrap();
            }
            2 => {
                fs::File::create(source.join("Inventory.miniplussave"))
                    .unwrap()
                    .set_len(MAX_FILE + 1)
                    .unwrap();
            }
            3 => {
                fs::create_dir(source.join("nested")).unwrap();
            }
            4 => {
                fs::write(source.join("future-fields.json"), "{}").unwrap();
            }
            _ => {
                fs::write(source.join("Player.miniplussave"), "104,120,{broken").unwrap();
            }
        }
        assert!(import_to(&source, &target, None).is_err());
        assert!(!target.exists());
    }
    let root = fixture(BASE);
    let source = root.0.join("source");
    assert!(
        import_to(&source, &source.join("inside"), None)
            .unwrap_err()
            .contains("inside")
    );
    assert!(
        import_to(
            &source,
            &PathBuf::from(format!("{}/saves/../escape", root.0.display())),
            None
        )
        .unwrap_err()
        .contains("traversal")
    );
    let archive = root.0.join("not-a-world.zip");
    fs::write(&archive, BASE).unwrap();
    assert!(import_to(&archive, &root.0.join("saves/Archive"), None).is_err());
}

#[test]
fn java230_migration_interruption_and_destination_race_never_replace_a_world() {
    let root = fixture(BASE);
    let source = root.0.join("source");
    let target = root.0.join("saves/Imported");
    let before = snapshot(&fs::canonicalize(&source).unwrap()).unwrap();
    assert!(
        import_with_hook(&source, &target, None, || Err(
            "simulated write interruption before publication".into()
        ))
        .is_err()
    );
    assert!(!target.exists());
    assert_eq!(fs::read_dir(root.0.join("saves")).unwrap().count(), 0);
    assert!(
        import_with_hook(&source, &target, None, || {
            fs::create_dir(&target).unwrap();
            Ok(())
        })
        .is_err()
    );
    assert!(target.is_dir());
    assert_eq!(
        fs::read_dir(&target).unwrap().count(),
        0,
        "even an empty foreign directory is protected"
    );
    fs::remove_dir(&target).unwrap();
    // A hard-killed process can leave an unlisted isolated stage. Retry uses a
    // fresh exclusive stage and does not overwrite/delete the orphan.
    let orphan = root.0.join("saves/.java-import-interrupted/native");
    fs::create_dir_all(&orphan).unwrap();
    fs::write(orphan.join("state.json"), "partial").unwrap();
    assert!(crate::worlds::load(&root.0).unwrap().is_empty());
    import_to(&source, &target, None).unwrap();
    assert_eq!(
        fs::read_to_string(orphan.join("state.json")).unwrap(),
        "partial"
    );
    assert_eq!(
        snapshot(&fs::canonicalize(&source).unwrap()).unwrap(),
        before
    );
    assert_eq!(crate::worlds::load(&root.0).unwrap().len(), 1);
}
