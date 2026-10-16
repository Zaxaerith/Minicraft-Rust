//! Player operation regressions for Java 2.3.0-dev colored furniture.
use super::registry_tests::tiny_world;
use super::{ActiveItem, FurnitureKind, Inventory, ItemId, World};
use crate::entity::furniture::bed;
use crate::item::dye::DyeColor;

#[test]
fn java230_all_colored_beds_place_sleep_save_and_pick_up() {
    let java_beds: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/beds.json")).unwrap();
    let java_sleep: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/operations.json")).unwrap();
    for color in DyeColor::ALL {
        let java = &java_beds[color as usize];
        let mut world = tiny_world();
        world.player.x = 6 * 16 + 8;
        world.player.y = 7 * 16 + 8;
        let item = color.bed_item();
        assert_eq!(item.display_name(), java["name"].as_str().unwrap());
        assert_eq!(item.asset_name(), java["itemSprite"].as_str().unwrap());
        assert_eq!(
            FurnitureKind::Bed.collision_radius(),
            (
                java["xr"].as_i64().unwrap() as i32,
                java["yr"].as_i64().unwrap() as i32
            )
        );
        world.player.inventory.add(item, 1);
        world.player.active_item = Some(ActiveItem::Stack(item));
        world.attack();
        assert_eq!(world.player.inventory.count(item), 0);
        let entity = &world.levels[0].entities.entities()[0];
        assert_eq!(entity.state, bed::state_for_color(color));
        assert_eq!(bed::color_from_state(entity.state), color);
        assert_eq!(
            u64::from(bed::color_from_state(entity.state) as u8),
            java["restoredColor"].as_u64().unwrap()
        );
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        world.clock.set_day_tick(48_600);
        assert!(world.use_furniture_front());
        assert_eq!(world.sleeping, 120);
        assert!(world.player_removed);
        assert_eq!(
            world.respawn_tile,
            Some((
                java_sleep["spawnx"].as_i64().unwrap() as i32,
                java_sleep["spawny"].as_i64().unwrap() as i32
            ))
        );
        assert_eq!(world.bed_level, Some(0));
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        assert_eq!(
            world.levels[0].entities.entities()[0].state,
            bed::state_for_color(color)
        );
        world.clock.set_day_tick(crate::core::sleep::END);
        world.tick_sleep_prelude();
        assert_eq!(world.sleeping, 0);
        assert!(!world.player_removed);
        world.pickup_target();
        assert!(world.levels[0].entities.entities().is_empty());
        assert_eq!(world.player.inventory.count(item), 1);
        assert_eq!(world.player.active_item, Some(ActiveItem::Stack(item)));
        let world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        assert_eq!(world.player.inventory.count(item), 1);
    }
}

#[test]
fn java230_full_bag_leaves_colored_bed_and_legacy_bed_is_unchanged() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::RedBed, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::RedBed));
    world.attack();
    world.player.inventory = Inventory::new(1);
    world.player.inventory.add(ItemId::Stone, 1);
    let state_before = world.levels[0].entities.entities()[0].state;
    assert_eq!(state_before, 2); // shares only a number with the Chest-specific death marker.
    world.pickup_target();
    assert_eq!(world.levels[0].entities.entities().len(), 1);
    assert_eq!(world.levels[0].entities.entities()[0].state, state_before);
    assert_eq!(world.player.inventory.count(ItemId::Stone), 1);
    assert_eq!(world.player.inventory.count(ItemId::RedBed), 0);
    // The bed remains pushable; the death chest guard applies only to containers.
    let entity = &world.levels[0].entities.entities()[0];
    let (x, y) = (entity.x, entity.y);
    assert!(
        world.levels[0]
            .entities
            .try_push_furniture_near(x - 4, y, 1, 0)
    );
    world.player.inventory.remove(ItemId::Stone, 1);
    world.pickup_target();
    assert_eq!(world.player.inventory.count(ItemId::RedBed), 1);
    assert!(world.levels[0].entities.entities().is_empty());

    let mut legacy = tiny_world();
    legacy.player.inventory.add(ItemId::Bed, 1);
    legacy.player.active_item = Some(ActiveItem::Stack(ItemId::Bed));
    legacy.attack();
    let mut legacy = World::from_save_string(&legacy.to_save_string().unwrap()).unwrap();
    assert_eq!(legacy.levels[0].entities.entities()[0].state, 0);
    assert_eq!(bed::color_from_state(0), DyeColor::Red);
    legacy.pickup_target();
    assert_eq!(legacy.player.inventory.count(ItemId::Bed), 1);
}

