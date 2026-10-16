//! Versioned natural flora: actual generation, watering/harvest, and saved maps.
use super::*;
use crate::level::tile::flower::FlowerVariant;

fn roundtrip(world: &World) -> World {
    World::from_save_string(&world.to_save_string().unwrap()).unwrap()
}

fn collect(world: &mut World, x: i32, y: i32) {
    // Elapse only the pickup delay; no items, counts or RNG are fabricated.
    for entity in &mut world.levels[world.current_level].entities.entities {
        entity.age = entity.age.max(31);
    }
    world.levels[world.current_level]
        .entities
        .collect_near(x, y, &mut world.player.inventory);
    world.update_progress(ProgressEvent::InventoryChanged);
}

#[test]
fn java230_flora_assembled_world_keeps_species_and_harvests_stay_harvested() {
    let mut world = World::new_with_generation(
        42,
        WorldSpec::default(),
        PlayOptions::survival(1),
        GenerationVersion::Java230Flora,
    );
    assert!(world.player.inventory.slots().is_empty());
    let level = &world.levels[world.current_level];
    let flowers: Vec<_> = level
        .tiles
        .iter()
        .zip(&level.data)
        .enumerate()
        .filter(|(_, (tile, _))| **tile == Tile::Flower230)
        .map(|(i, (_, d))| (i, *d))
        .collect();
    assert!(flowers.len() > 100);
    assert!(
        flowers.iter().any(|&(_, d)| d != 0),
        "assembly must not erase flower species data"
    );
    assert!(
        world
            .levels
            .iter()
            .filter(|l| l.depth != 0)
            .all(|l| !l.tiles.contains(&Tile::Flower230))
    );
    let (index, data) = flowers[0];
    let x = (index % world.width) as i32;
    let y = (index / world.width) as i32;
    world.attack_target_tile(x, y, index);
    collect(&mut world, x * TILE_SIZE + 8, y * TILE_SIZE + 8);
    assert_eq!(
        world
            .player
            .inventory
            .count(FlowerVariant::from_data(data).unwrap().item()),
        1
    );
    let restored = roundtrip(&world);
    assert_eq!(
        restored.levels[restored.current_level].tiles[index],
        Tile::Grass
    );
    assert_eq!(restored.levels[restored.current_level].data[index], 0);
    assert_eq!(
        restored.generation_version(),
        GenerationVersion::Java230Flora
    );
}

