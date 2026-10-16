//! Player-operation coverage for appended 2.3.0 identities and legacy tiles.
use super::{ActiveItem, Direction, Tile, World, entity::EntityKind, registry_tests::tiny_world};
use crate::item::{ItemId, ToolItem, ToolKind, ToolTier, dye::DyeColor};
use crate::level::tile::flower::FlowerVariant;

const TARGET: usize = 9 + 8 * 16;

fn round_trip(world: &World) -> World {
    World::from_save_string(&world.to_save_string().unwrap()).unwrap()
}

fn collect_target_drop(world: &mut World) {
    world.player.x = 9 * 16 + 8;
    world.player.y = 8 * 16 + 8;
    for entity in &mut world.levels[0].entities.entities {
        entity.age = 31;
    }
    world.levels[0].entities.collect_near(
        world.player.x,
        world.player.y,
        &mut world.player.inventory,
    );
}

#[test]
fn java230_wool_real_attack_save_last_shear_use_and_pickup_keep_every_color() {
    for color in DyeColor::ALL {
        let mut world = tiny_world();
        let wool = color.wool_item();
        world.player.direction = Direction::Right;
        world.levels[0].tiles[TARGET] = Tile::Hole;
        assert_eq!(world.player.inventory.add(wool, 1), 0);
        world.player.active_item = Some(ActiveItem::Stack(wool));
        world.attack();
        let tile = crate::item::wool::wool_tile(wool).unwrap();
        assert_eq!(world.levels[0].tiles[TARGET], tile);
        assert_eq!(world.player.inventory.count(wool), 0);
        assert_eq!(world.player.active_item, None);
        let mut world = round_trip(&world);
        assert_eq!(world.levels[0].tiles[TARGET], tile);
        let mut tool = ToolItem::new(ToolKind::Shears, ToolTier::Iron);
        tool.durability = 1;
        world.player.inventory.add_tool(tool).unwrap();
        world.player.active_item = Some(ActiveItem::Tool(0));
        world.attack();
        assert_eq!(world.levels[0].tiles[TARGET], Tile::Hole, "{color:?}");
        assert!(world.player.inventory.tools().is_empty());
        assert_eq!(world.player.active_item, None);
        assert_eq!(world.player.stamina, 9);
        assert_eq!(world.levels[0].entities.entities().iter().filter(|entity| matches!(entity.kind, EntityKind::Item(stack) if stack.item == wool && stack.count == 1)).count(), 1);
        let mut world = round_trip(&world);
        collect_target_drop(&mut world);
        assert_eq!(world.player.inventory.count(wool), 1);
        assert!(
            !world.levels[0]
                .entities
                .entities()
                .iter()
                .any(|entity| matches!(entity.kind, EntityKind::Item(_)))
        );
    }
}

#[test]
fn java230_flower_real_place_save_harvest_and_pickup_preserve_all_variants() {
    for variant in FlowerVariant::ALL {
        for shovel in [false, true] {
            let mut world = tiny_world();
            world.player.direction = Direction::Right;
            let item = variant.item();
            assert_eq!(world.player.inventory.add(item, 1), 0);
            world.player.active_item = Some(ActiveItem::Stack(item));
            world.attack();
            assert_eq!(world.levels[0].tiles[TARGET], Tile::Flower230);
            assert_eq!(world.levels[0].data[TARGET], variant as u16);
            assert_eq!(world.player.inventory.count(item), 0);
            let mut world = round_trip(&world);
            assert_eq!(world.levels[0].tiles[TARGET], Tile::Flower230);
            assert_eq!(world.levels[0].data[TARGET], variant as u16);
            if shovel {
                let mut tool = ToolItem::new(ToolKind::Shovel, ToolTier::Iron);
                tool.durability = 1;
                world.player.inventory.add_tool(tool).unwrap();
                world.player.active_item = Some(ActiveItem::Tool(0));
            }
            world.attack();
            assert_eq!(world.levels[0].tiles[TARGET], Tile::Grass, "{variant:?}");
            assert_eq!(world.levels[0].data[TARGET], 0);
            assert_eq!(world.player.active_item, None);
            assert!(world.player.inventory.tools().is_empty());
            let mut world = round_trip(&world);
            collect_target_drop(&mut world);
            assert_eq!(world.player.inventory.count(item), 1, "{variant:?}");
            assert_eq!(world.player.inventory.slots().len(), 1);
        }
    }
}

