//! Official Java Save -> transactional importer -> live World operations -> disk.
use super::*;
use crate::item::dye::DyeColor;
use crate::saveload::{
    migration,
    migration::tests::{COLORS, fixture},
};

fn collect(world: &mut World, x: i32, y: i32) {
    for e in &mut world.levels[world.current_level].entities.entities {
        e.age = e.age.max(31);
    }
    world.levels[world.current_level]
        .entities
        .collect_near(x, y, &mut world.player.inventory);
    world.update_progress(ProgressEvent::InventoryChanged);
}
fn canonical(name: &str) -> String {
    let key = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    match key.as_str() {
        "flower230" => "flower",
        "obsidian" => "obsidianfloor",
        "cloudcactus" => "cloudore",
        "lapis" => "lapisore",
        "woodplanks" => "woodfloor",
        "stonebricks" => "stonefloor",
        "stone" => "rawstone",
        _ => &key,
    }
    .to_owned()
}

#[test]
fn java230_migration_all_saved_cells_player_progress_actions_and_disk_resume() {
    let root = fixture(COLORS);
    let source = root.0.join("source");
    migration::import_to(&source, &root.0.join("saves/Imported"), None).unwrap();
    let record = crate::worlds::load(&root.0).unwrap().pop().unwrap();
    let mut world = crate::worlds::load_state(&record).unwrap().unwrap();
    // Exhaustive column-major Java file vs native row-major map: no tile count
    // or approximate hash can hide a transpose or an underlay-ID collision.
    for index in 0..6 {
        let names =
            std::fs::read_to_string(source.join(format!("Level{index}.miniplussave"))).unwrap();
        let data =
            std::fs::read_to_string(source.join(format!("Level{index}data.miniplussave"))).unwrap();
        let names: Vec<_> = names.trim_end_matches(',').split(',').collect();
        let data: Vec<_> = data.trim_end_matches(',').split(',').collect();
        for x in 0..128 {
            for y in 0..128 {
                let saved = y + x * 128;
                let native = x + y * 128;
                let tile = world.levels[index].tiles[native];
                let value = world.levels[index].data[native];
                assert_eq!(
                    canonical(names[saved + 4]),
                    canonical(&format!("{tile:?}")),
                    "layer={index}, x={x},y={y}"
                );
                let expected = data[saved].parse::<u16>().unwrap();
                if tile == Tile::Torch {
                    assert_eq!(expected, 62);
                    assert_eq!(value, u16::from(Tile::CyanWool.id()));
                } else {
                    assert_eq!(value, expected, "layer={index}, x={x},y={y}");
                }
            }
        }
    }
    assert_eq!((world.player.x, world.player.y), (104, 120));
    assert_eq!(world.current_level, 3);
    assert_eq!(world.player.health, 7);
    assert_eq!(world.player.max_health, 15);
    assert_eq!(world.player.hunger, 6);
    assert_eq!(world.player.imported_shirt_color, Some(16777420));
    assert_eq!(world.player.inventory.count(ItemId::Wood), 1008);
    assert_eq!(world.player.inventory.count(ItemId::CyanWool), 4);
    assert_eq!(world.player.inventory.tools().len(), 1);
    assert_eq!(world.player.inventory.tools()[0].durability, 17);
    assert_eq!(world.player.active_item, None);
    assert_eq!(
        world.progress.current_tutorial().unwrap().id,
        "minicraft.tutorial.getting_rocks"
    );
    assert_eq!(world.progress.tutorial_completed_count(), 1);
    assert_eq!(world.signs[3][&(13 + 8 * 128)], "Java230\nkeep\nall\nlines");
    assert_eq!(world.clock.get().game_time, 123456);
    let native: serde_json::Value = serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    assert_eq!(
        native["world"]["progress"]["java_advancements"]["Version"],
        "2.3.0-dev2"
    );
    assert!(
        native["world"]["progress"]["recipe_completed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "java230:minicraft.advancements.recipes.red_dye_from_rose")
    );
    let java: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/migration-operations.json"
    ))
    .unwrap();
    assert_eq!(java["activeRestored"], false);

    let arena = &world.levels[3].entities;
    let death = arena
        .entities()
        .iter()
        .find(|e| e.death_chest_lifetime.is_some())
        .unwrap();
    assert_eq!(death.death_chest_lifetime, Some(123));
    assert_eq!(death.storage.as_ref().unwrap().count(ItemId::WhiteLily), 2);
    assert_eq!(death.storage.as_ref().unwrap().tools()[0].durability, 7);
    assert!(arena.entities().iter().any(|e|matches!(&e.kind,entity::EntityKind::Mob(m) if m.species==spawn::NaturalMob::Sheep && m.sheared && m.health==4)));
    // Harvest the actual Java Violet, then use the actual Java Dye Vat menu.
    world.player.active_item = None;
    world.player.direction = Direction::Right;
    world.player.x = 8 * 16 + 8;
    world.player.y = 8 * 16 + 8;
    world.attack();
    collect(&mut world, 9 * 16 + 8, 8 * 16 + 8);
    assert_eq!(world.player.inventory.count(ItemId::Violet), 1);
    assert_eq!(world.levels[3].tiles[9 + 8 * 128], Tile::Grass);
    world.player.x = 136;
    world.player.y = 120;
    world.player.direction = Direction::Down;
    assert!(world.use_furniture_front());
    assert_eq!(world.crafting_station, Some(FurnitureKind::DyeVat));
    world.refresh_crafting_order();
    world
        .select_crafting_product_for_parity("Purple Dye")
        .unwrap();
    world.craft_selected_recipe();
    assert_eq!(world.player.inventory.count(ItemId::Violet), 0);
    assert_eq!(world.player.inventory.count(ItemId::PurpleDye), 1);
    // Actual saved stair pair and depth mapping, without generating any layer.
    world.player.x = 14 * 16 + 8;
    world.player.y = 8 * 16 + 8;
    world.apply_level_change(1);
    assert_eq!(world.levels[world.current_level].depth, -1);
    assert_eq!(
        world.tile_at_pixel(world.player.x, world.player.y),
        Tile::StairsUp
    );
    world.apply_level_change(-1);
    assert_eq!(world.levels[world.current_level].depth, 0);
    world.player.x = 104;
    world.player.y = 120;
    world.player.direction = Direction::Down;
    assert!(world.use_furniture_front());
    assert_eq!(world.sleeping, 120);
    let bed = world.levels[3]
        .entities
        .entities()
        .iter()
        .find(|e| matches!(e.kind, entity::EntityKind::Furniture(FurnitureKind::Bed)))
        .unwrap();
    assert_eq!(
        bed.state,
        crate::entity::furniture::bed::state_for_color(DyeColor::Cyan)
    );
    world.clock.set_day_tick(crate::core::sleep::END);
    world.tick_sleep_prelude();
    assert!(!world.player_removed);
    assert_eq!(world.respawn_tile, Some((6, 7)));
    // One real entity tick consumes one of the imported remaining 123 ticks.
    let remaining_before = world.levels[3]
        .entities
        .entities()
        .iter()
        .find(|e| e.death_chest_lifetime.is_some())
        .map(|e| e.death_chest_lifetime.unwrap() - e.age)
        .unwrap();
    assert!(
        remaining_before > 0 && remaining_before < 123,
        "sleep must advance the imported chest timer"
    );
    world.tick_level_entities(None, false, true);
    let death = world.levels[3]
        .entities
        .entities()
        .iter()
        .find(|e| e.death_chest_lifetime.is_some())
        .unwrap();
    assert_eq!(
        death.death_chest_lifetime.unwrap() - death.age,
        remaining_before - 1
    );
    world.player.x = 200;
    world.player.y = 136;
    world.retrieve_death_chests();
    assert_eq!(world.player.inventory.count(ItemId::WhiteLily), 4);
    assert_eq!(
        world
            .player
            .inventory
            .tools()
            .iter()
            .filter(|t| t.kind == ToolKind::Sword && t.durability == 7)
            .count(),
        1
    );
    let before =
        serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap()).unwrap();
    crate::worlds::save_state(&record, &world).unwrap();
    drop(world);
    let restored = crate::worlds::load_state(&record).unwrap().unwrap();
    let mut before = before;
    let mut after =
        serde_json::from_str::<serde_json::Value>(&restored.to_save_string().unwrap()).unwrap();
    for value in [&mut before, &mut after] {
        // HashSet iteration order is not game progress. Preserve strict array
        // order for maps, inventory, entities, recipe lists and every other field.
        for key in [
            "tutorial_completed",
            "quest_completed",
            "group_unlocked",
            "quest_unlocked",
            "criterion_hits",
            "achievements",
            "unlocking_hits",
            "recipe_completed",
        ] {
            value["world"]["progress"][key]
                .as_array_mut()
                .unwrap()
                .sort_by_key(serde_json::Value::to_string);
        }
    }
    fn first_diff(a: &serde_json::Value, b: &serde_json::Value, path: String) -> Option<String> {
        if a == b {
            return None;
        }
        match (a, b) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                for (key, value) in a {
                    if let Some(diff) = first_diff(value, &b[key], format!("{path}/{key}")) {
                        return Some(diff);
                    }
                }
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) if a.len() == b.len() => {
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    if let Some(diff) = first_diff(a, b, format!("{path}/{i}")) {
                        return Some(diff);
                    }
                }
            }
            _ => {}
        }
        Some(path)
    }
    assert!(
        before == after,
        "first persisted-state difference: {:?}",
        first_diff(&before, &after, String::new())
    );
    assert_eq!(restored.levels[3].tiles[9 + 8 * 128], Tile::Grass);
    assert_eq!(restored.player.inventory.count(ItemId::PurpleDye), 1);
}