#[test]
fn java230_flora_watering_is_renewable_deterministic_saved_and_rng_isolated() {
    let java: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/watering-flora.json"
    ))
    .unwrap();
    assert_eq!(java["observedVariants"].as_array().unwrap().len(), 19);
    assert_eq!(java["capacity"], crate::item::WATERING_CAN_CAPACITY);
    assert_eq!(java["chanceDenominator"], 60);
    let mut world = registry_tests::tiny_world();
    world.generation_version = GenerationVersion::Java230Flora;
    // This focused fixture tests water use; the from-scratch test below crafts the can.
    world.player.inventory.add(ItemId::WateringCan, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::WateringCan));
    let target = 9 + 8 * world.width;
    let water = 8 + 8 * world.width;
    world.levels[0].tiles[water] = Tile::Water;
    world.attack_target_tile(8, 8, water);
    assert_eq!(world.player.watering_content, 1800);
    let mut resumed = roundtrip(&world);
    let mut expected = JavaRandom::new(0);
    let rng = serde_json::to_value(&world.random).unwrap();
    let mut counts = [0; 19];
    for attempt in 0..12_000 {
        if world.player.watering_content == 0 {
            world.attack_target_tile(8, 8, water);
            resumed.attack_target_tile(8, 8, water);
        }
        let content = world.player.watering_content;
        let blooms = expected.next_int(60) == 0;
        let variant = if blooms {
            Some(expected.next_int(19) as u16)
        } else {
            None
        };
        world.attack_target_tile(9, 8, target);
        resumed.attack_target_tile(9, 8, target);
        assert_eq!(world.player.watering_content, content - 1);
        assert_eq!(
            world.levels[0].tiles[target],
            resumed.levels[0].tiles[target]
        );
        assert_eq!(world.levels[0].data[target], resumed.levels[0].data[target]);
        if let Some(variant) = variant {
            assert_eq!(world.levels[0].tiles[target], Tile::Flower230);
            assert_eq!(world.levels[0].data[target], variant);
            counts[variant as usize] += 1;
            // Harvest actual blooms so the next interaction again targets grass.
            for w in [&mut world, &mut resumed] {
                w.player.active_item = None;
                w.attack_target_tile(9, 8, target);
                collect(w, 9 * TILE_SIZE + 8, 8 * TILE_SIZE + 8);
                w.player.active_item = Some(ActiveItem::Stack(ItemId::WateringCan));
            }
        } else {
            assert_eq!(world.levels[0].tiles[target], Tile::Grass);
        }
        if attempt == 4000 {
            resumed = roundtrip(&resumed);
        }
    }
    assert!(
        counts.iter().all(|&count| count > 0),
        "all19 must be obtainable: {counts:?}"
    );
    for variant in FlowerVariant::ALL {
        assert_eq!(
            world.player.inventory.count(variant.item()),
            counts[variant as usize]
        );
        assert_eq!(
            resumed.player.inventory.count(variant.item()),
            counts[variant as usize]
        );
    }
    assert_eq!(
        rng,
        serde_json::to_value(&world.random).unwrap(),
        "watering must not alter unrelated drop/AI randomness"
    );
    assert_eq!(
        serde_json::to_value(&world.flora_random).unwrap(),
        serde_json::to_value(&resumed.flora_random).unwrap()
    );
    // No bloom with empty content, no invalid target replacement.
    world.player.watering_content = 0;
    let before = serde_json::to_value(&world.flora_random).unwrap();
    world.attack_target_tile(9, 8, target);
    assert_eq!(before, serde_json::to_value(&world.flora_random).unwrap());
    world.player.watering_content = 1;
    world.levels[0].tiles[target] = Tile::StairsDown;
    world.attack_target_tile(9, 8, target);
    assert_eq!(world.levels[0].tiles[target], Tile::StairsDown);
    // Legacy worlds keep their old map and interaction contract.
    world.generation_version = GenerationVersion::Legacy224;
    world.levels[0].tiles[target] = Tile::Grass;
    for _ in 0..100 {
        world.player.watering_content = 1;
        world.attack_target_tile(9, 8, target);
    }
    assert_eq!(world.levels[0].tiles[target], Tile::Grass);
    assert_eq!(before, serde_json::to_value(&world.flora_random).unwrap());
}

#[test]
fn java230_flora_old_states_default_legacy_without_reinterpreting_or_regenerating() {
    let mut world = registry_tests::tiny_world();
    world.player.inventory.add(ItemId::CyanWool, 4); // Phase1 identity remains unchanged.
    world.levels[0].tiles[0] = Tile::Flower;
    world.levels[0].data[0] = 51;
    world.levels[0].tiles[1] = Tile::Flower230;
    world.levels[0].data[1] = 18;
    let mut old: serde_json::Value =
        serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    old["world"]
        .as_object_mut()
        .unwrap()
        .remove("generation_version");
    old["world"].as_object_mut().unwrap().remove("flora_random");
    let restored = World::from_save_string(&old.to_string()).unwrap();
    assert_eq!(restored.generation_version(), GenerationVersion::Legacy224);
    assert_eq!(restored.levels[0].tiles, world.levels[0].tiles);
    assert_eq!(restored.levels[0].data, world.levels[0].data);
    assert_eq!(restored.player.inventory.count(ItemId::CyanWool), 4);
    assert_eq!(roundtrip(&restored).levels[0].data, world.levels[0].data);
    for invalid in [
        serde_json::json!("UnknownFutureGenerator"),
        serde_json::Value::Null,
        serde_json::json!(42),
    ] {
        old["world"]["generation_version"] = invalid;
        assert!(
            World::from_save_string(&old.to_string()).is_err(),
            "a damaged marker must never cause regeneration"
        );
    }
}

fn face(world: &mut World, x: i32, y: i32) {
    world.player.x = x - 16;
    world.player.y = y;
    world.player.direction = Direction::Right;
}

fn open_station(world: &mut World, kind: FurnitureKind) {
    let (x, y) = world.levels[world.current_level]
        .entities
        .entities()
        .iter()
        .find_map(|e| {
            matches!(e.kind,entity::EntityKind::Furniture(k) if k==kind).then_some((e.x, e.y))
        })
        .expect("crafted station was placed");
    face(world, x, y);
    assert!(world.use_furniture_front(), "station interaction {kind:?}");
    assert_eq!(world.crafting_station, Some(kind));
}

