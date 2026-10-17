//! Cross-language per-instance values plus actual arena update and native reload.
use super::*;
use crate::item::{ItemId, ItemStack};
use crate::level::{noise::JavaRandom, tile::Tile};
use std::collections::BTreeMap;

fn species(name: &str) -> mob::NaturalMob {
    mob::NaturalMob::ALL
        .into_iter()
        .find(|m| format!("{m:?}") == name)
        .expect("official species")
}

#[test]
fn java35_enemy_constructor_contact_and_drop_oracle() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/entity-state35.json"
    ))
    .unwrap();
    for sample in oracle["enemies"].as_array().unwrap() {
        let kind = species(sample["species"].as_str().unwrap());
        let level = sample["level"].as_u64().unwrap() as u8;
        let diff = sample["difficulty"].as_u64().unwrap() as u8;
        let mut arena = EntityArena::default();
        arena
            .spawn_mob_level(kind, level, diff, false, 120, 120)
            .unwrap();
        let EntityKind::Mob(mob) = &arena.entities()[0].kind else {
            panic!()
        };
        assert_eq!(
            u64::from(mob.max_health),
            sample["maxHealth"].as_u64().unwrap(),
            "{kind:?} level {level} diff {diff}"
        );
        let mut tiles = vec![Tile::Dirt; 16 * 16];
        let data = vec![0; tiles.len()];
        let damage = arena
            .tick_with_player_difficulty(
                &mut tiles,
                &data,
                16,
                16,
                Some((124, 120)),
                false,
                false,
                &mut JavaRandom::new(7),
                diff,
                -1,
                50_000,
                300,
                false,
            )
            .player_damage;
        assert_eq!(
            u64::from(damage),
            sample["contactDamage"].as_u64().unwrap(),
            "{kind:?} level {level} diff {diff}"
        );
        let mut actual = BTreeMap::new();
        for stack in stats::mob_drops_at_level(kind, level, diff, false, &mut JavaRandom::new(1234))
        {
            *actual.entry(stack.item.display_name()).or_insert(0u16) += stack.count;
        }
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            sample["drops"],
            "{kind:?} level {level} diff {diff}"
        );
    }
}

#[test]
fn java35_mob_level_health_combat_and_native_reload() {
    let mut arena = EntityArena::default();
    arena
        .import_mob_level(mob::NaturalMob::Zombie, 3, 1, false, 120, 120, 73, false)
        .unwrap();
    let mut random = JavaRandom::new(1234);
    let hit = arena.damage_nearest(120, 120, 9, &mut random).unwrap();
    assert_eq!(
        (hit.level, hit.health, hit.damage, hit.defeated),
        (3, 64, 9, false)
    );
    // Java Mob.doHurt rejects a second hit during the ten tick hurt window.
    assert_eq!(
        arena
            .damage_nearest(120, 120, 9, &mut random)
            .unwrap()
            .damage,
        0
    );
    let json = serde_json::to_string(&arena).unwrap();
    let mut loaded: EntityArena = serde_json::from_str(&json).unwrap();
    let EntityKind::Mob(mob) = &loaded.entities()[0].kind else {
        panic!()
    };
    assert_eq!(
        (mob.level, mob.health, mob.max_health, mob.hurt_time),
        (3, 64, 90, 10)
    );
    let mut tiles = vec![Tile::Dirt; 256];
    let data = vec![0; 256];
    for _ in 0..10 {
        loaded.tick_with_player_difficulty(
            &mut tiles,
            &data,
            16,
            16,
            Some((180, 180)),
            false,
            false,
            &mut random,
            1,
            -1,
            50_000,
            300,
            false,
        );
    }
    let (x, y) = (loaded.entities()[0].x, loaded.entities()[0].y);
    let hit = loaded.damage_nearest(x, y, 100, &mut random).unwrap();
    assert!(hit.defeated);
    assert_eq!(hit.level, 3);
    assert_eq!(loaded.mob_count(), 0);
    assert!(loaded.entities().iter().any(|e| matches!(
        e.kind,
        EntityKind::Item(ItemStack {
            item: ItemId::Cloth,
            ..
        })
    )));
    let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mob = old["entities"][0]["kind"]["Mob"].as_object_mut().unwrap();
    mob.remove("level");
    mob.remove("difficulty");
    mob.remove("score_mode");
    mob.insert("max_health".into(), 5.into());
    mob.insert("health".into(), 3.into());
    old["entities"][0]
        .as_object_mut()
        .unwrap()
        .remove("spawner");
    old["entities"][0]
        .as_object_mut()
        .unwrap()
        .remove("statue_boss_health");
    let old: EntityArena = serde_json::from_value(old).unwrap();
    let EntityKind::Mob(mob) = &old.entities()[0].kind else {
        panic!()
    };
    assert_eq!((mob.level, mob.health, mob.max_health), (1, 3, 5));
}

