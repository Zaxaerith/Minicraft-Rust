//! New PassiveV1 integration; historical PigV1 assertions remain in dynamic_tests.
use super::{tests::Owned, *};
use crate::{
    entity::mob::NaturalMob,
    experimental::infdev::{
        entities::{
            Kind, Page, Position,
            dynamic::{Animal, ItemDrop, animal_drops},
        },
        survival::{Action, EntityRules, Loot, StationKind, recipes_for},
    },
    item::{ItemId, ItemStack, ToolItem, ToolKind, ToolTier, dye::DyeColor},
    level::noise::JavaRandom,
    world::ActiveItem,
};
use std::collections::BTreeSet;
fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/infdev4a/passive-animals.json"
    ))
    .unwrap()
}
fn put(w: &mut World, page: Page) {
    page.validate().unwrap();
    w.survival.as_mut().unwrap().entity_pages_present = true;
    w.entity_cache.insert(
        page.key,
        entities::CachedEntities {
            page,
            dirty: true,
            touched: 0,
        },
    );
}
fn floor(w: &mut World, x: i64, y: i64, ground: Ground) {
    w.set_cell(x, y, Cell { ground, data: 0 }).unwrap();
}
fn loot(p: &Page) -> serde_json::Value {
    let mut map = BTreeMap::<String, u16>::new();
    for e in &p.entities {
        if let Kind::Drop(ItemDrop {
            loot: Loot::Stack(s),
            ..
        }) = &e.kind
        {
            *map.entry(if s.item == ItemId::Wool {
                "White Wool".to_owned()
            } else {
                s.item.display_name().to_owned()
            })
            .or_default() += s.count;
        }
    }
    serde_json::to_value(map).unwrap()
}
#[test]
fn passive_original_java_health_drops_and_16_actual_dye_shear_interactions() {
    let o = oracle();
    for d in 0..3 {
        for (i, species) in [NaturalMob::Cow, NaturalMob::Pig, NaturalMob::Sheep]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                crate::entity::stats::mob_health_at_level(species, 1, d).unwrap() as u64,
                o["healthCowPigSheep"][d as usize][i].as_u64().unwrap()
            );
        }
    }
    for row in o["normalDrops"].as_array().unwrap() {
        let species = if row["kind"] == "Cow" {
            NaturalMob::Cow
        } else {
            NaturalMob::Sheep
        };
        let mut a = Animal::of(species);
        a.mob.sheep_color = if species == NaturalMob::Sheep {
            DyeColor::Red as u8
        } else {
            15
        };
        a.mob.sheared = row["cut"].as_bool().unwrap_or(false);
        let mut rng = JavaRandom::new(row["seed"].as_i64().unwrap());
        let stacks = animal_drops(&a, &mut rng);
        let mut p = Page::empty(Key::at_depth(0, 0, 0));
        for s in stacks {
            p.insert(
                Position {
                    depth: 0,
                    x: 0,
                    y: 0,
                },
                Kind::Drop(ItemDrop {
                    loot: Loot::Stack(s),
                    delay: 0,
                }),
                None,
            )
            .unwrap();
        }
        assert_eq!(loot(&p), row["loot"]);
    }
    for row in o["interactions"].as_array().unwrap() {
        let color = DyeColor::from_index(row["color"].as_u64().unwrap() as u16).unwrap();
        let at = Position {
            depth: 0,
            x: 0,
            y: 0,
        };
        let mut p = Page::empty(at.key());
        p.insert(at, Kind::Animal(Animal::of(NaturalMob::Sheep)), None)
            .unwrap();
        let mut pages = BTreeMap::from([(p.key, p)]);
        let mut s = super::super::survival::State::new(42);
        s.player
            .inventory
            .add_stack(ItemStack::new(color.dye_item(), 2));
        s.player.active_item = Some(ActiveItem::Stack(color.dye_item()));
        assert!(dynamic::hit_animal(&mut s, &mut pages, at).unwrap());
        assert_eq!(
            s.player.inventory.count(color.dye_item()) as u64,
            row["dyeRemaining"].as_u64().unwrap()
        );
        assert_eq!(
            pages[&at.key()].entities[0]
                .kind
                .animal()
                .unwrap()
                .mob
                .sheep_color,
            color as u8
        );
        let mut t = ToolItem::new(ToolKind::Shears, ToolTier::Iron);
        t.durability = 2;
        s.player.inventory.add_tool(t);
        s.player.active_item = Some(ActiveItem::Tool(0));
        s.player.attack_time = 0;
        s.random = JavaRandom::new(42);
        assert!(dynamic::hit_animal(&mut s, &mut pages, at).unwrap());
        assert_eq!(
            s.player.inventory.tools()[0].durability as u64,
            row["durability"].as_u64().unwrap()
        );
        assert!(
            pages[&at.key()].entities[0]
                .kind
                .animal()
                .unwrap()
                .mob
                .sheared
        );
        assert_eq!(loot(&pages[&at.key()]), row["loot"]);
        // Repeated shearing cannot create wool; a normal subsequent hit may hurt.
        s.player.attack_time = 0;
        let before = loot(&pages[&at.key()]);
        dynamic::hit_animal(&mut s, &mut pages, at).unwrap();
        assert_eq!(loot(&pages[&at.key()]), before);
        assert!(
            s.player.inventory.tools().is_empty(),
            "last Shears use must remove the broken tool"
        );
    }
}
#[test]
fn passive_original_cow_sheep_ai_and_grazing_are_transactional() {
    let root = Owned::new();
    let mut w = passive_world(&root.0, "Walk", 42, 4).unwrap();
    w.visit(0, 0).unwrap();
    for x in 27..38 {
        for y in 27..38 {
            floor(&mut w, x, y, Ground::Dirt);
        }
    }
    let at = Position {
        depth: 0,
        x: 32,
        y: 32,
    };
    let player = Position {
        depth: 0,
        x: 0,
        y: 0,
    };
    let o = oracle();
    for (species, name) in [(NaturalMob::Cow, "Cow"), (NaturalMob::Sheep, "Sheep")] {
        let mut a = Animal::of(species);
        a.px = 0;
        a.py = 0;
        let mut p = Page::empty(at.key());
        p.insert(at, Kind::Animal(a), None).unwrap();
        let mut pages = BTreeMap::from([(p.key, p)]);
        let mut s = (**w.survival.as_ref().unwrap()).clone();
        s.random = JavaRandom::new(42);
        for (i, row) in o["dirtWalkSeed42"][name]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            assert!(
                w.tick_animals(&mut s, &mut pages, player)
                    .unwrap()
                    .is_empty()
            );
            let p = &pages[&at.key()];
            let e = &p.entities[0];
            let a = e.kind.animal().unwrap();
            let q = p.position(e);
            assert_eq!(
                serde_json::json!([
                    q.x * 16 + i64::from(a.px),
                    q.y * 16 + i64::from(a.py),
                    a.mob.x_move,
                    a.mob.y_move,
                    a.mob.walk_time,
                    a.mob.walk_distance
                ]),
                *row,
                "{name} first difference tick {}",
                i + 1
            );
        }
    }
    for row in o["grazing"].as_array().unwrap() {
        let species = if row["kind"] == "Cow" {
            NaturalMob::Cow
        } else {
            NaturalMob::Sheep
        };
        let mut a = Animal::of(species);
        a.mob.sheared = species == NaturalMob::Sheep;
        w.visit_layer(-1, 0, 0).unwrap();
        floor(&mut w, 32, 32, Ground::Grass);
        let at = Position { depth: -1, ..at };
        let mut p = Page::empty(at.key());
        p.insert(at, Kind::Animal(a), None).unwrap();
        put(&mut w, p);
        w.survival.as_mut().unwrap().random = JavaRandom::new(row["seed"].as_i64().unwrap());
        w.checkpoint().unwrap();
        let dir = w.directory.clone();
        assert!(w.act_dynamic(Action::Tick).unwrap());
        assert_eq!(w.cell(32, 32).unwrap().ground, Ground::Dirt);
        assert!(
            !w.entity_page(at.key()).unwrap().entities[0]
                .kind
                .animal()
                .unwrap()
                .mob
                .sheared
        );
        assert!(
            w.commit_with_hook(|| Err("power loss during graze".into()))
                .is_err()
        );
        drop(w);
        w = World::open(&dir, 4).unwrap();
        assert_eq!(w.cell(32, 32).unwrap().ground, Ground::Grass);
        assert_eq!(
            w.entity_page(at.key()).unwrap().entities[0]
                .kind
                .animal()
                .unwrap()
                .mob
                .sheared,
            species == NaturalMob::Sheep
        );
        w.survival_action(Action::Tick).unwrap();
        w.unload_all().unwrap();
        w.collect_garbage().unwrap();
        drop(w);
        w = World::open(&dir, 4).unwrap();
        assert_eq!(w.cell(32, 32).unwrap().ground, Ground::Dirt);
        assert!(
            !w.entity_page(at.key()).unwrap().entities[0]
                .kind
                .animal()
                .unwrap()
                .mob
                .sheared
        );
    }
}
#[test]
fn passive_natural_species_share_bounded_pages_and_restart_without_respawn() {
    let root = Owned::new();
    let mut w = passive_world(&root.0, "Natural", 42, 16).unwrap();
    assert!(w.survival().unwrap().entity_rules == EntityRules::PassiveV1);
    // Real generated habitat, no injected terrain or animals.
    for x in -1..=1 {
        for y in -1..=1 {
            w.cell(x * 64, y * 64).unwrap();
        }
    }
    for _ in 0..180 {
        w.survival_action(Action::Tick).unwrap();
    }
    let mut species = BTreeSet::new();
    let keys: Vec<_> = w.resident.keys().copied().collect();
    let mut pages = Vec::new();
    for key in keys {
        let p = w.entity_page(key).unwrap();
        for e in &p.entities {
            if let Some(a) = e.kind.animal() {
                species.insert(a.mob.species as u8);
                assert!(e.birth.is_some());
            }
        }
        pages.push((key, p));
    }
    assert!(species.contains(&(NaturalMob::Cow as u8)));
    assert!(species.contains(&(NaturalMob::Pig as u8)));
    assert!(species.contains(&(NaturalMob::Sheep as u8)));
    let dir = w.directory.clone();
    w.unload_all().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    for (key, p) in pages {
        assert!(w.entity_page(key).unwrap() == p);
    }
    assert!(w.survival().unwrap().entity_rules == EntityRules::PassiveV1);
}
#[test]
fn passive_body_collision_extremes_despawn_and_unloaded_lifetime_are_bounded() {
    let at = Position {
        depth: 0,
        x: -1,
        y: 0,
    };
    let mut p = Page::empty(at.key());
    let mut a = Animal::of(NaturalMob::Cow);
    a.px = 15;
    p.insert(at, Kind::Animal(a.clone()), None).unwrap();
    let pages = BTreeMap::from([(p.key, p)]);
    assert!(passive::blocked(
        &pages,
        None,
        Position { x: 0, ..at },
        0,
        8,
        None
    ));
    assert!(!passive::blocked(
        &pages,
        None,
        Position {
            x: 0,
            depth: -1,
            ..at
        },
        0,
        8,
        None
    ));
    assert!(!passive::blocked(
        &pages,
        None,
        Position {
            x: i64::MAX,
            y: i64::MIN,
            ..at
        },
        0,
        0,
        None
    ));
    let state = super::super::survival::State::new(42);
    let player = Position {
        depth: 0,
        x: 0,
        y: 0,
    };
    let mut rng = JavaRandom::new(42);
    a.age = 18001;
    assert!(passive::despawn(
        &mut a,
        Position { x: 32, ..at },
        player,
        &state.player,
        &mut rng
    ));
    a.age = 1;
    assert!(passive::despawn(
        &mut a,
        Position { x: i64::MAX, ..at },
        player,
        &state.player,
        &mut rng
    ));
    assert!(!passive::despawn(
        &mut a,
        player,
        player,
        &state.player,
        &mut rng
    ));
    assert_eq!(a.idle, 0);
    let o = oracle();
    assert_eq!(o["originalFarRemoved"], false);
    assert_eq!(o["expiredAwayRemoved"], true);
    // Original Java suppresses recoil while hurt; new-rule visible recoil is an explicit fix.
    assert!(
        o["originalHurtMovement"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r[0] == 512 && r[1] == 512)
    );
}
#[test]
fn passive_cross_page_knockback_full_destination_keeps_identity_and_color() {
    let root = Owned::new();
    let mut w = passive_world(&root.0, "Cross", 42, 4).unwrap();
    w.visit_layer(-1, -4, 2).unwrap();
    for x in -2..=2 {
        for y in -1..=1 {
            floor(&mut w, x, y, Ground::Grass);
        }
    }
    let from = Key::at_depth(-1, -1, 0);
    let to = Key::at_depth(-1, 0, 0);
    let pos = Position {
        depth: -1,
        x: -1,
        y: 0,
    };
    let mut a = Animal::of(NaturalMob::Sheep);
    a.px = 15;
    a.mob.sheep_color = DyeColor::Pink as u8;
    a.mob.sheared = true;
    a.knock_x = 6;
    a.mob.hurt_time = 10;
    let mut p = Page::empty(from);
    p.insert(pos, Kind::Animal(a), None).unwrap();
    let original = p.entities[0].clone();
    put(&mut w, p);
    let mut full = Page::empty(to);
    for _ in 0..64 {
        full.insert(
            Position { x: 1, ..pos },
            Kind::Drop(ItemDrop {
                loot: Loot::Stack(ItemStack::new(ItemId::Wood, 1)),
                delay: 1,
            }),
            None,
        )
        .unwrap();
    }
    put(&mut w, full);
    w.survival.as_mut().unwrap().random = JavaRandom::new(329);
    w.checkpoint().unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.cell(0, 0).unwrap().ground,
        Ground::Grass,
        "rejected migration cannot commit grazing alone"
    );
    assert!(w.entity_page(from).unwrap().entities[0] == original);
    put(&mut w, Page::empty(to));
    w.survival_action(Action::Tick).unwrap();
    assert!(w.entity_page(from).unwrap().entities.is_empty());
    let moved = w.entity_page(to).unwrap().entities[0].clone();
    assert!(moved.birth == original.birth);
    let a = moved.kind.animal().unwrap();
    assert_eq!(a.mob.sheep_color, DyeColor::Pink as u8);
    assert!(a.mob.sheared);
    assert_eq!(a.knock_x, 5);
    let dir = w.directory.clone();
    w.unload_all().unwrap();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.entity_page(to).unwrap().entities[0] == moved);
}
#[test]
fn passive_real_shears_and_dye_recipes_keep_legacy_menu_and_original_costs() {
    for b in [None, Some(StationKind::Workbench), Some(StationKind::Anvil)] {
        assert_eq!(
            recipes_for(EntityRules::PigV1, b).len(),
            super::super::survival::recipes(b).len()
        );
    }
    let recipes = recipes_for(EntityRules::PassiveV1, Some(StationKind::Anvil));
    let r = recipes
        .iter()
        .find(|r| r.product_name() == "Shears")
        .unwrap();
    let mut inv = crate::item::Inventory::new(27);
    inv.add_stack(ItemStack::new(ItemId::IronIngot, 4));
    let crate::item::crafting::RecipeOutput::Tool(t) = r.output else {
        panic!("Shears recipe is not a tool")
    };
    let recipe = crate::item::ToolRecipe {
        output: t,
        costs: r.costs,
        tool_cost: r.tool_cost,
    };
    assert!(recipe.craft(&mut inv).is_some());
    assert_eq!(inv.count(ItemId::IronIngot), 0);
    assert_eq!(inv.tools()[0].kind, ToolKind::Shears);
    assert!(recipe.craft(&mut inv).is_none());
    assert_eq!(
        recipes_for(EntityRules::PassiveV1, Some(StationKind::DyeVat)).len(),
        35
    );
    assert!(
        recipes_for(EntityRules::PassiveV1, Some(StationKind::Workbench))
            .iter()
            .any(|r| r.product_name() == "Dye Vat")
    );
}

