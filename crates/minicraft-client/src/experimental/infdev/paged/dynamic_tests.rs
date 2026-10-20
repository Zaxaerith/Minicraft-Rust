use super::tests::Owned;
use super::*;
use crate::{
    experimental::infdev::{
        entities::{
            Kind, Page, Position,
            dynamic::{ItemDrop, Pig},
        },
        survival::{Action, EntityRules, Loot},
    },
    item::{ItemId, ItemStack, ToolItem, ToolKind, ToolTier},
};

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
fn grass(w: &mut World, x: i64, y: i64) {
    w.set_cell(
        x,
        y,
        Cell {
            ground: Ground::Grass,
            data: 0,
        },
    )
    .unwrap();
}
fn crossing(root: &Owned) -> (World, Key, Key) {
    let mut w = World::create_survival(&root.0, "Dynamic", 42, 4).unwrap();
    w.visit_layer(-1, -4, 2).unwrap();
    for x in -2..=1 {
        grass(&mut w, x, 0);
    }
    let from = Key::at_depth(-1, -1, 0);
    let to = Key::at_depth(-1, 0, 0);
    let mut page = Page::empty(from);
    let mut pig = Pig::new();
    pig.px = 15;
    pig.mob.x_move = 1;
    page.insert(
        Position {
            depth: -1,
            x: -1,
            y: 0,
        },
        Kind::Pig(pig),
        None,
    )
    .unwrap();
    put(&mut w, page);
    w.checkpoint().unwrap();
    (w, from, to)
}
#[test]
fn dynamic_cross_chunk_identity_publication_failure_and_gc_are_atomic() {
    let root = Owned::new();
    let (mut w, from, to) = crossing(&root);
    let original = w.entity_page(from).unwrap().entities[0].clone();
    let directory = w.directory.clone();
    assert!(w.act_dynamic(Action::Tick).unwrap());
    assert!(w.entity_page(from).unwrap().entities.is_empty());
    let moved = w
        .entity_page(to)
        .unwrap()
        .entities
        .iter()
        .find(|e| e.birth == original.birth)
        .unwrap()
        .clone();
    assert_eq!(moved.birth, original.birth);
    assert_eq!(moved.x, 0);
    assert!(
        w.commit_with_hook(|| Err("interrupted migration".into()))
            .is_err()
    );
    assert!(w.collect_garbage().is_err());
    drop(w);
    let mut w = World::open(&directory, 1).unwrap();
    assert!(w.entity_page(from).unwrap().entities.contains(&original));
    assert!(
        w.entity_page(to)
            .unwrap()
            .entities
            .iter()
            .all(|e| e.birth != original.birth)
    );
    // With both chunks loaded, actual public action commits both owners together.
    w.capacity = 4;
    w.cell(0, 0).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert!(w.entity_page(from).unwrap().entities.is_empty());
    w.unload_all().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let mut w = World::open(&directory, 1).unwrap();
    assert!(w.entity_page(from).unwrap().entities.is_empty());
    assert_eq!(
        w.entity_page(to)
            .unwrap()
            .entities
            .iter()
            .filter(|e| e.birth == original.birth)
            .count(),
        1
    );
}
#[test]
fn dynamic_full_destination_and_unknown_pig_state_never_delete_animal() {
    let root = Owned::new();
    let (mut w, from, to) = crossing(&root);
    let original = w.entity_page(from).unwrap();
    let mut full = Page::empty(to);
    for _ in 0..64 {
        full.insert(
            Position {
                depth: -1,
                x: 0,
                y: 0,
            },
            Kind::Drop(ItemDrop {
                loot: Loot::Stack(ItemStack::new(ItemId::Wood, 1)),
                delay: 0,
            }),
            None,
        )
        .unwrap();
    }
    put(&mut w, full);
    w.checkpoint().unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert!(w.entity_page(from).unwrap() == original);
    assert_eq!(w.entity_page(to).unwrap().entities.len(), 64);
    let mut bad = original.clone();
    let Kind::Pig(pig) = &mut bad.entities[0].kind else {
        unreachable!()
    };
    pig.mob.health = 9;
    assert!(bad.validate().is_err());
    bad.entities[0].birth = None;
    assert!(bad.validate().is_err());
}
#[test]
fn dynamic_paged_drops_exceed_session_limit_and_preserve_tool_water_across_layers() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "ManyDrops", 42, 1).unwrap();
    let mut keys = Vec::new();
    for index in 0..17 {
        let at = Position {
            depth: -(index % 5) as i8,
            x: (i64::from(index) - 8) * 64,
            y: -65,
        };
        let mut page = Page::empty(at.key());
        for _ in 0..40 {
            page.insert(
                at,
                Kind::Drop(ItemDrop {
                    loot: Loot::Stack(ItemStack::new(ItemId::Wood, 1)),
                    delay: 0,
                }),
                None,
            )
            .unwrap();
        }
        put(&mut w, page);
        keys.push(at.key());
        w.checkpoint().unwrap();
        w.unload_all().unwrap();
    }
    let mut page = Page::empty(Key::at_depth(-1, -65, -129));
    let at = Position {
        depth: -1,
        x: -65,
        y: -129,
    };
    let mut tool = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
    tool.durability = 23;
    for loot in [
        Loot::Stack(ItemStack::with_water(731)),
        Loot::Stack(ItemStack::with_water(23)),
        Loot::Tool(tool),
    ] {
        page.insert(at, Kind::Drop(ItemDrop { loot, delay: 0 }), None)
            .unwrap();
    }
    put(&mut w, page);
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    let mut count = 0;
    for key in keys {
        count += w.entity_page(key).unwrap().entities.len();
    }
    assert_eq!(count, 680);
    w.visit_layer(-1, -65, -129).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory.tools(), &[tool]);
    let water: Vec<_> = w
        .survival()
        .unwrap()
        .player
        .inventory
        .slots()
        .iter()
        .map(|s| s.watering_content)
        .collect();
    assert_eq!(water, vec![Some(731), Some(23)]);
    let inventory = w.survival().unwrap().player.inventory.clone();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, inventory);
    w.unload_all().unwrap();
    drop(w);
    let w = World::open(&dir, 1).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, inventory);
}
#[test]
fn dynamic_actual_hunting_drop_pickup_food_and_disk_preserve_last_tool_use() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Hunt", 42, 4).unwrap();
    w.visit(10, 10).unwrap();
    grass(&mut w, 10, 10);
    grass(&mut w, 10, 11);
    let at = Position {
        depth: 0,
        x: 10,
        y: 11,
    };
    let mut page = Page::empty(at.key());
    page.insert(at, Kind::Pig(Pig::new()), None).unwrap();
    put(&mut w, page);
    let mut tool = ToolItem::new(ToolKind::Axe, ToolTier::Gem);
    tool.durability = 1;
    let state = w.survival.as_mut().unwrap();
    state.player.inventory.add_tool(tool);
    state.player.active_item = Some(crate::world::ActiveItem::Tool(0));
    w.survival_action(Action::Attack).unwrap();
    assert!(w.survival().unwrap().player.inventory.tools().is_empty());
    assert!(
        w.entity_page(at.key())
            .unwrap()
            .entities
            .iter()
            .all(|e| !matches!(e.kind, Kind::Pig(_)))
    );
    w.survival_action(Action::Tick).unwrap();
    let n = w
        .survival()
        .unwrap()
        .player
        .inventory
        .count(ItemId::RawPork);
    assert!((1..=2).contains(&n));
    let state = w.survival.as_mut().unwrap();
    state.player.hunger = 5;
    state.player.attack_time = 0;
    state.player.active_item = Some(crate::world::ActiveItem::Stack(ItemId::RawPork));
    w.survival_action(Action::Attack).unwrap();
    assert_eq!(w.survival().unwrap().player.hunger, 6);
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::RawPork),
        n - 1
    );
    w.unload_all().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let w = World::open(&dir, 1).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::RawPork),
        n - 1
    );
    assert_eq!(w.survival().unwrap().player.hunger, 6);
}
#[test]
fn dynamic_natural_pigs_spawn_only_on_loaded_surface_and_legacy_marker_stays_absent() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Natural", 42, 4).unwrap();
    assert!(w.survival().unwrap().entity_rules == EntityRules::PigV1);
    for _ in 0..180 {
        w.survival_action(Action::Tick).unwrap();
    }
    let keys: Vec<_> = w.resident.keys().copied().collect();
    let mut n = 0;
    for key in keys {
        for e in w.entity_page(key).unwrap().entities {
            if matches!(e.kind, Kind::Pig(_)) {
                let page = Page::empty(key);
                let p = page.position(&e);
                let c = w.cell_at(p.depth, p.x, p.y).unwrap();
                assert!(matches!(c.ground, Ground::Grass | Ground::Flower));
                n += 1;
            }
        }
    }
    assert!(
        n > 0,
        "natural Seed42 must produce pigs without injected floor/entities"
    );
    // An actual historical empty state has no entity root or new marker.
    let mut old = World::create_survival(&root.0, "Old", 42, 1).unwrap();
    old.survival.as_mut().unwrap().entity_rules = EntityRules::Legacy;
    old.checkpoint().unwrap();
    let bytes = objects::read(
        &old.directory,
        objects::Domain::Sessions,
        old.saved.session.unwrap(),
    )
    .unwrap();
    assert!(!String::from_utf8(bytes).unwrap().contains("entity_rules"));
    let dir = old.directory.clone();
    drop(old);
    let mut old = World::open(&dir, 1).unwrap();
    for _ in 0..180 {
        old.survival_action(Action::Tick).unwrap();
    }
    assert!(old.saved.entity_root.is_none());
}
#[test]
fn dynamic_original_java_pig_health_drops_and_120_ai_ticks_match() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/infdev4a/animals.json"
    ))
    .unwrap();
    for d in 0..3 {
        assert_eq!(
            crate::entity::stats::mob_health_at_level(crate::entity::mob::NaturalMob::Pig, 1, d)
                .unwrap() as u64,
            oracle["healthByDifficulty"][d as usize].as_u64().unwrap()
        );
    }
    for row in oracle["normalDrops"].as_array().unwrap() {
        let mut r = crate::level::noise::JavaRandom::new(row["seed"].as_i64().unwrap());
        assert_eq!(
            crate::experimental::infdev::entities::dynamic::pig_drop(&mut r).count as u64,
            row["pork"].as_u64().unwrap()
        );
    }
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "AI", 42, 4).unwrap();
    w.visit(0, 0).unwrap();
    for y in 28..36 {
        for x in 28..36 {
            grass(&mut w, x, y);
        }
    }
    let at = Position {
        depth: 0,
        x: 32,
        y: 32,
    };
    let mut page = Page::empty(at.key());
    let mut p = Pig::new();
    p.px = 0;
    p.py = 0;
    page.insert(at, Kind::Pig(p), None).unwrap();
    let mut pages = BTreeMap::from([(page.key, page)]);
    let mut s = (**w.survival.as_ref().unwrap()).clone();
    s.random = crate::level::noise::JavaRandom::new(42);
    for (tick, row) in oracle["walkSeed42"].as_array().unwrap().iter().enumerate() {
        w.tick_pigs(
            &mut s,
            &mut pages,
            Position {
                depth: 0,
                x: 0,
                y: 0,
            },
        )
        .unwrap();
        let page = &pages[&at.key()];
        let e = &page.entities[0];
        let Kind::Pig(p) = &e.kind else {
            unreachable!()
        };
        let q = page.position(e);
        let observed = serde_json::json!([
            q.x * 16 + i64::from(p.px),
            q.y * 16 + i64::from(p.py),
            p.mob.x_move,
            p.mob.y_move,
            p.mob.walk_time,
            p.mob.walk_distance
        ]);
        assert_eq!(
            &observed,
            row,
            "first Java AI difference at tick {}",
            tick + 1
        );
    }
}
#[test]
fn dynamic_full_bag_partial_pickup_and_interrupted_retrieval_keep_one_owner() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Partial", 42, 4).unwrap();
    w.visit_layer(-1, -65, -129).unwrap();
    let at = Position {
        depth: -1,
        x: -65,
        y: -129,
    };
    let mut page = Page::empty(at.key());
    let mut t = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
    t.durability = 23;
    for loot in [
        Loot::Stack(ItemStack::new(ItemId::Wood, 10)),
        Loot::Stack(ItemStack::with_water(731)),
        Loot::Tool(t),
    ] {
        page.insert(at, Kind::Drop(ItemDrop { loot, delay: 0 }), None)
            .unwrap();
    }
    put(&mut w, page);
    for _ in 0..27 {
        w.survival
            .as_mut()
            .unwrap()
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
            .unwrap();
    }
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    let complete = fs::read(dir.join("checkpoint.json")).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.entity_page(at.key()).unwrap().entities.len(), 3);
    w.survival.as_mut().unwrap().player.inventory.take_tool(0);
    assert!(w.act_dynamic(Action::Tick).unwrap());
    assert_eq!(w.entity_page(at.key()).unwrap().entities.len(), 2);
    assert!(
        w.commit_with_hook(|| Err("interrupted pickup".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), complete);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.entity_page(at.key()).unwrap().entities.len(), 3);
    assert_eq!(w.survival().unwrap().player.inventory.tools().len(), 27);
    w.survival.as_mut().unwrap().player.inventory = crate::item::Inventory::new(27);
    w.survival_action(Action::Tick).unwrap();
    let inventory = w.survival().unwrap().player.inventory.clone();
    assert_eq!(inventory.count(ItemId::Wood), 10);
    assert_eq!(inventory.tools(), &[t]);
    assert!(
        inventory
            .slots()
            .iter()
            .any(|s| s.watering_content == Some(731))
    );
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, inventory);
    assert!(w.entity_page(at.key()).unwrap().entities.is_empty());
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
}
#[test]
fn dynamic_new_rule_death_with_animals_drops_and_corrupt_pickup_recovers_whole_inventory() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "AnimalDeath", 42, 4).unwrap();
    w.visit_layer(-1, -65, -65).unwrap();
    w.set_cell(
        -65,
        -65,
        Cell {
            ground: Ground::Dirt,
            data: 0,
        },
    )
    .unwrap();
    let at = Position {
        depth: -1,
        x: -65,
        y: -65,
    };
    let other = Position { x: -75, ..at };
    let mut page = Page::empty(at.key());
    page.insert(other, Kind::Pig(Pig::new()), None).unwrap();
    page.insert(
        other,
        Kind::Drop(ItemDrop {
            loot: Loot::Stack(ItemStack::new(ItemId::Wood, 7)),
            delay: 0,
        }),
        None,
    )
    .unwrap();
    put(&mut w, page);
    let state = w.survival.as_mut().unwrap();
    state.player.inventory.add_stack(ItemStack::with_water(731));
    state.player.inventory.add_stack(ItemStack::with_water(23));
    let mut tool = ToolItem::new(ToolKind::Axe, ToolTier::Iron);
    tool.durability = 23;
    state.player.inventory.add_tool(tool);
    state.player.active_item = Some(crate::world::ActiveItem::Tool(0));
    let owned = state.player.inventory.clone();
    w.survival_action(Action::DiagnosticDeath).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    let page = w.entity_page(at.key()).unwrap();
    assert_eq!(page.entities.len(), 3);
    let Kind::DeathChest(c) = &page
        .entities
        .iter()
        .find(|e| matches!(e.kind, Kind::DeathChest(_)))
        .unwrap()
        .kind
    else {
        unreachable!()
    };
    assert_eq!(c.inventory, owned);
    w.survival_action(Action::Confirm).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    w.visit_layer(-1, at.x, at.y).unwrap();
    w.checkpoint().unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, owned);
    let current = w.find(w.saved.entity_root, at.key()).unwrap().unwrap();
    let dir = w.directory.clone();
    drop(w);
    fs::write(
        objects::path(&dir, objects::Domain::Entities, current),
        b"torn post-pickup page",
    )
    .unwrap();
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.recovered_backup());
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    let page = w.entity_page(at.key()).unwrap();
    assert_eq!(page.entities.len(), 3);
    let Kind::DeathChest(c) = &page
        .entities
        .iter()
        .find(|e| matches!(e.kind, Kind::DeathChest(_)))
        .unwrap()
        .kind
    else {
        unreachable!()
    };
    assert_eq!(c.inventory, owned);
    assert!(w.collect_garbage().is_err());
}
