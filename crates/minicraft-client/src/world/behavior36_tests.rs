use super::*;
use crate::item::dye::DyeColor;
use serde_json::Value;
use std::collections::BTreeMap;

fn dropped(world: &World) -> Value {
    let mut counts = BTreeMap::<String, u16>::new();
    for e in world.levels[world.current_level].entities.entities() {
        if let EntityKind::Item(stack) = e.kind {
            let name = if stack.item == ItemId::Wool {
                "White Wool"
            } else {
                stack.item.display_name()
            };
            *counts.entry(name.to_owned()).or_default() += stack.count;
        }
    }
    serde_json::to_value(counts).unwrap()
}
#[test]
fn java36_all_sheep_colors_dye_shear_death_and_native_resume() {
    let source: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/sheep36.json")).unwrap();
    for case in source["cases"].as_array().unwrap() {
        let color = DyeColor::from_index(case["color"].as_u64().unwrap() as u16).unwrap();
        let difficulty = case["difficulty"].as_u64().unwrap() as usize;
        let mut world = registry_tests::tiny_world();
        world.difficulty = difficulty;
        let id = world.levels[0]
            .entities
            .spawn_mob_level(
                spawn::NaturalMob::Sheep,
                1,
                difficulty as u8,
                false,
                148,
                136,
            )
            .unwrap();
        world.player.direction = Direction::Right;
        world.player.inventory.add(color.dye_item(), 2);
        world.player.active_item = Some(ActiveItem::Stack(color.dye_item()));
        world.attack();
        assert_eq!(
            world.player.inventory.count(color.dye_item()) as u64,
            case["dyeRemaining"].as_u64().unwrap()
        );
        let EntityKind::Mob(mob) = &world.levels[0].entities.entities()[0].kind else {
            panic!()
        };
        assert_eq!(
            u64::from(mob.sheep_color),
            case["colorAfter"].as_u64().unwrap()
        );
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        let mut shears = ToolItem::new(ToolKind::Shears, ToolTier::Iron);
        shears.durability = 1;
        let tool = world.player.inventory.add_tool(shears).unwrap();
        world.player.active_item = Some(ActiveItem::Tool(tool));
        world.random = JavaRandom::new(1234);
        world.attack();
        assert!(
            world.player.inventory.tools().is_empty(),
            "last durability removes the used shears"
        );
        assert_eq!(dropped(&world), case["shearDrops"]);
        let original = &world.levels[0]
            .entities
            .entities()
            .iter()
            .find(|e| e.id == id)
            .unwrap();
        let EntityKind::Mob(mob) = &original.kind else {
            panic!()
        };
        assert!(mob.sheared);
        assert_eq!(mob.sheep_color, color as u8);
        // The same actual creature grazes, regrowing wool without changing dye.
        let grazing_seed = (0..100_000)
            .find(|s| JavaRandom::new(*s).next_int(1000) == 0)
            .unwrap();
        world.levels[0].tiles[9 + 8 * 16] = Tile::Grass;
        let level = &mut world.levels[0];
        level.entities.tick_with_player_difficulty(
            &mut level.tiles,
            &level.data,
            16,
            16,
            None,
            false,
            true,
            &mut JavaRandom::new(grazing_seed),
            difficulty as u8,
            0,
            0,
            0,
            false,
        );
        let EntityKind::Mob(mob) = &world.levels[0]
            .entities
            .entities()
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .kind
        else {
            panic!()
        };
        assert!(!mob.sheared);
        assert_eq!(mob.sheep_color, color as u8);
        // Exact source death rolls, both cut states, and native ground persistence.
        for cut in [false, true] {
            let mut fresh = registry_tests::tiny_world();
            fresh.levels[0]
                .entities
                .spawn_mob_level(
                    spawn::NaturalMob::Sheep,
                    1,
                    difficulty as u8,
                    false,
                    148,
                    136,
                )
                .unwrap();
            let EntityKind::Mob(m) = &mut fresh.levels[0].entities.entities[0].kind else {
                panic!()
            };
            m.sheep_color = color as u8;
            m.sheared = cut;
            fresh.levels[0]
                .entities
                .damage_nearest(148, 136, 1000, &mut JavaRandom::new(1234))
                .unwrap();
            assert_eq!(
                dropped(&fresh),
                case[if cut {
                    "shearedDeathDrops"
                } else {
                    "deathDrops"
                }]
            );
            let loaded = World::from_save_string(&fresh.to_save_string().unwrap()).unwrap();
            assert_eq!(dropped(&loaded), dropped(&fresh));
        }
    }
}