fn craft(world: &mut World, name: &str) {
    world.refresh_crafting_order();
    world.select_crafting_product_for_parity(name).unwrap();
    let recipes = ordered_recipe_views(
        world.crafting_station,
        &world.player.inventory,
        false,
        &world.crafting_order,
        &world.progress,
    );
    let recipe = recipes[world.inventory_selection];
    assert!(
        recipe_can_craft(&recipe, &world.player.inventory, false),
        "survival materials for {name}"
    );
    world.craft_selected_recipe();
}

fn place(world: &mut World, item: ItemId) {
    let level = &world.levels[world.current_level];
    let index = (world.width + 1..level.tiles.len() - world.width)
        .find(|&i| {
            i % world.width > 1
                && level.tiles[i] == Tile::Grass
                && level.tiles[i - 1] == Tile::Grass
                && level.entities.entities().iter().all(|e| {
                    (e.x - (i % world.width) as i32 * 16 - 8).abs() > 32
                        || (e.y - (i / world.width) as i32 * 16 - 8).abs() > 32
                })
        })
        .unwrap();
    face(
        world,
        (index % world.width) as i32 * 16 + 8,
        (index / world.width) as i32 * 16 + 8,
    );
    world.player.active_item = Some(ActiveItem::Stack(item));
    let before = world.player.inventory.count(item);
    world.attack();
    assert_eq!(
        world.player.inventory.count(item),
        before - 1,
        "place crafted {item:?}"
    );
}

fn gather(world: &mut World, tile: Tile, item: ItemId, amount: u16) {
    for _ in 0..500 {
        if world.player.inventory.count(item) >= amount {
            return;
        }
        let index = world.levels[world.current_level]
            .tiles
            .iter()
            .position(|&t| t == tile)
            .expect("resource exists in natural map");
        let x = (index % world.width) as i32;
        let y = (index / world.width) as i32;
        for _ in 0..150 {
            if world.levels[world.current_level].tiles[index] != tile {
                break;
            }
            if tile != Tile::Tree {
                if !world
                    .player
                    .inventory
                    .tools()
                    .iter()
                    .any(|t| t.kind == ToolKind::Pickaxe && !t.is_depleted())
                {
                    assert_eq!(
                        world.levels[world.current_level].depth, 0,
                        "spare crafted pickaxe must last this short ore trip"
                    );
                    open_station(world, FurnitureKind::Workbench);
                    craft(world, "Wood Pickaxe");
                }
                let tool = world
                    .player
                    .inventory
                    .tools()
                    .iter()
                    .position(|t| t.kind == ToolKind::Pickaxe && !t.is_depleted())
                    .unwrap();
                world.player.active_item = Some(ActiveItem::Tool(tool));
            } else {
                world.player.active_item = None;
            }
            face(world, x * 16 + 8, y * 16 + 8);
            // Only elapse stat-recharge time. This test omits pathfinding, AI
            // and wall-clock waiting, but never restores stats or mints items.
            for _ in 0..100 {
                world.tick_survival_stats();
            }
            assert!(world.player.stamina > 0);
            world.attack_target_tile(x, y, index);
            world.remove_depleted_active_tool();
        }
        assert_ne!(
            world.levels[world.current_level].tiles[index], tile,
            "resource harvesting terminates"
        );
        collect(world, x * 16 + 8, y * 16 + 8);
    }
    panic!("could not gather {item:?} from {tile:?}");
}