#[test]
fn passive_shear_full_inventory_interruption_and_pickup_keep_single_wool_owner() {
    let root = Owned::new();
    let mut w = passive_world(&root.0, "FullBag", 42, 4).unwrap();
    w.visit_layer(-1, 0, 0).unwrap();
    for x in -1..=1 {
        for y in -1..=2 {
            floor(&mut w, x, y, Ground::Dirt);
        }
    }
    let at = Position {
        depth: -1,
        x: 0,
        y: 1,
    };
    let mut a = Animal::of(NaturalMob::Sheep);
    a.mob.sheep_color = DyeColor::Pink as u8;
    let mut p = Page::empty(at.key());
    p.insert(at, Kind::Animal(a), None).unwrap();
    put(&mut w, p);
    let state = w.survival.as_mut().unwrap();
    state.player.direction = crate::world::Direction::Down;
    for _ in 0..26 {
        state
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
            .unwrap();
    }
    let mut shears = ToolItem::new(ToolKind::Shears, ToolTier::Iron);
    shears.durability = 2;
    state.player.inventory.add_tool(shears).unwrap();
    state.player.active_item = Some(ActiveItem::Tool(26));
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    assert!(w.act_dynamic(Action::Attack).unwrap());
    assert!(
        w.commit_with_hook(|| Err("interrupted shearing".into()))
            .is_err()
    );
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert!(
        !w.entity_page(at.key()).unwrap().entities[0]
            .kind
            .animal()
            .unwrap()
            .mob
            .sheared
    );
    assert_eq!(
        w.survival().unwrap().player.inventory.tools()[26].durability,
        2
    );
    w.survival_action(Action::Attack).unwrap();
    let p = w.entity_page(at.key()).unwrap();
    assert!(p.entities[0].kind.animal().unwrap().mob.sheared);
    let expected = loot(&p)["Pink Wool"].as_u64().unwrap();
    assert!((1..=3).contains(&expected));
    assert_eq!(
        w.survival().unwrap().player.inventory.tools()[26].durability,
        1
    );
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::PinkWool),
        0
    );
    assert_eq!(
        loot(&w.entity_page(at.key()).unwrap())["Pink Wool"]
            .as_u64()
            .unwrap(),
        expected
    );
    assert!(w.animal_blocks_player(-1, 0, 1, 8, 8).unwrap());
    let state = w.survival.as_mut().unwrap();
    state.player.inventory.take_tool(0);
    state.player.active_item = None;
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::PinkWool) as u64,
        expected
    );
    assert_eq!(w.entity_page(at.key()).unwrap().entities.len(), 1);
    w.survival_action(Action::Tick).unwrap();
    w.unload_all().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::PinkWool) as u64,
        expected
    );
    assert!(
        w.entity_page(at.key()).unwrap().entities[0]
            .kind
            .animal()
            .unwrap()
            .mob
            .sheared
    );
}

