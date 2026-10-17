//! Official Save instances enter real World actions, then native disk resume.
use super::*;
use crate::saveload::{migration, migration::tests::fixture};

#[test]
fn official_saved_locked_chests_reward_only_the_last_unlock_across_disk_resume() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/dungeon-chest-reward35.json"
    ))
    .unwrap();
    let root = fixture(crate::saveload::migration::tests::COLORS);
    let source = root.0.join("source");
    let path = source.join("Entities.miniplussave");
    let original = std::fs::read_to_string(&path).unwrap();
    // A controlled fixture variation using two actual official Save.writeEntity
    // strings, subsequently official Load/use-tested in the behavior oracle.
    // Retain every other entity; this is not a new natural-world source claim.
    let replacement = original
        .replace(
            "DungeonChest[280:136:Gem_2:true:3]",
            oracle["officialSavedEntities"][0].as_str().unwrap(),
        )
        .replace(
            "DungeonChest[312:136:Gold_3:false:3]",
            oracle["officialSavedEntities"][1].as_str().unwrap(),
        );
    assert_ne!(replacement, original);
    std::fs::write(&path, &replacement).unwrap();
    migration::import_to(&source, &root.0.join("saves/Chests"), None).unwrap();
    let record = crate::worlds::load(&root.0).unwrap().pop().unwrap();
    let mut world = crate::worlds::load_state(&record).unwrap().unwrap();
    world.current_level = 5;
    world.player.direction = Direction::Down;
    world.player.inventory.remove(ItemId::Key, 1);
    for (index, case) in oracle["cases"].as_array().unwrap().iter().enumerate() {
        if index == 1 {
            assert_eq!(world.player.inventory.add(ItemId::Key, 2), 0);
        }
        world.player.x = if index < 3 { 40 } else { 104 };
        world.player.y = 44;
        assert!(world.use_furniture_front());
        assert_eq!(world.inventory_open, case["used"].as_bool().unwrap());
        assert_eq!(
            u64::from(world.player.inventory.count(ItemId::Key)),
            case["keys"].as_u64().unwrap()
        );
        let arena = &world.levels[5].entities;
        let apples: u16 = arena
            .entities()
            .iter()
            .filter_map(|e| match e.kind {
                EntityKind::Item(s) if s.item == ItemId::GoldenApple => Some(s.count),
                _ => None,
            })
            .sum();
        assert_eq!(u64::from(apples), case["goldApples"].as_u64().unwrap());
        let locked = arena
            .entities()
            .iter()
            .filter(|e| {
                matches!(e.kind, EntityKind::Furniture(FurnitureKind::DungeonChest)) && e.state == 0
            })
            .count();
        assert_eq!(locked as u64, case["chestCount"].as_u64().unwrap());
        world.close_container();
        crate::worlds::save_state(&record, &world).unwrap();
        world = crate::worlds::load_state(&record).unwrap().unwrap();
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), replacement);
}