#[test]
fn java230_flora_survival_from_empty_bag_to_natural_dyes_wool_bed_and_restore() {
    let mut world = World::new_with_generation(
        42,
        WorldSpec::default(),
        PlayOptions::survival(1),
        GenerationVersion::Java230Flora,
    );
    assert!(world.player.inventory.slots().is_empty());
    assert!(world.player.inventory.tools().is_empty());
    // Headless movement is abbreviated to resource/station coordinates. All
    // materials are harvested or dropped by the production natural spawn code.
    gather(&mut world, Tile::Tree, ItemId::Wood, 65);
    world.crafting_station = None;
    craft(&mut world, "Workbench");
    place(&mut world, ItemId::Workbench);
    open_station(&mut world, FurnitureKind::Workbench);
    craft(&mut world, "Wood Pickaxe");
    gather(&mut world, Tile::Rock, ItemId::Stone, 40);
    gather(&mut world, Tile::Rock, ItemId::Coal, 15);
    open_station(&mut world, FurnitureKind::Workbench);
    craft(&mut world, "Furnace");
    place(&mut world, ItemId::Furnace);
    open_station(&mut world, FurnitureKind::Workbench);
    craft(&mut world, "Rock Pickaxe");
    // Two fresh rock picks provide an ordinary, craftable reserve underground.
    craft(&mut world, "Rock Pickaxe");
    let stairs = world.levels[world.current_level]
        .tiles
        .iter()
        .position(|&t| t == Tile::StairsDown)
        .unwrap();
    world.player.x = (stairs % world.width) as i32 * 16 + 8;
    world.player.y = (stairs / world.width) as i32 * 16 + 8;
    world.apply_level_change(1);
    assert_eq!(world.levels[world.current_level].depth, -1);
    gather(&mut world, Tile::IronOre, ItemId::IronOre, 39);
    world.player.x = (stairs % world.width) as i32 * 16 + 8;
    world.player.y = (stairs / world.width) as i32 * 16 + 8;
    world.apply_level_change(-1);
    open_station(&mut world, FurnitureKind::Furnace);
    for _ in 0..13 {
        craft(&mut world, "Iron");
    }
    open_station(&mut world, FurnitureKind::Workbench);
    craft(&mut world, "Anvil");
    place(&mut world, ItemId::Anvil);
    open_station(&mut world, FurnitureKind::Anvil);
    craft(&mut world, "Watering Can");
    open_station(&mut world, FurnitureKind::Workbench);
    craft(&mut world, "Dye Vat");
    place(&mut world, ItemId::DyeVat);
    // Sheep are selected by real surface spawn rules, not manually inserted.
    for _ in 0..1000 {
        if world.player.inventory.count(ItemId::Wool) >= 8 {
            break;
        }
        let current = world.current_level;
        super::try_queue_natural_spawn(
            &mut world.levels[current],
            world.width,
            world.height,
            world.player.x,
            world.player.y,
            18_000,
            0,
            &mut world.random,
        );
        world.tick_level_entities(None, false, true);
        while let Some((x, y)) = world.levels[current]
            .entities
            .entities()
            .iter()
            .find_map(|e| {
                matches!(&e.kind,entity::EntityKind::Mob(m) if m.species==spawn::NaturalMob::Sheep)
                    .then_some((e.x, e.y))
            })
        {
            face(&mut world, x, y);
            world.player.active_item = None;
            // Mob hurt cooldown elapses through normal entity ticks between hits.
            for _ in 0..100 {
                if !world.levels[current].entities.mob_near(x, y) {
                    break;
                }
                world.attack();
                for _ in 0..12 {
                    world.tick_level_entities(None, false, true);
                }
            }
            collect(&mut world, x, y);
            // Sheep can walk; collect at the actual drop coordinates as well.
            let drops: Vec<_> = world.levels[current]
                .entities
                .entities()
                .iter()
                .filter_map(|e| {
                    matches!(e.kind,entity::EntityKind::Item(s) if s.item==ItemId::Wool)
                        .then_some((e.x, e.y))
                })
                .collect();
            for (dx, dy) in drops {
                collect(&mut world, dx, dy);
            }
        }
    }
    assert!(world.player.inventory.count(ItemId::Wool) >= 8);
    open_station(&mut world, FurnitureKind::Workbench);
    craft(&mut world, "Loom");
    place(&mut world, ItemId::Loom);
    // Three naturally generated roses suffice for a red bed via dyed wool.
    gather(&mut world, Tile::Flower230, ItemId::Rose, 3);
    open_station(&mut world, FurnitureKind::DyeVat);
    for _ in 0..3 {
        craft(&mut world, "Red Dye");
    }
    open_station(&mut world, FurnitureKind::Loom);
    for _ in 0..3 {
        craft(&mut world, "Red Wool");
    }
    craft(&mut world, "Red Bed");
    place(&mut world, ItemId::RedBed);
    // Watering natural grass supplies otherwise unavailable White Lily.
    let water = world.levels[world.current_level]
        .tiles
        .iter()
        .position(|&t| t == Tile::Water)
        .unwrap();
    let grass = world.levels[world.current_level]
        .tiles
        .iter()
        .position(|&t| t == Tile::Grass)
        .unwrap();
    let gx = (grass % world.width) as i32;
    let gy = (grass / world.width) as i32;
    let wx = (water % world.width) as i32;
    let wy = (water / world.width) as i32;
    for _ in 0..30_000 {
        world.player.active_item = Some(ActiveItem::Stack(ItemId::WateringCan));
        if world.player.watering_content == 0 {
            world.attack_target_tile(wx, wy, water);
        }
        world.attack_target_tile(gx, gy, grass);
        if world.levels[world.current_level].tiles[grass] == Tile::Flower230 {
            let white =
                world.levels[world.current_level].data[grass] == FlowerVariant::WhiteLily as u16;
            world.player.active_item = None;
            world.attack_target_tile(gx, gy, grass);
            if white {
                collect(&mut world, gx * 16 + 8, gy * 16 + 8);
                break;
            }
        }
    }
    assert_eq!(world.player.inventory.count(ItemId::WhiteLily), 1);
    open_station(&mut world, FurnitureKind::DyeVat);
    craft(&mut world, "White Dye");
    assert_eq!(world.player.inventory.count(ItemId::WhiteDye), 1);
    world.clock.set_day_tick(48_600);
    open_station_for_sleep(&mut world);
    assert!(world.player_removed);
    let respawn = world.respawn_tile;
    let mut world = roundtrip(&world);
    assert_eq!(world.respawn_tile, respawn);
    assert_eq!(world.player.inventory.count(ItemId::WhiteDye), 1);
    world.clock.set_day_tick(crate::core::sleep::END);
    world.tick_sleep_prelude();
    assert!(!world.player_removed);
    assert_eq!(world.levels[world.current_level].tiles[grass], Tile::Grass);
    // Real crafted new materials also survive loose drops and death recovery.
    world.player.active_item = Some(ActiveItem::Stack(ItemId::WhiteDye));
    let location = (world.player.x, world.player.y);
    world.drop_active_item(true);
    collect(&mut world, location.0, location.1);
    assert_eq!(world.player.inventory.count(ItemId::WhiteDye), 1);
    world.die_player();
    let mut world = roundtrip(&world);
    world.respawn();
    world.player.x = location.0;
    world.player.y = location.1;
    world.retrieve_death_chests();
    assert_eq!(world.player.inventory.count(ItemId::WhiteDye), 1);
    assert_eq!(world.generation_version(), GenerationVersion::Java230Flora);
}