#[test]
fn java36_real_natural_sheep_colors_and_legacy_white_defaults() {
    let source: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/sheep-natural36.json"
    ))
    .unwrap();
    let mut colors = std::collections::BTreeSet::new();
    for sample in source["samples"].as_array().unwrap() {
        assert_eq!(
            u64::from(spawn::sheep_color(sample["roll"].as_f64().unwrap())),
            sample["color"].as_u64().unwrap()
        );
        colors.insert(sample["color"].as_u64().unwrap());
    }
    assert_eq!(colors.len(), 6);
    let mut world = registry_tests::tiny_world();
    world.levels[0]
        .entities
        .spawn_mob(spawn::NaturalMob::Sheep, 148, 136);
    let mut old: Value = serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    old["world"]["levels"][0]["entities"]["entities"][0]["kind"]["Mob"]
        .as_object_mut()
        .unwrap()
        .remove("sheep_color");
    let loaded = World::from_save_string(&old.to_string()).unwrap();
    let EntityKind::Mob(m) = &loaded.levels[0].entities.entities()[0].kind else {
        panic!()
    };
    assert_eq!(m.sheep_color, 15);
    old["world"]["levels"][0]["entities"]["entities"][0]["kind"]["Mob"]["sheep_color"] = 16.into();
    assert!(World::from_save_string(&old.to_string()).is_err());
}

