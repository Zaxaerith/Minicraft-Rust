use super::{tests::Owned, *};
use crate::{
    entity::{mob::NaturalMob, stats},
    experimental::infdev::{
        entities::{
            Kind, Page, Position,
            dynamic::{Animal, ItemDrop},
        },
        survival::{Action, EnemyRules, Loot, State},
    },
    item::{ItemId, ItemStack, ToolItem, ToolKind, ToolTier},
    level::noise::JavaRandom,
    world::ActiveItem,
};
fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/infdev4a/enemies.json"
    ))
    .unwrap()
}
fn put(w: &mut World, p: Page) {
    p.validate().unwrap();
    w.survival.as_mut().unwrap().entity_pages_present = true;
    w.entity_cache.insert(
        p.key,
        entities::CachedEntities {
            page: p,
            dirty: true,
            touched: 0,
        },
    );
}
fn floor(w: &mut World, x: i64, y: i64) {
    w.set_cell(
        x,
        y,
        Cell {
            ground: Ground::Dirt,
            data: 0,
        },
    )
    .unwrap();
}
#[test]
fn zombie_original_java_health_contact_and_128_seed_loot_are_exact() {
    let o = oracle();
    for d in 0..3 {
        for level in 1..=3 {
            assert_eq!(
                stats::mob_health_at_level(NaturalMob::Zombie, level, d).unwrap() as u64,
                o["healthByDifficultyLevel"][d as usize][level as usize - 1]
                    .as_u64()
                    .unwrap()
            );
        }
    }
    for row in o["contact"].as_array().unwrap() {
        let d = row["difficulty"].as_u64().unwrap() as u8;
        let l = row["level"].as_u64().unwrap() as u8;
        assert_eq!(
            10 - stats::mob_contact_damage(NaturalMob::Zombie, l, d) as u64,
            row["health"].as_u64().unwrap()
        );
        if d == 1 {
            let mut s = State::new(42);
            enemy::hurt_player(&mut s, l);
            assert_eq!(s.player.health as u64, row["health"].as_u64().unwrap());
            assert_eq!(s.player.hurt_time as u64, row["hurtTime"].as_u64().unwrap());
            let hp = s.player.health;
            enemy::hurt_player(&mut s, l);
            assert_eq!(s.player.health, hp);
        }
    }
    for row in o["normalDrops"].as_array().unwrap() {
        let mut r = JavaRandom::new(row["seed"].as_i64().unwrap());
        let loot: BTreeMap<_, _> =
            stats::mob_drops_at_level(NaturalMob::Zombie, 1, 1, false, &mut r)
                .into_iter()
                .map(|s| (s.item.display_name(), s.count))
                .collect();
        assert_eq!(serde_json::to_value(loot).unwrap(), row["loot"]);
    }
}
#[test]
fn zombie_original_underground_spawn_retains_unsupported_rolls_and_exact_levels() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Spawn", 42, 8).unwrap();
    assert!(w.survival().unwrap().enemy_rules == EnemyRules::UndergroundZombieV1);
    for depth in -3..=-1 {
        w.visit_layer(depth, 0, 0).unwrap();
        for x in -64..64 {
            for y in -64..64 {
                floor(&mut w, x, y);
            }
        }
        for row in oracle()["undergroundFirstSpawn"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["depth"] == depth)
        {
            let mut s = (**w.survival.as_ref().unwrap()).clone();
            s.random = JavaRandom::new(row["seed"].as_i64().unwrap());
            let mut pages = BTreeMap::new();
            for x in -1..=0 {
                for y in -1..=0 {
                    let k = Key::at_depth(depth, x * 64, y * 64);
                    pages.insert(k, Page::empty(k));
                }
            }
            w.spawn_enemy(&mut s, &mut pages, Position { depth, x: 0, y: 0 })
                .unwrap();
            let expected: Vec<_> = row["enemies"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["kind"] == "Zombie")
                .cloned()
                .collect();
            let actual:Vec<_>=pages.values().flat_map(|p|p.entities.iter().map(|e|{let a=e.kind.mover().unwrap();let q=p.position(e);serde_json::json!({"kind":"Zombie","level":a.mob.level,"health":a.mob.health,"x":q.x*16+i64::from(a.px),"y":q.y*16+i64::from(a.py)})})).collect();
            assert_eq!(actual, expected, "depth {depth} seed {}", row["seed"]);
        }
    }
    // No surface/night claim, no dungeon monster substitution.
    for depth in [0, -4] {
        let mut s = State::new(42);
        let k = Key::at_depth(depth, 0, 0);
        let mut p = BTreeMap::from([(k, Page::empty(k))]);
        w.spawn_enemy(&mut s, &mut p, Position { depth, x: 0, y: 0 })
            .unwrap();
        assert!(p[&k].entities.is_empty());
    }
}
#[test]
fn zombie_original_isolated_chase_and_wander_match_every_tick() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "AI", 42, 4).unwrap();
    w.visit_layer(-1, 0, 0).unwrap();
    for x in 27..58 {
        for y in 27..58 {
            floor(&mut w, x, y);
        }
    }
    let pos = Position {
        depth: -1,
        x: 32,
        y: 32,
    };
    for distance in [48, 300] {
        let mut a = Animal::of_level(NaturalMob::Zombie, 1);
        a.px = 0;
        a.py = 0;
        let mut p = Page::empty(pos.key());
        p.insert(pos, Kind::Enemy(a), None).unwrap();
        let mut pages = BTreeMap::from([(p.key, p)]);
        let mut s = State::new(42);
        s.random = JavaRandom::new(42);
        let pixel = 512 + distance;
        s.player.x = pixel % 16;
        s.player.y = pixel % 16;
        let at = Position {
            depth: -1,
            x: i64::from(pixel / 16),
            y: i64::from(pixel / 16),
        };
        for (i, row) in oracle()["walkSeed42"][distance.to_string()]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            w.tick_animals(&mut s, &mut pages, at).unwrap();
            let p = &pages[&pos.key()];
            let e = &p.entities[0];
            let a = e.kind.mover().unwrap();
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
                "distance {distance}, first tick {}",
                i + 1
            );
        }
    }
}
#[test]
fn zombie_attack_death_fault_recovery_and_real_chest_pickup_are_one_transaction() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Death", 42, 4).unwrap();
    w.visit_layer(-1, -65, -65).unwrap();
    for x in -66..=-64 {
        for y in -66..=-64 {
            floor(&mut w, x, y);
        }
    }
    let at = Position {
        depth: -1,
        x: -65,
        y: -65,
    };
    let mut a = Animal::of_level(NaturalMob::Zombie, 1);
    a.py = 2;
    a.mob.y_move = 1;
    let mut p = Page::empty(at.key());
    p.insert(at, Kind::Enemy(a), None).unwrap();
    put(&mut w, p);
    let s = w.survival.as_mut().unwrap();
    s.player.health = 1;
    s.player
        .inventory
        .add_stack(ItemStack::new(ItemId::Wood, 1));
    s.player
        .inventory
        .add_stack(ItemStack::new(ItemId::Cloth, 7));
    s.player.inventory.add_stack(ItemStack::with_water(731));
    let mut t = ToolItem::new(ToolKind::Sword, ToolTier::Wood);
    t.durability = 23;
    s.player.inventory.add_tool(t);
    s.player.active_item = Some(ActiveItem::Tool(0));
    assert!(s.player.inventory.remove(ItemId::Wood, 1));
    assert!(s.player.inventory.slot_revision() > 0);
    let owned = s.player.inventory.clone();
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    assert!(w.act_dynamic(Action::Tick).unwrap());
    assert_eq!(w.survival().unwrap().player.health, 0);
    assert!(w.survival().unwrap().death_recorded);
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    assert!(
        w.commit_with_hook(|| Err("interrupted enemy death".into()))
            .is_err()
    );
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert_eq!(w.survival().unwrap().player.health, 1);
    assert!(w.survival().unwrap().player.inventory == owned);
    assert_eq!(w.entity_page(at.key()).unwrap().entities.len(), 1);
    w.survival_action(Action::Tick).unwrap();
    w.unload_all().unwrap();
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert_eq!(w.survival().unwrap().player.health, 0);
    let p = w.entity_page(at.key()).unwrap();
    assert_eq!(
        p.entities
            .iter()
            .filter(|e| matches!(&e.kind,Kind::DeathChest(c) if c.inventory==owned))
            .count(),
        1
    );
    w.survival_action(Action::Confirm).unwrap();
    assert_eq!(w.depth(), 0);
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    // Explicit fixture travel, public movement/loot logic; no remote chest access.
    w.visit_layer(-1, at.x, at.y).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.slots(),
        owned.slots()
    );
    assert_eq!(
        w.survival().unwrap().player.inventory.tools(),
        owned.tools()
    );
    assert_eq!(
        w.survival().unwrap().player.inventory.capacity(),
        owned.capacity()
    );
    let transferred = w.survival().unwrap().player.inventory.clone();
    assert!(w.survival().unwrap().player.active_item.is_none());
    w.survival_action(Action::Tick).unwrap();
    assert!(w.survival().unwrap().player.inventory == transferred);
    w.unload_all().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let w = World::open(&dir, 1).unwrap();
    assert!(w.survival().unwrap().player.inventory == transferred);
}
#[test]
fn zombie_cross_page_knockback_full_page_and_persistence_keep_instance() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Cross", 42, 4).unwrap();
    w.visit_layer(-1, -4, 2).unwrap();
    for x in -2..=1 {
        for y in -1..=1 {
            floor(&mut w, x, y);
        }
    }
    let at = Position {
        depth: -1,
        x: -1,
        y: 0,
    };
    let from = at.key();
    let to = Key::at_depth(-1, 0, 0);
    let mut a = Animal::of_level(NaturalMob::Zombie, 3);
    a.mob.health = 73;
    a.px = 15;
    a.knock_x = 6;
    a.mob.hurt_time = 10;
    let mut p = Page::empty(from);
    p.insert(at, Kind::Enemy(a), None).unwrap();
    let before = p.entities[0].clone();
    put(&mut w, p);
    let mut p = Page::empty(to);
    for _ in 0..64 {
        p.insert(
            Position { x: 1, ..at },
            Kind::Drop(ItemDrop {
                loot: Loot::Stack(ItemStack::new(ItemId::Wood, 1)),
                delay: 1,
            }),
            None,
        )
        .unwrap();
    }
    put(&mut w, p);
    w.survival_action(Action::Tick).unwrap();
    assert!(w.entity_page(from).unwrap().entities[0] == before);
    put(&mut w, Page::empty(to));
    w.survival_action(Action::Tick).unwrap();
    assert!(w.entity_page(from).unwrap().entities.is_empty());
    let moved = w.entity_page(to).unwrap().entities[0].clone();
    assert!(before.birth == moved.birth);
    assert_eq!(moved.kind.mover().unwrap().mob.health, 73);
    assert_eq!(moved.kind.mover().unwrap().mob.level, 3);
    let dir = w.directory.clone();
    w.unload_all().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.entity_page(to).unwrap().entities[0] == moved);
}
#[test]
fn zombie_legacy_rules_stay_absent_and_sword_uses_original_material_costs() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Legacy", 42, 4).unwrap();
    let s = w.survival.as_mut().unwrap();
    s.enemy_rules = EnemyRules::None;
    let old = serde_json::to_value(&s).unwrap();
    assert!(old.get("enemy_rules").is_none());
    assert!(
        !s.recipes(Some(super::super::survival::StationKind::Workbench))
            .iter()
            .any(|r| r.product_name() == "Wood Sword")
    );
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert!(w.survival().unwrap().enemy_rules == EnemyRules::None);
    w.visit_layer(-1, 0, 0).unwrap();
    for _ in 0..100 {
        w.survival_action(Action::Tick).unwrap();
    }
    assert!(
        w.entity_cache
            .values()
            .flat_map(|p| &p.page.entities)
            .all(|e| !matches!(e.kind, Kind::Enemy(_)))
    );
    let mut s = State::new(42);
    s.player
        .inventory
        .add_stack(ItemStack::new(ItemId::Wood, 5));
    let r = s
        .recipes(Some(super::super::survival::StationKind::Workbench))
        .into_iter()
        .find(|r| r.product_name() == "Wood Sword")
        .unwrap();
    let crate::item::RecipeOutput::Tool(t) = r.output else {
        unreachable!()
    };
    assert!(
        crate::item::ToolRecipe {
            output: t,
            costs: r.costs,
            tool_cost: r.tool_cost
        }
        .craft(&mut s.player.inventory)
        .is_some()
    );
    assert_eq!(s.player.inventory.count(ItemId::Wood), 0);
}
#[test]
fn zombie_kill_full_bag_last_tool_and_corrupt_pickup_keep_exact_loot_owner() {
    let root = Owned::new();
    let mut w = World::create_survival(&root.0, "Loot", 42, 4).unwrap();
    w.visit_layer(-1, 0, 0).unwrap();
    for x in -1..=1 {
        for y in -1..=2 {
            floor(&mut w, x, y);
        }
    }
    let at = Position {
        depth: -1,
        x: 0,
        y: 1,
    };
    let mut z = Animal::of_level(NaturalMob::Zombie, 1);
    z.mob.health = 1;
    let mut p = Page::empty(at.key());
    p.insert(at, Kind::Enemy(z), None).unwrap();
    put(&mut w, p);
    let s = w.survival.as_mut().unwrap();
    s.player.direction = crate::world::Direction::Down;
    for _ in 0..26 {
        s.player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
            .unwrap();
    }
    let mut sword = ToolItem::new(ToolKind::Sword, ToolTier::Wood);
    sword.durability = 1;
    s.player.inventory.add_tool(sword).unwrap();
    s.player.active_item = Some(ActiveItem::Tool(26));
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    assert!(w.act_dynamic(Action::Attack).unwrap());
    assert!(
        w.commit_with_hook(|| Err("interrupted kill".into()))
            .is_err()
    );
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert_eq!(
        w.entity_page(at.key()).unwrap().entities[0]
            .kind
            .mover()
            .unwrap()
            .mob
            .health,
        1
    );
    assert_eq!(
        w.survival().unwrap().player.inventory.tools()[26].durability,
        1
    );
    w.survival_action(Action::Attack).unwrap();
    let before = w.entity_page(at.key()).unwrap();
    assert!(
        before
            .entities
            .iter()
            .all(|e| !matches!(e.kind, Kind::Enemy(_)))
    );
    assert_eq!(w.survival().unwrap().player.inventory.tools().len(), 26);
    let cloth: u16 = before
        .entities
        .iter()
        .map(|e| {
            if let Kind::Drop(ItemDrop {
                loot: Loot::Stack(s),
                ..
            }) = &e.kind
            {
                if s.item == ItemId::Cloth { s.count } else { 0 }
            } else {
                0
            }
        })
        .sum();
    assert!((1..=3).contains(&cloth));
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Cloth),
        cloth
    );
    let current = w.find(w.saved.entity_root, at.key()).unwrap().unwrap();
    drop(w);
    fs::write(
        objects::path(&dir, objects::Domain::Entities, current),
        b"torn enemy pickup",
    )
    .unwrap();
    let mut w = World::open(&dir, 4).unwrap();
    assert!(w.recovered_backup());
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Cloth),
        0
    );
    assert!(w.entity_page(at.key()).unwrap() == before);
    w.survival_action(Action::Tick).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Cloth),
        cloth
    );
    w.unload_all().unwrap();
    assert!(
        w.collect_garbage().unwrap_err().contains("pinned"),
        "ambiguous corruption must keep GC protection"
    );
    assert!(objects::path(&dir, objects::Domain::Entities, current).exists());
}
