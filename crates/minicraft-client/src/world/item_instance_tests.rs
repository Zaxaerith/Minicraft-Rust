use super::registry_tests::tiny_world;
use super::*;
fn water(world: &World) -> Vec<Option<u16>> {
    world
        .player
        .inventory
        .slots()
        .iter()
        .filter(|s| s.item == ItemId::WateringCan)
        .map(|s| s.watering_content)
        .collect()
}
fn select(world: &mut World, index: usize) {
    world.player.active_item = Some(ActiveItem::Slot {
        index,
        revision: world.player.inventory.slot_revision(),
    });
}
fn pair() -> World {
    let mut w = tiny_world();
    w.player.direction = Direction::Right;
    w.player.inventory.add_stack(ItemStack::with_water(731));
    w.player.inventory.add_stack(ItemStack::with_water(23));
    w
}
fn restore(w: &World) -> World {
    World::from_save_string(&w.to_save_string().unwrap()).unwrap()
}
#[test]
fn independent_water_cans_use_refill_render_and_native_save() {
    let mut w = pair();
    let target = 9 + 8 * 16;
    w.levels[0].tiles[target] = Tile::Wheat;
    select(&mut w, 0);
    w.attack();
    assert_eq!(water(&w), vec![Some(730), Some(23)]);
    select(&mut w, 1);
    w.attack();
    assert_eq!(water(&w), vec![Some(730), Some(22)]);
    w.levels[0].tiles[target] = Tile::Water;
    w.attack();
    assert_eq!(water(&w), vec![Some(730), Some(1800)]);
    let w = restore(&w);
    assert_eq!(water(&w), vec![Some(730), Some(1800)]);
    let assets = Assets::load(&[]).unwrap();
    assert_ne!(
        assets.item_stack(ItemStack::with_water(1)),
        assets.item_stack(ItemStack::with_water(0))
    );
}
#[test]
fn water_cans_container_tools_full_bag_and_exact_transfer() {
    let mut w = pair();
    let mut tool = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
    tool.durability = 17;
    w.player.inventory.add_tool(tool).unwrap();
    let id = w.levels[0]
        .entities
        .spawn_furniture(FurnitureKind::Chest, 152, 136);
    assert!(w.transfer_player_to_chest(id, 1, true));
    assert_eq!(water(&w), vec![Some(731)]);
    assert_eq!(
        w.levels[0]
            .entities
            .death_chest_storage(id)
            .unwrap()
            .slots()[0]
            .watering_content,
        Some(23)
    );
    assert!(w.transfer_player_to_chest(id, w.player.inventory.slots().len(), true));
    let mut w = restore(&w);
    w.player.inventory.set_capacity(1);
    assert!(!w.transfer_chest_to_player(id, 0, true));
    assert!(!w.transfer_chest_to_player(id, 1, true));
    assert_eq!(
        w.levels[0]
            .entities
            .death_chest_storage(id)
            .unwrap()
            .tools()[0],
        tool
    );
    w.player.inventory.set_capacity(27);
    assert!(w.transfer_chest_to_player(id, 0, true));
    assert_eq!(water(&w), vec![Some(731), Some(23)]);
    assert!(w.transfer_chest_to_player(id, 0, true));
    assert_eq!(w.player.inventory.tools(), &[tool]);
    assert_eq!(
        w.levels[0]
            .entities
            .death_chest_storage(id)
            .unwrap()
            .used_slots(),
        0
    );
}
#[test]
fn water_cans_death_drop_pickup_and_save_conserve_instances() {
    let mut w = pair();
    select(&mut w, 1);
    w.die_player();
    assert!(water(&w).is_empty());
    let mut w = restore(&w);
    w.retrieve_death_chests();
    assert_eq!(water(&w), vec![Some(731), Some(23)]);
    select(&mut w, 0);
    w.drop_active_item(true);
    assert_eq!(water(&w), vec![Some(23)]);
    let mut w = restore(&w);
    for e in &mut w.levels[0].entities.entities {
        e.age = e.age.max(31);
    }
    w.levels[0]
        .entities
        .collect_near(w.player.x, w.player.y, &mut w.player.inventory);
    assert_eq!(water(&w), vec![Some(23), Some(731)]);
    assert!(
        !w.levels[0]
            .entities
            .entities()
            .iter()
            .any(|e| matches!(e.kind, EntityKind::Item(_)))
    );
}