#[test]
fn passive_original_java_try_spawn_32_seed_first_candidates_and_colors_match() {
    let root = Owned::new();
    let mut w = passive_world(&root.0, "SpawnOracle", 42, 8).unwrap();
    w.visit(0, 0).unwrap();
    for x in -64..64 {
        for y in -64..64 {
            floor(&mut w, x, y, Ground::Grass);
        }
    }
    for row in oracle()["dayGrassFirstSpawn"].as_array().unwrap() {
        let mut state = (**w.survival.as_ref().unwrap()).clone();
        state.random = JavaRandom::new(row["seed"].as_i64().unwrap());
        let mut pages = BTreeMap::new();
        for x in -1..=0 {
            for y in -1..=0 {
                let key = Key::at_depth(0, x * 64, y * 64);
                pages.insert(key, Page::empty(key));
            }
        }
        w.spawn_animal(
            &mut state,
            &mut pages,
            Position {
                depth: 0,
                x: 0,
                y: 0,
            },
        )
        .unwrap();
        let mut actual = Vec::new();
        for p in pages.values() {
            for e in &p.entities {
                let a = e.kind.animal().unwrap();
                let pos = p.position(e);
                actual.push(serde_json::json!({"kind":format!("{:?}",a.mob.species),"x":pos.x*16+i64::from(a.px),"y":pos.y*16+i64::from(a.py),"color":a.mob.sheep_color}));
            }
        }
        assert_eq!(
            serde_json::json!(actual),
            row["animals"],
            "first original trySpawn difference at seed {}",
            row["seed"]
        );
    }
}

// Preserve the exact historical v4 fixture rule, independent of new-world defaults.
fn passive_world(
    root: &std::path::Path,
    name: &str,
    seed: i64,
    capacity: usize,
) -> Result<World, String> {
    let mut w = World::create_survival(root, name, seed, capacity)?;
    w.survival.as_mut().unwrap().environment = None;
    w.survival.as_mut().unwrap().enemy_rules =
        crate::experimental::infdev::survival::EnemyRules::None;
    w.checkpoint()?;
    Ok(w)
}