#[test]
fn java230_dye_vat_place_open_save_and_pick_up() {
    let mut world = tiny_world();
    assert_eq!(FurnitureKind::DyeVat.collision_radius(), (0, 0));
    assert!(FurnitureKind::DyeVat.crafting());
    world.player.inventory.add(ItemId::DyeVat, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::DyeVat));
    world.attack();
    assert_eq!(world.player.inventory.count(ItemId::DyeVat), 0);
    let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert!(world.use_furniture_front());
    assert!(world.inventory_open);
    assert_eq!(world.crafting_station, Some(FurnitureKind::DyeVat));
    assert!(!world.personal_crafting);
    assert_eq!(world.inventory_pane, 1);
    world.close_container();
    world.pickup_target();
    assert!(world.levels[0].entities.entities().is_empty());
    assert_eq!(world.player.inventory.count(ItemId::DyeVat), 1);
    let world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert_eq!(world.player.inventory.count(ItemId::DyeVat), 1);
}

#[test]
fn java230_drop_input_keeps_tool_instances_and_creative_item_totals() {
    use crate::entity::EntityKind;
    use crate::io::Input;
    use crate::item::{ToolItem, ToolKind, ToolTier};
    for mode in [super::GameMode::Survival, super::GameMode::Creative] {
        let mut world = tiny_world();
        world.mode = mode;
        let mut dropped = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
        dropped.durability = 17;
        let mut retained = ToolItem::new(ToolKind::Axe, ToolTier::Wood);
        retained.durability = 9;
        world.player.inventory.add_tool(dropped);
        world.player.inventory.add_tool(retained);
        world.player.active_item = Some(ActiveItem::Tool(0));
        world.player.attack_item = Some(ActiveItem::Tool(1));
        world.player.fishing_level = Some(0);
        world.player.fishing_ticks = 5;
        world.tick_gameplay_player(&Input {
            drop_one: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.tools(), &[retained]);
        assert_eq!(world.player.active_item, None);
        assert_eq!(world.player.attack_item, None);
        assert_eq!(world.player.fishing_level, None);
        assert_eq!(world.player.fishing_ticks, 0);
        assert!(
            matches!(world.levels[0].entities.entities()[0].kind, EntityKind::Tool(tool) if tool == dropped)
        );
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        world.levels[0].entities.entities[0].age = 31;
        world.tick_gameplay_player(&Input::default());
        assert!(world.levels[0].entities.entities().is_empty());
        assert_eq!(world.player.inventory.tools(), &[retained, dropped]);
        world.player.inventory.add(ItemId::BrownDye, 1_000);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::BrownDye));
        world.tick_gameplay_player(&Input {
            drop_stack: true,
            drop_one: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.count(ItemId::BrownDye), 1);
        assert!(
            matches!(world.levels[0].entities.entities()[0].kind, EntityKind::Item(stack) if stack == crate::item::ItemStack::new(ItemId::BrownDye, 999))
        );
        world.tick_gameplay_player(&Input {
            drop_one: true,
            ..Input::default()
        });
        assert_eq!(world.player.inventory.count(ItemId::BrownDye), 0);
        assert_eq!(world.player.active_item, None);
        for entity in &mut world.levels[0].entities.entities {
            entity.age = 31;
        }
        world.tick_gameplay_player(&Input::default());
        assert_eq!(world.player.inventory.count(ItemId::BrownDye), 1_000);
        assert!(world.levels[0].entities.entities().is_empty());
    }
}