#[test]
fn quick_container_interaction_transfers_selected_can_and_full_storage_keeps_it() {
    let mut world = pair();
    let id = world.levels[0]
        .entities
        .spawn_furniture(FurnitureKind::Chest, 148, 136);
    select(&mut world, 1);
    world.tick(&Input {
        select: true,
        ..Default::default()
    });
    assert_eq!(water(&world), vec![Some(731)]);
    assert_eq!(
        world.levels[0]
            .entities
            .death_chest_storage(id)
            .unwrap()
            .slots(),
        &[ItemStack::with_water(23)]
    );
    assert_eq!(world.player.active_item, None);
    world.levels[0]
        .entities
        .entities_mut_for_storage(id)
        .unwrap()
        .set_capacity(1);
    select(&mut world, 0);
    world.tick(&Input {
        select: true,
        ..Default::default()
    });
    assert_eq!(water(&world), vec![Some(731)]);
    assert!(world.player.active_item.is_some());
    assert_eq!(
        world.levels[0]
            .entities
            .death_chest_storage(id)
            .unwrap()
            .slots(),
        &[ItemStack::with_water(23)]
    );
    let resumed = restore(&world);
    assert_eq!(water(&resumed), vec![Some(731)]);
    assert_eq!(
        resumed.levels[0]
            .entities
            .death_chest_storage(id)
            .unwrap()
            .slots(),
        &[ItemStack::with_water(23)]
    );
}
#[test]
fn legacy_shared_water_migrates_once_without_inventing_other_instances() {
    let mut w = tiny_world();
    w.player.inventory.add(ItemId::WateringCan, 1);
    let mut old: serde_json::Value = serde_json::from_str(&w.to_save_string().unwrap()).unwrap();
    old["world"]["player"]
        .as_object_mut()
        .unwrap()
        .remove("item_instance_version");
    old["world"]["player"]["watering_content"] = serde_json::json!(731);
    old["world"]["player"]["inventory"]["slots"][0]
        .as_object_mut()
        .unwrap()
        .remove("watering_content");
    old["world"]["player"]["active_item"] = serde_json::json!({"Stack":"WateringCan"});
    let mut w = World::from_save_string(&old.to_string()).unwrap();
    assert_eq!(water(&w), vec![Some(731)]);
    w.levels[0].tiles[9 + 8 * 16] = Tile::Wheat;
    w.player.direction = Direction::Right;
    w.attack();
    assert_eq!(water(&w), vec![Some(730)]);
    assert_eq!(water(&restore(&w)), vec![Some(730)]);
    old["world"]["player"]["inventory"]["slots"][0]["count"] = serde_json::json!(2);
    let mut w = World::from_save_string(&old.to_string()).unwrap();
    assert_eq!(water(&w), vec![None, None]);
    assert_eq!(w.player.watering_content, 731);
    assert_eq!(w.player.active_item, None);
    assert!(
        crate::saveload::save::inventory_fields(&w)
            .unwrap_err()
            .contains("unknown")
    );
    select(&mut w, 1);
    w.levels[0].tiles[9 + 8 * 16] = Tile::Water;
    w.player.direction = Direction::Right;
    w.attack();
    assert_eq!(water(&w), vec![None, Some(1800)]);
    assert_eq!(water(&restore(&w)), vec![None, Some(1800)]);
}
#[test]
fn slot_revision_cancels_stale_selection_and_invalid_native_state_is_rejected() {
    let mut w = tiny_world();
    w.player.inventory.add(ItemId::Wood, 1);
    w.player.inventory.add_stack(ItemStack::with_water(731));
    w.player.inventory.add_stack(ItemStack::with_water(23));
    select(&mut w, 1);
    w.player.inventory.remove(ItemId::Wood, 1);
    w.normalize_instance_selection();
    assert_eq!(w.player.active_item, None);
    let mut bad: serde_json::Value = serde_json::from_str(&w.to_save_string().unwrap()).unwrap();
    bad["world"]["player"]["inventory"]["slots"][0]["watering_content"] = serde_json::json!(1801);
    assert!(
        World::from_save_string(&bad.to_string())
            .err()
            .unwrap()
            .contains("1800")
    );
}