#[test]
fn java36_boss_room_activation_preserves_entities_and_saved_player_edits() {
    let source: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/boss-room36.json"
    ))
    .unwrap();
    let size = 128;
    let mut world = World::empty_for_java_import(42, size, PlayOptions::survival(1));
    world.generation_version = GenerationVersion::Java230World;
    world.levels.push(Level {
        depth: -4,
        tiles: vec![Tile::Grass; size * size],
        data: vec![0; size * size],
        max_mob_count: 0,
        pending_spawns: Vec::new(),
        entities: EntityArena::default(),
    });
    world.signs.push(HashMap::new());
    assembly36::boss_room(&mut world.levels[0], size);
    for y in 60..=68 {
        for x in 60..=68 {
            world.levels[0].data[x + y * size] = 99;
        }
    }
    world.levels[0].tiles[69 + 64 * size] = Tile::OrnateWood;
    let chest = world.levels[0]
        .entities
        .spawn_furniture(FurnitureKind::Chest, 1064, 1032);
    world.levels[0]
        .entities
        .entities_mut_for_storage(chest)
        .unwrap()
        .add(ItemId::Wood, 7);
    world.levels[0]
        .entities
        .import_statue(1032, 1032, 1731)
        .unwrap();
    world.player.x = 1020;
    world.player.y = 1032;
    world.player.direction = Direction::Right;
    // Actual player interaction, including persistence between taps.
    for _ in 0..2 {
        world.use_target();
    }
    let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert_eq!(
        world.levels[0].data[64 + 64 * size],
        99,
        "load alone never repairs"
    );
    world.use_target();
    for cell in source["cells"].as_array().unwrap() {
        let x = cell[0].as_u64().unwrap() as usize;
        let y = cell[1].as_u64().unwrap() as usize;
        let expected = match cell[2].as_str().unwrap() {
            "OBSIDIAN BOSS FLOOR" => Tile::BossFloor,
            "OBSIDIAN BOSS WALL" => Tile::BossWall,
            "OBSIDIAN BOSS DOOR" => Tile::BossDoor,
            _ => panic!(),
        };
        assert_eq!(
            (
                world.levels[0].tiles[x + y * size],
                u64::from(world.levels[0].data[x + y * size])
            ),
            (expected, cell[3].as_u64().unwrap())
        );
    }
    assert_eq!(world.levels[0].tiles[69 + 64 * size], Tile::OrnateWood);
    assert_eq!(
        world.levels[0]
            .entities
            .death_chest_storage(chest)
            .unwrap()
            .count(ItemId::Wood),
        7
    );
    assert_eq!(
        world.levels[0]
            .entities
            .active_boss(spawn::NaturalMob::ObsidianKnight),
        Some((1731, 5000))
    );
    // A player-installed non-arena tile survives reload and explicit repair.
    world.levels[0].tiles[63 + 64 * size] = Tile::OrnateWood;
    world.levels[0].data[63 + 64 * size] = 17;
    assembly36::repair_boss_room(&mut world.levels[0], size);
    let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert_eq!(
        (
            world.levels[0].tiles[63 + 64 * size],
            world.levels[0].data[63 + 64 * size]
        ),
        (Tile::OrnateWood, 17)
    );
    // Inactive-level scheduling must neither reset the boss nor repair player work.
    world.levels.push(Level {
        depth: -3,
        tiles: vec![Tile::Grass; size * size],
        data: vec![0; size * size],
        max_mob_count: 0,
        pending_spawns: Vec::new(),
        entities: EntityArena::default(),
    });
    world.signs.push(HashMap::new());
    world.current_level = 1;
    world.tick(&Input::default());
    assert_eq!(
        world.levels[0]
            .entities
            .active_boss(spawn::NaturalMob::ObsidianKnight),
        Some((1731, 5000))
    );
    world.current_level = 0;
    let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert_eq!(
        (
            world.levels[0].tiles[63 + 64 * size],
            world.levels[0].data[63 + 64 * size]
        ),
        (Tile::OrnateWood, 17)
    );
    world.player.inventory.add_stack(ItemStack::with_water(731));
    world.player.health = 0;
    world.die_player();
    world.tick(&Input::default());
    assert!(
        !world.levels[0]
            .entities
            .has_mob(spawn::NaturalMob::ObsidianKnight)
    );
    assert!(
        world.levels[0]
            .entities
            .entities()
            .iter()
            .any(|e| e.statue_boss_health == Some(1731))
    );
    let expected_statues = source["afterRemovedPlayer"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(",");
    let actual_statues = super::migration35_world_tests::native_level_entities(&world.levels[0], 5)
        .into_iter()
        .filter(|row| row["name"] == "KnightStatue")
        .collect::<Vec<_>>();
    assert_eq!(
        actual_statues,
        super::migration35_world_tests::source_entities(&expected_statues)
    );
    assert!(
        world.player.inventory.slots().is_empty(),
        "dead player must not immediately retrieve its death chest"
    );
    assert!(world.levels[0].entities.entities().iter().any(|e| {
        e.storage
            .as_ref()
            .is_some_and(|s| s.slots().iter().any(|i| i.watering_content == Some(731)))
    }));
    assert_eq!(
        World::from_save_string(&world.to_save_string().unwrap())
            .unwrap()
            .levels[0]
            .entities
            .entities()
            .iter()
            .filter(|e| e.statue_boss_health == Some(1731))
            .count(),
        1
    );
}

#[test]
fn java36_pending_sheep_color_reaches_runtime_and_actual_renderer() {
    let assets = Assets::load(&[]).unwrap();
    let mut rendered = std::collections::BTreeSet::new();
    for color in DyeColor::ALL {
        let mut world = registry_tests::tiny_world();
        world.levels[0].pending_spawns.push(spawn::SpawnIntent {
            kind: spawn::NaturalMob::Sheep,
            x: 148,
            y: 136,
            sheep_color: color as u8,
        });
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        world.tick_level_entities(None, false, false);
        let level = &world.levels[0];
        let entity = &level.entities.entities()[0];
        let EntityKind::Mob(mob) = &entity.kind else {
            panic!()
        };
        assert_eq!(mob.sheep_color, color as u8);
        let mut screen = Screen::new();
        entity_render::render_entity(&mut screen, &assets, entity, 0, 0, &level.tiles, 16, 16, 0);
        assert!(
            screen
                .pixels()
                .contains(&(color.packed_color() & 0xff_ffff)),
            "actual sheep sprite must contain {color:?} wool pixels"
        );
        rendered.insert(screen.pixels().to_vec());
    }
    assert_eq!(rendered.len(), 16);
    let mut invalid = registry_tests::tiny_world();
    invalid.levels[0].pending_spawns.push(spawn::SpawnIntent {
        kind: spawn::NaturalMob::Cow,
        x: 148,
        y: 136,
        sheep_color: 0,
    });
    assert!(invalid.to_save_string().is_err());
}