#[test]
fn java35_imported_combat_spawner_and_statue_enter_world_runtime() {
    let root = fixture(include_bytes!(
        "../../tests/fixtures/java230/migration-entities35.zip"
    ));
    let source = root.0.join("source");
    let original = std::fs::read(source.join("Entities.miniplussave")).unwrap();
    migration::import_to(&source, &root.0.join("saves/Runtime"), None).unwrap();
    let record = crate::worlds::load(&root.0).unwrap().pop().unwrap();
    let mut world = crate::worlds::load_state(&record).unwrap().unwrap();

    // Take the official saved chest tool (Iron Pickaxe11), without adding gear.
    let chest = world.levels[3]
        .entities
        .entities()
        .iter()
        .find(|e| e.x == 168 && e.y == 136)
        .unwrap();
    let chest_id = chest.id;
    let tool_row = chest.storage.as_ref().unwrap().slots().len();
    assert!(world.transfer_chest_to_player(chest_id, tool_row, true));
    assert_eq!(world.player.inventory.tools()[0].durability, 11);

    // This actual saved Zombie is level3/90HP. No test replacement mob.
    world.current_level = 3;
    world.player.x = 252;
    world.player.y = 136;
    world.player.direction = Direction::Right;
    world.player.active_item = Some(ActiveItem::Tool(0));
    let zombie_id = world.levels[3]
        .entities
        .entities()
        .iter()
        .find(|e| e.x == 264 && e.y == 136 && matches!(e.kind, EntityKind::Mob(_)))
        .unwrap()
        .id;
    world.attack();
    let zombie = world.levels[3]
        .entities
        .entities()
        .iter()
        .find_map(|e| {
            if e.id == zombie_id
                && let EntityKind::Mob(m) = &e.kind
            {
                Some(m)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!((zombie.level, zombie.max_health), (3, 90));
    assert!((1..90).contains(&zombie.health));
    let remaining = zombie.health;
    assert_eq!(world.player.inventory.tools()[0].durability, 10);

    // The genuine Sheep Spawner is on passable saved grass. Set its timer to
    // the next attempt boundary; do not alter source terrain or spawn a stand-in.
    let spawner = world.levels[3]
        .entities
        .entities
        .iter_mut()
        .find(|e| e.x == 344 && e.y == 248 && e.spawner.is_some())
        .unwrap();
    let spawner_id = spawner.id;
    spawner.spawner.as_mut().unwrap().spawn_ticks = 1;
    let sheep_before = world.levels[3]
        .entities
        .entities()
        .iter()
        .filter(|e| matches!(&e.kind, EntityKind::Mob(m) if m.species == spawn::NaturalMob::Sheep))
        .count();
    world.player.x = 332;
    world.player.y = 248;
    world.tick_level_entities(Some((332, 248)), false, false);
    let sheep_after = world.levels[3]
        .entities
        .entities()
        .iter()
        .filter(|e| matches!(&e.kind, EntityKind::Mob(m) if m.species == spawn::NaturalMob::Sheep))
        .count();
    assert_eq!(sheep_after, sheep_before + 1);
    let spawner = world.levels[3]
        .entities
        .entities()
        .iter()
        .find(|e| e.id == spawner_id)
        .unwrap();
    assert!((200..=500).contains(&spawner.spawner.as_ref().unwrap().spawn_ticks));
    let health_before = spawner.health;
    let durability = world.player.inventory.tools()[0].durability;
    world.attack();
    assert!(
        world.levels[3]
            .entities
            .entities()
            .iter()
            .find(|e| e.id == spawner_id)
            .unwrap()
            .health
            < health_before
    );
    assert_eq!(
        world.player.inventory.tools()[0].durability,
        durability,
        "official Spawner interaction pays no tool durability"
    );

    // Persist two actual taps, then use the restored statue a third time.
    world.current_level = 5;
    world.player.x = 388;
    world.player.y = 400;
    world.player.active_item = None;
    world.use_target();
    world.use_target();
    assert!(
        world.levels[5]
            .entities
            .entities()
            .iter()
            .any(|e| e.statue_boss_health == Some(1731) && e.state == 2)
    );
    crate::worlds::save_state(&record, &world).unwrap();
    let mut world = crate::worlds::load_state(&record).unwrap().unwrap();
    let zombie = world.levels[3]
        .entities
        .entities()
        .iter()
        .find_map(|e| {
            if e.id == zombie_id
                && let EntityKind::Mob(m) = &e.kind
            {
                Some(m)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!((zombie.level, zombie.health), (3, remaining));
    world.use_target();
    assert_eq!(
        world.levels[5]
            .entities
            .active_boss(spawn::NaturalMob::ObsidianKnight),
        Some((1731, 5000))
    );
    assert!(
        !world.levels[5]
            .entities
            .entities()
            .iter()
            .any(|e| e.statue_boss_health.is_some())
    );
    world.tick_level_entities(Some((388, 400)), false, false);
    crate::worlds::save_state(&record, &world).unwrap();
    let world = crate::worlds::load_state(&record).unwrap().unwrap();
    assert_eq!(
        world.levels[5]
            .entities
            .active_boss(spawn::NaturalMob::ObsidianKnight),
        Some((1731, 5000))
    );
    assert_eq!(
        std::fs::read(source.join("Entities.miniplussave")).unwrap(),
        original
    );
}