#[test]
fn official_java_two_can_trace_and_inventory_encoding_match_native_instances() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/watering-instances35.json"
    ))
    .unwrap();
    let mut w = pair();
    let target = 9 + 8 * 16;
    w.levels[0].tiles[target] = Tile::Wheat;
    let trace = oracle["operations"].as_array().unwrap();
    // JSON recorded by actual Java item interactions, not Rust expected output.
    for (index, case) in trace.iter().enumerate() {
        match index {
            0 => {}
            1 => {
                select(&mut w, 0);
                w.attack();
            }
            2 => {
                select(&mut w, 1);
                w.attack();
            }
            3 => {
                w.levels[0].tiles[8 + 7 * 16] = Tile::Water;
                w.player.direction = Direction::Up;
                w.attack();
            }
            _ => panic!("unexpected Java trace step"),
        }
        assert_eq!(serde_json::json!(water(&w)), case["contents"]);
        assert_eq!(
            u64::from(w.levels[0].data[target]),
            case["cropData"].as_u64().unwrap()
        );
    }
}

#[test]
fn official_java_bare_farmland_consumes_only_instance_water_and_empty_can_does_nothing() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/watering-instances35.json"
    ))
    .unwrap();
    let mut w = tiny_world();
    w.player.inventory.add_stack(ItemStack::with_water(9));
    select(&mut w, 0);
    w.player.direction = Direction::Right;
    let target = 9 + 8 * 16;
    w.levels[0].tiles[target] = Tile::Farmland;
    w.attack();
    assert_eq!(
        water(&w),
        vec![Some(
            oracle["bareFarmland"]["contentAfter"].as_u64().unwrap() as u16
        )]
    );
    assert_eq!(
        u64::from(w.levels[0].data[target]),
        oracle["bareFarmland"]["dataAfter"].as_u64().unwrap()
    );
    w.player.inventory.slot_mut(0).unwrap().watering_content = Some(0);
    w.levels[0].tiles[target] = Tile::Wheat;
    w.attack();
    assert_eq!(water(&w), vec![Some(0)]);
    assert_eq!(w.levels[0].data[target], 0);
}