fn open_station_for_sleep(world: &mut World) {
    let (x, y) = world.levels[world.current_level]
        .entities
        .entities()
        .iter()
        .find_map(|e| {
            matches!(e.kind, entity::EntityKind::Furniture(FurnitureKind::Bed))
                .then_some((e.x, e.y))
        })
        .unwrap();
    face(world, x, y);
    assert!(world.use_furniture_front());
    assert_eq!(world.sleeping, 120);
}

#[test]
fn java230_flora_all_sixteen_colors_have_survival_recipe_paths() {
    use crate::item::dye::DyeColor;
    // Resource availability graph, not a claim that all16 GUI crafts were played.
    // Seeds/resources, all19 watering outputs and counts are tested separately;
    // exact recipe multiplicities remain checked against the Java recipe oracle.
    let mut reachable = vec![
        ItemId::Wood,
        ItemId::Stone,
        ItemId::Coal,
        ItemId::IronOre,
        ItemId::Wool,
        ItemId::Cactus,
    ];
    reachable.extend(FlowerVariant::ALL.map(|variant| variant.item()));
    for _ in 0..10 {
        let before = reachable.len();
        for station in [
            None,
            Some(FurnitureKind::Workbench),
            Some(FurnitureKind::Furnace),
            Some(FurnitureKind::Anvil),
            Some(FurnitureKind::DyeVat),
            Some(FurnitureKind::Loom),
        ] {
            let available = match station {
                None => true,
                Some(kind) => crate::item::item_for_furniture(kind)
                    .is_some_and(|item| reachable.contains(&item)),
            };
            if !available {
                continue;
            }
            for recipe in crate::item::recipe_views(station) {
                if recipe
                    .costs
                    .iter()
                    .all(|cost| reachable.contains(&cost.item))
                    && let RecipeOutput::Stack(output) = recipe.output
                    && !reachable.contains(&output.item)
                {
                    reachable.push(output.item);
                }
            }
        }
        if before == reachable.len() {
            break;
        }
    }
    for color in DyeColor::ALL {
        for item in [color.dye_item(), color.wool_item(), color.bed_item()] {
            assert!(
                reachable.contains(&item),
                "unreachable Survival item {item:?}"
            );
        }
    }
}