#[test]
fn java230_all_added_items_drop_pickup_death_chest_and_save_conserve_counts() {
    // Every group fits the normal 27-slot player bag; no test-only oversized bag.
    for group in ItemId::ALL[142..].chunks(20) {
        let mut world = tiny_world();
        for &item in group {
            assert_eq!(world.player.inventory.add(item, 2), 0);
        }
        let mut world = round_trip(&world);
        // Drive the same Q / Shift-Q edges as the running game. Rust active
        // items reference the bag, so each drop must deduct exactly once.
        for &item in group {
            world.player.active_item = Some(ActiveItem::Stack(item));
            world.tick_gameplay_player(&crate::io::Input {
                drop_one: true,
                ..Default::default()
            });
            assert_eq!(world.player.inventory.count(item), 1);
            world.tick_gameplay_player(&crate::io::Input {
                drop_stack: true,
                ..Default::default()
            });
            assert_eq!(world.player.inventory.count(item), 0);
            assert_eq!(world.player.active_item, None);
        }
        let mut world = round_trip(&world);
        for &item in group {
            assert_eq!(world.player.inventory.count(item), 0);
            let dropped: u16 = world.levels[0]
                .entities
                .entities()
                .iter()
                .filter_map(|entity| match entity.kind {
                    EntityKind::Item(stack) if stack.item == item => Some(stack.count),
                    _ => None,
                })
                .sum();
            assert_eq!(dropped, 2, "serialized loose {item:?}");
        }
        // spawn_item scatters each drop by up to 5px. Stay at the original
        // center: moving to one corner drop can leave the opposite corner
        // outside the 12px pickup radius despite both entities being intact.
        for entity in &mut world.levels[0].entities.entities {
            assert!((entity.x - world.player.x).abs() <= 5);
            assert!((entity.y - world.player.y).abs() <= 5);
            entity.age = 31;
        }
        world.tick_gameplay_player(&crate::io::Input::default());
        for &item in group {
            assert_eq!(world.player.inventory.count(item), 2);
        }
        world.player.active_item = Some(ActiveItem::Stack(group[0]));
        let location = (world.player.x, world.player.y);
        world.die_player();
        assert!(world.player.inventory.slots().is_empty());
        let mut world = round_trip(&world);
        let chest_id = world.levels[0].entities.entities()[0].id;
        let storage = world.levels[0]
            .entities
            .death_chest_storage(chest_id)
            .unwrap();
        for &item in group {
            assert_eq!(storage.count(item), 2);
        }
        world.respawn();
        world.player.x = location.0;
        world.player.y = location.1;
        world.retrieve_death_chests();
        assert!(world.levels[0].entities.entities().is_empty());
        for &item in group {
            assert_eq!(world.player.inventory.count(item), 2);
        }
        let world = round_trip(&world);
        for &item in group {
            assert_eq!(world.player.inventory.count(item), 2);
        }
    }
}

#[test]
fn java230_legacy_flower_shape_data_and_two_drop_shovel_rule_survive() {
    for data in [0, 1, 16, 17, 19] {
        let mut world = tiny_world();
        world.player.direction = Direction::Right;
        world.levels[0].tiles[TARGET] = Tile::Flower;
        world.levels[0].data[TARGET] = data;
        let mut world = round_trip(&world);
        assert_eq!(world.levels[0].tiles[TARGET], Tile::Flower);
        assert_eq!(world.levels[0].data[TARGET], data);
        world
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Shovel, ToolTier::Wood))
            .unwrap();
        world.player.active_item = Some(ActiveItem::Tool(0));
        world.attack();
        assert_eq!(world.levels[0].tiles[TARGET], Tile::Grass);
        collect_target_drop(&mut world);
        assert_eq!(world.player.inventory.count(ItemId::Flower), 1);
        assert_eq!(world.player.inventory.count(ItemId::Rose), 1);
    }
}

#[test]
fn java230_shears_do_not_gain_general_attacks_while_enabling_wool_harvest() {
    for tile in [Tile::Tree, Tile::Flower230] {
        let mut world = tiny_world();
        world.player.direction = Direction::Right;
        world.levels[0].tiles[TARGET] = tile;
        world.levels[0].data[TARGET] = 3;
        let tool = ToolItem::new(ToolKind::Shears, ToolTier::Iron);
        world.player.inventory.add_tool(tool).unwrap();
        world.player.active_item = Some(ActiveItem::Tool(0));
        world.attack();
        assert_eq!(world.levels[0].tiles[TARGET], tile);
        assert_eq!(world.levels[0].data[TARGET], 3);
        assert_eq!(world.player.inventory.tools(), &[tool]);
        assert!(world.levels[0].entities.entities().is_empty());
    }
    let mut world = tiny_world();
    world.player.direction = Direction::Right;
    let tool = ToolItem::new(ToolKind::Shears, ToolTier::Iron);
    world.player.inventory.add_tool(tool).unwrap();
    world.player.active_item = Some(ActiveItem::Tool(0));
    let species = crate::entity::mob::NaturalMob::Zombie;
    world.levels[0]
        .entities
        .spawn_mob(species, world.player.x + 12, world.player.y);
    let health = world.levels[0].entities.active_boss(species).unwrap();
    world.attack();
    assert_eq!(world.levels[0].entities.active_boss(species), Some(health));
    assert_eq!(world.player.inventory.tools(), &[tool]);
}