#[test]
fn official_java_saved_two_cans_import_real_actions_death_and_disk_resume() {
    use crate::saveload::{migration, migration::tests::fixture};
    let root = fixture(include_bytes!(
        "../../tests/fixtures/java230/migration-instances.zip"
    ));
    let source = root.0.join("source");
    let target = root.0.join("saves/Instances");
    let inventory = std::fs::read_to_string(source.join("Inventory.miniplussave")).unwrap();
    let entities = std::fs::read_to_string(source.join("Entities.miniplussave")).unwrap();
    assert!(inventory.contains("Watering Can_731"));
    assert!(entities.contains("Watering Can_23"));
    migration::import_to(&source, &target, None).unwrap();
    let record = crate::worlds::load(&root.0).unwrap().pop().unwrap();
    let mut w = crate::worlds::load_state(&record).unwrap().unwrap();
    assert_eq!(water(&w), vec![Some(731)]);
    assert!(w.levels[3].entities.entities().iter().any(|e|matches!(&e.kind,EntityKind::Mob(m) if m.species==spawn::NaturalMob::Zombie && m.level==3)));
    let can = w
        .player
        .inventory
        .slots()
        .iter()
        .position(|s| s.item == ItemId::WateringCan)
        .unwrap();
    select(&mut w, can);
    w.player.direction = Direction::Down;
    w.attack();
    assert_eq!(water(&w), vec![Some(730)]);
    w.player.x = 168;
    w.player.y = 120;
    w.player.direction = Direction::Down;
    assert!(w.use_furniture_front());
    let chest = w.container_id.unwrap();
    let slot = w.levels[3]
        .entities
        .death_chest_storage(chest)
        .unwrap()
        .slots()
        .iter()
        .position(|s| s.watering_content == Some(23))
        .unwrap();
    w.inventory_pane = 1;
    w.inventory_selection = slot;
    w.tick_container_ui(&Input {
        select: true,
        ..Default::default()
    });
    w.close_container();
    assert_eq!(water(&w), vec![Some(730), Some(23)]);
    let can = w
        .player
        .inventory
        .slots()
        .iter()
        .position(|s| s.watering_content == Some(23))
        .unwrap();
    select(&mut w, can);
    let wet = w.levels[3]
        .tiles
        .iter()
        .position(|t| *t == Tile::Water)
        .unwrap();
    w.attack_target_tile((wet % w.width) as i32, (wet / w.width) as i32, wet);
    assert_eq!(water(&w), vec![Some(730), Some(1800)]);
    assert!(w.transfer_player_to_chest(chest, can, true));
    let slot = w.levels[3]
        .entities
        .death_chest_storage(chest)
        .unwrap()
        .slots()
        .iter()
        .position(|s| s.watering_content == Some(1800))
        .unwrap();
    assert!(w.transfer_chest_to_player(chest, slot, true));
    assert_eq!(water(&w), vec![Some(730), Some(1800)]);
    w.player.x = 104;
    w.player.y = 120;
    w.die_player();
    assert!(water(&w).is_empty());
    crate::worlds::save_state(&record, &w).unwrap();
    let mut w = crate::worlds::load_state(&record).unwrap().unwrap();
    w.retrieve_death_chests();
    assert_eq!(water(&w), vec![Some(730), Some(1800)]);
    crate::worlds::save_state(&record, &w).unwrap();
    let w = crate::worlds::load_state(&record).unwrap().unwrap();
    assert_eq!(water(&w), vec![Some(730), Some(1800)]);
    assert_eq!(
        std::fs::read_to_string(source.join("Inventory.miniplussave")).unwrap(),
        inventory
    );
    assert_eq!(
        std::fs::read_to_string(source.join("Entities.miniplussave")).unwrap(),
        entities
    );
}

#[test]
fn legacy_ground_can_split_preserves_motion_age_and_unknown_contents() {
    let mut world = tiny_world();
    let original_id = world.levels[0]
        .entities
        .spawn_item(ItemStack::with_water(731), 96, 64);
    let entity = world.levels[0].entities.entities.last_mut().unwrap();
    entity.age = 117;
    entity.x = 97;
    entity.y = 65;
    entity.item_motion = Some(crate::entity::ItemMotion {
        precise_x: 97.25,
        precise_y: 65.5,
        height: 1.25,
        velocity_x: 0.75,
        velocity_y: -0.5,
        velocity_z: 0.25,
        lifetime: 901,
    });
    let mut legacy: serde_json::Value =
        serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    legacy["world"]["player"]
        .as_object_mut()
        .unwrap()
        .remove("item_instance_version");
    legacy["world"]["player"]["watering_content"] = serde_json::json!(731);
    let saved_entity = &mut legacy["world"]["levels"][0]["entities"]["entities"][0];
    saved_entity["kind"]["Item"]["count"] = serde_json::json!(3);
    saved_entity["kind"]["Item"]
        .as_object_mut()
        .unwrap()
        .remove("watering_content");
    let mut expected = saved_entity.clone();
    expected["kind"]["Item"]["count"] = serde_json::json!(1);
    expected.as_object_mut().unwrap().remove("id");

    let mut upgraded = World::from_save_string(&legacy.to_string()).unwrap();
    let entities = upgraded.levels[0].entities.entities();
    assert_eq!(entities.len(), 3);
    let mut ids = std::collections::BTreeSet::new();
    for entity in entities {
        assert!(ids.insert(entity.id));
        let mut state = serde_json::to_value(entity).unwrap();
        state.as_object_mut().unwrap().remove("id");
        assert_eq!(state, expected);
        assert!(matches!(
            entity.kind,
            EntityKind::Item(ItemStack {
                item: ItemId::WateringCan,
                count: 1,
                watering_content: None
            })
        ));
    }
    assert!(ids.contains(&original_id));
    assert!(water(&upgraded).is_empty());
    let resumed = restore(&upgraded);
    assert_eq!(
        serde_json::to_value(resumed.levels[0].entities.entities()).unwrap(),
        serde_json::to_value(upgraded.levels[0].entities.entities()).unwrap()
    );
    let new_id = upgraded.levels[0]
        .entities
        .spawn_item(ItemStack::new(ItemId::Wood, 1), 16, 16);
    assert!(!ids.contains(&new_id));
}