#[test]
fn java35_spawner_runtime_gates_level_and_destruction() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/entity-state35.json"
    ))
    .unwrap();
    for case in oracle["spawners"].as_array().unwrap() {
        let name = case["scenario"].as_str().unwrap();
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(1234);
        arena
            .import_spawner(FurnitureKind::ZombieSpawner, 104, 120, 3, &mut random)
            .unwrap();
        arena.entities[0].spawner.as_mut().unwrap().spawn_ticks = 1;
        let mut tiles = vec![Tile::Dirt; 256];
        let data = vec![0; 256];
        if name == "torch" {
            tiles[6 + 7 * 16] = Tile::Torch;
        }
        if name == "blocked" {
            for y in 6..=8 {
                for x in 5..=7 {
                    tiles[x + y * 16] = Tile::Rock;
                }
            }
        }
        let px = if name == "far" { 400 } else { 104 };
        let depth = if name == "day" || name == "night" {
            0
        } else {
            -1
        };
        // Full mob count gate (no actual population at this location needed).
        let maximum = if name == "full" { 0 } else { 300 };
        arena.tick_with_player_difficulty(
            &mut tiles,
            &data,
            16,
            16,
            Some((px, 120)),
            false,
            false,
            &mut random,
            1,
            depth,
            if name == "day" { 20_000 } else { 50_000 },
            maximum,
            false,
        );
        let mobs: Vec<_> = arena
            .entities()
            .iter()
            .filter_map(|e| {
                if let EntityKind::Mob(m) = &e.kind {
                    Some(m)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            mobs.len(),
            case["spawned"].as_array().unwrap().len(),
            "{name}"
        );
        for m in mobs {
            assert_eq!(
                (m.species, m.level, m.max_health),
                (mob::NaturalMob::Zombie, 3, 90)
            );
        }
        assert!((200..=500).contains(&arena.entities[0].spawner.as_ref().unwrap().spawn_ticks));
        arena.damage_spawner_near(104, 120, 2, true, false, false, &mut JavaRandom::new(55));
        assert_eq!(
            u64::from(arena.entities[0].health),
            case["healthAfterHit"].as_u64().unwrap()
        );
        let before = serde_json::to_value(&arena).unwrap();
        let mut loaded: EntityArena = serde_json::from_value(before.clone()).unwrap();
        assert_eq!(serde_json::to_value(&loaded).unwrap(), before);
        assert_eq!(
            loaded.damage_spawner_near(104, 120, 2, true, true, false, &mut random),
            Some(true)
        );
        assert!(
            loaded
                .entities()
                .iter()
                .all(|e| !matches!(e.kind,EntityKind::Furniture(k)if k.spawner_mob().is_some()))
        );
        assert!(
            loaded
                .entities()
                .iter()
                .all(|e| !matches!(e.kind, EntityKind::Item(_)))
        );
    }
}

#[test]
fn java35_statue_saved_boss_health_survives_activation_and_player_absence() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/entity-state35.json"
    ))
    .unwrap();
    let expected = oracle["statue"]["bossHealthAfterAwakening"]
        .as_u64()
        .unwrap() as u16;
    let mut arena = EntityArena::default();
    arena.import_statue(104, 120, expected).unwrap();
    let mut arena: EntityArena =
        serde_json::from_str(&serde_json::to_string(&arena).unwrap()).unwrap();
    for tap in 1..=3 {
        assert_eq!(arena.tap_statue_near(104, 120), Some(tap));
    }
    assert_eq!(
        arena.active_boss(mob::NaturalMob::ObsidianKnight),
        Some((1731, 5000))
    );
    let boss = arena
        .entities()
        .iter()
        .find_map(|entity| {
            if let EntityKind::Mob(mob) = &entity.kind {
                Some(mob)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        u64::from(boss.phase),
        oracle["statue"]["phase"].as_u64().unwrap()
    );
    let mut phase_tiles = vec![Tile::Dirt; 256];
    let phase_data = vec![0; 256];
    arena.tick_with_player_difficulty(
        &mut phase_tiles,
        &phase_data,
        16,
        16,
        Some((180, 180)),
        false,
        false,
        &mut JavaRandom::new(7),
        1,
        -4,
        50_000,
        300,
        false,
    );
    let boss = arena
        .entities()
        .iter()
        .find_map(|entity| {
            if let EntityKind::Mob(mob) = &entity.kind {
                Some(mob)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        u64::from(boss.phase),
        oracle["statue"]["phaseAfterTick"].as_u64().unwrap()
    );
    assert_eq!(boss.health, 1731);
    arena.import_statue(136, 120, 5000).unwrap();
    assert_eq!(
        arena.tap_statue_near(136, 120),
        None,
        "existing live boss prevents a second activation"
    );
    let mut tiles = vec![Tile::Dirt; 256];
    let data = vec![0; 256];
    arena.tick_with_player_difficulty(
        &mut tiles,
        &data,
        16,
        16,
        None,
        false,
        false,
        &mut JavaRandom::new(7),
        1,
        -4,
        50_000,
        300,
        false,
    );
    assert!(!arena.has_mob(mob::NaturalMob::ObsidianKnight));
    assert!(
        arena
            .entities()
            .iter()
            .any(|e| e.statue_boss_health == Some(1731))
    );
    assert!(arena.import_statue(0, 0, 0).is_err());
    assert!(arena.import_statue(0, 0, 5001).is_err());
    assert!(
        arena
            .import_mob_level(mob::NaturalMob::Zombie, 5, 1, false, 0, 0, 1, false)
            .is_err()
    );
    assert!(
        arena
            .import_spawner(
                FurnitureKind::ZombieSpawner,
                0,
                0,
                0,
                &mut JavaRandom::new(1)
            )
            .is_err()
    );
}

#[test]
fn spawner_level_cannot_be_lost_through_power_glove_pickup() {
    let mut arena = EntityArena::default();
    let mut rng = JavaRandom::new(3);
    arena
        .import_spawner(FurnitureKind::ZombieSpawner, 104, 120, 3, &mut rng)
        .unwrap();
    assert!(arena.pickup_furniture_near(104, 120, 14, true).is_none());
    assert_eq!(arena.entities()[0].spawner.as_ref().unwrap().level, 3);
    // Level-one items are representable and retain the normal creative route.
    arena
        .import_spawner(FurnitureKind::SheepSpawner, 152, 120, 1, &mut rng)
        .unwrap();
    assert_eq!(
        arena.pickup_furniture_near(152, 120, 14, true),
        Some(FurnitureKind::SheepSpawner)
    );
    assert_eq!(arena.entities().len(), 1);
}

#[test]
fn native_entity_instance_validation_rejects_corruption_without_rescaling_old_health() {
    let mut arena = EntityArena::default();
    let mut rng = JavaRandom::new(3);
    arena.spawn_mob(mob::NaturalMob::Zombie, 104, 120);
    arena
        .import_spawner(FurnitureKind::ZombieSpawner, 136, 120, 3, &mut rng)
        .unwrap();
    arena.import_statue(168, 120, 1731).unwrap();
    assert!(arena.validate_states().is_ok());
    let source = serde_json::to_value(&arena).unwrap();
    for (entity_index, key, value) in [
        (0, "level", serde_json::json!(0)),
        (0, "level", serde_json::json!(5)),
        (0, "difficulty", serde_json::json!(3)),
        (0, "health", serde_json::json!(6)),
        (0, "max_health", serde_json::json!(0)),
    ] {
        let mut invalid = source.clone();
        invalid["entities"][entity_index]["kind"]["Mob"][key] = value;
        let invalid: EntityArena = serde_json::from_value(invalid).unwrap();
        assert!(invalid.validate_states().is_err(), "{key}");
    }
    for (entity_index, key, value) in [
        (
            1,
            "spawner",
            serde_json::json!({"level":5,"spawn_ticks":200}),
        ),
        (
            1,
            "spawner",
            serde_json::json!({"level":3,"spawn_ticks":501}),
        ),
        (2, "statue_boss_health", serde_json::json!(0)),
        (2, "statue_boss_health", serde_json::json!(5001)),
        (0, "statue_boss_health", serde_json::json!(1731)),
    ] {
        let mut invalid = source.clone();
        invalid["entities"][entity_index][key] = value;
        let invalid: EntityArena = serde_json::from_value(invalid).unwrap();
        assert!(invalid.validate_states().is_err(), "{key}");
    }
    let EntityKind::Mob(mob) = &arena.entities()[0].kind else {
        panic!()
    };
    assert_eq!(
        (mob.level, mob.max_health),
        (1, 5),
        "historical Rust max HP is not silently upgraded"
    );
}

#[test]
fn spawner_runtime_preserves_score_mode_for_levelled_slime_drops() {
    let mut arena = EntityArena::default();
    let mut random = JavaRandom::new(13);
    arena
        .import_spawner(FurnitureKind::SlimeSpawner, 104, 120, 4, &mut random)
        .unwrap();
    arena.entities[0].spawner.as_mut().unwrap().spawn_ticks = 1;
    let mut tiles = vec![Tile::Dirt; 256];
    let data = vec![0; 256];
    arena.tick_with_player_difficulty(
        &mut tiles,
        &data,
        16,
        16,
        Some((104, 120)),
        false,
        false,
        &mut random,
        1,
        -3,
        50_000,
        300,
        true,
    );
    let m = arena
        .entities()
        .iter()
        .find_map(|entity| {
            if let EntityKind::Mob(mob) = &entity.kind {
                Some(mob)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!((m.level, m.max_health, m.score_mode), (4, 32, true));
    let drops =
        stats::mob_drops_at_level(m.species, m.level, m.difficulty, m.score_mode, &mut random);
    assert!((1..=2).contains(&drops[0].count));
}

#[test]
fn java35_passive_difficulty_health_preserves_valid_saved_current_health() {
    // Cow.java -> PassiveMob(sprites, 5); Pig/Sheep -> default factor 3.
    for (species, expected) in [
        (mob::NaturalMob::Cow, [5, 10, 15]),
        (mob::NaturalMob::Pig, [5, 8, 11]),
        (mob::NaturalMob::Sheep, [5, 8, 11]),
    ] {
        for (difficulty, health) in expected.into_iter().enumerate() {
            assert_eq!(
                stats::mob_health_at_level(species, 1, difficulty as u8),
                Some(health)
            );
            let mut arena = EntityArena::default();
            arena
                .import_mob_level(species, 1, difficulty as u8, false, 104, 120, health, false)
                .unwrap();
            let arena: EntityArena =
                serde_json::from_str(&serde_json::to_string(&arena).unwrap()).unwrap();
            let EntityKind::Mob(mob) = &arena.entities()[0].kind else {
                panic!()
            };
            assert_eq!((mob.level, mob.health, mob.max_health), (1, health, health));
            assert!(arena.validate_states().is_ok());
        }
    }
    // The old Rust constructor and already-persisted max HP do not change.
    let mut historical = EntityArena::default();
    historical.spawn_mob(mob::NaturalMob::Cow, 104, 120);
    let EntityKind::Mob(cow) = &historical.entities()[0].kind else {
        panic!()
    };
    assert_eq!((cow.health, cow.max_health), (5, 5));
    assert!(historical.validate_states().is_ok());
}

#[test]
fn java35_passive_constructor_and_contact_match_official_runtime_oracle() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/entity-state35.json"
    ))
    .unwrap();
    let samples = oracle["passiveMobs"].as_array().unwrap();
    assert_eq!(samples.len(), 9);
    for sample in samples {
        let species = species(sample["species"].as_str().unwrap());
        let difficulty = sample["difficulty"].as_u64().unwrap() as u8;
        let mut arena = EntityArena::default();
        arena
            .spawn_mob_level(species, 1, difficulty, false, 120, 120)
            .unwrap();
        let EntityKind::Mob(mob) = &arena.entities()[0].kind else {
            panic!()
        };
        assert_eq!(
            u64::from(mob.health),
            sample["initialHealth"].as_u64().unwrap(),
            "{species:?} diff {difficulty}"
        );
        assert_eq!(
            u64::from(mob.max_health),
            sample["maxHealth"].as_u64().unwrap(),
            "{species:?} diff {difficulty}"
        );
        let mut tiles = vec![Tile::Dirt; 256];
        let data = vec![0; 256];
        let contact = arena
            .tick_with_player_difficulty(
                &mut tiles,
                &data,
                16,
                16,
                Some((124, 120)),
                false,
                false,
                &mut JavaRandom::new(7),
                difficulty,
                0,
                50_000,
                300,
                false,
            )
            .player_damage;
        assert_eq!(
            u64::from(contact),
            sample["contactDamage"].as_u64().unwrap()
        );
        assert!(arena.validate_states().is_ok());
    }
}