#[test]
fn actual_container_input_moves_exact_tool_instances_and_rejects_full_bag() {
    let mut world = tiny_world();
    world.player.inventory.add_stack(ItemStack::with_water(731));
    world.player.inventory.add(ItemId::Wood, 2);
    let mut first = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
    first.durability = 17;
    let mut second = first;
    second.durability = 11;
    world.player.inventory.add_tool(first).unwrap();
    world.player.inventory.add_tool(second).unwrap();
    world.player.active_item = Some(ActiveItem::Tool(1));
    let chest = world.levels[0].entities.spawn_furniture(
        FurnitureKind::Chest,
        world.player.x + 12,
        world.player.y,
    );
    world.levels[0]
        .entities
        .entities_mut_for_storage(chest)
        .unwrap()
        .add_stack(ItemStack::with_water(23));
    world.player.direction = Direction::Right;

    // Enter through the gameplay menu event, rather than calling the transfer helper.
    world.tick(&Input {
        craft: true,
        ..Default::default()
    });
    assert_eq!(world.container_id, Some(chest));
    assert!(world.inventory_open);
    world.tick(&Input {
        cursor_up_pressed: true,
        ..Default::default()
    });
    assert_eq!(world.inventory_item_selection, 3);
    world.tick(&Input {
        select: true,
        ..Default::default()
    });
    assert_eq!(world.player.inventory.tools(), &[first]);
    assert_eq!(world.player.active_item, None);
    assert_eq!(
        world.levels[0]
            .entities
            .death_chest_storage(chest)
            .unwrap()
            .tools(),
        &[second]
    );
    assert_eq!(water(&world), vec![Some(731)]);

    world.tick(&Input {
        cursor_right_pressed: true,
        ..Default::default()
    });
    assert_eq!(world.inventory_pane, 1);
    world.tick(&Input {
        cursor_down_pressed: true,
        ..Default::default()
    });
    assert_eq!(world.inventory_selection, 1);
    world.player.inventory.set_capacity(3);
    world.tick(&Input {
        attack: true,
        ..Default::default()
    });
    assert_eq!(world.player.inventory.tools(), &[first]);
    assert_eq!(
        world.levels[0]
            .entities
            .death_chest_storage(chest)
            .unwrap()
            .tools(),
        &[second]
    );
    world.player.inventory.set_capacity(27);
    world.tick(&Input {
        attack: true,
        ..Default::default()
    });
    assert_eq!(world.player.inventory.tools(), &[first, second]);
    assert!(
        world.levels[0]
            .entities
            .death_chest_storage(chest)
            .unwrap()
            .tools()
            .is_empty()
    );
    assert_eq!(
        world.levels[0]
            .entities
            .death_chest_storage(chest)
            .unwrap()
            .slots(),
        &[ItemStack::with_water(23)]
    );
    world.tick(&Input {
        exit: true,
        ..Default::default()
    });
    assert!(!world.inventory_open);
    assert_eq!(world.container_id, None);
    let resumed = restore(&world);
    assert_eq!(resumed.player.inventory.tools(), &[first, second]);
    assert_eq!(water(&resumed), vec![Some(731)]);
    assert_eq!(
        resumed.levels[0]
            .entities
            .death_chest_storage(chest)
            .unwrap()
            .slots(),
        &[ItemStack::with_water(23)]
    );
}
