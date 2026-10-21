use super::{tests::Owned, *};
use crate::{
    entity::{mob::NaturalMob, stats},
    experimental::infdev::{
        entities::{Kind, Page, Position, dynamic::Animal},
        environment::Environment,
        survival::{Action, State},
    },
    item::{ItemId, ItemStack},
    level::noise::JavaRandom,
    world::{ActiveItem, Direction},
};
fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/infdev4a/environment.json"
    ))
    .unwrap()
}
fn floor(w: &mut World, x: i64, y: i64) {
    w.set_cell(
        x,
        y,
        Cell {
            ground: Ground::Dirt,
            data: 731,
        },
    )
    .unwrap();
}

#[test]
fn calendar_uses_shared_clock_matches_java_boundaries_pauses_and_restores() {
    let rows = oracle();
    for row in rows["times"].as_array().unwrap() {
        let e = Environment {
            day_tick: row["tick"].as_u64().unwrap() as u32,
            past_day_one: row["pastDayOne"].as_bool().unwrap(),
            ..Environment::new()
        };
        e.validate().unwrap();
        assert_eq!(
            u64::from(e.day_tick / 16200),
            row["phase"].as_u64().unwrap()
        );
    }
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Time", 42, 1).unwrap();
    {
        let e = w.survival.as_mut().unwrap().environment.as_mut().unwrap();
        e.day_tick = 64799;
        e.game_time = i32::MAX;
    }
    w.survival_action(Action::Crafting).unwrap();
    let paused = w.survival().unwrap().environment.clone();
    for _ in 0..100 {
        w.survival_action(Action::Tick).unwrap();
    }
    assert_eq!(w.survival().unwrap().environment, paused);
    w.survival_action(Action::Close).unwrap();
    w.survival_action(Action::Tick).unwrap();
    let env = w.survival().unwrap().environment.as_ref().unwrap();
    assert_eq!(
        (env.day_tick, env.game_time, env.past_day_one),
        (0, i32::MIN, true)
    );
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    let env = w.survival().unwrap().environment.clone();
    assert_eq!(env.as_ref().unwrap().day_tick, 0);
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival().unwrap().environment.as_ref().unwrap().day_tick,
        1
    );
    let mut legacy = serde_json::to_value(State::new(42)).unwrap();
    legacy.as_object_mut().unwrap().remove("environment");
    let s: State = serde_json::from_value(legacy.clone()).unwrap();
    assert!(s.environment.is_none());
    assert_eq!(serde_json::to_value(s).unwrap(), legacy);
    legacy["environment"] =
        serde_json::json!({"version":"Future","day_tick":0,"game_time":0,"past_day_one":false});
    assert!(serde_json::from_value::<State>(legacy).is_err());
}

#[test]
fn original_torch_square_and_explicit_negative_coordinate_extension_cross_pages() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Light", 42, 1).unwrap();
    for row in oracle()["torches"].as_array().unwrap() {
        let origin = row["origin"].as_i64().unwrap();
        let dx = row["dx"].as_i64().unwrap();
        let dy = row["dy"].as_i64().unwrap();
        let torch = Position {
            depth: -1,
            x: origin,
            y: origin,
        };
        let mut page = Page::empty(torch.key());
        page.insert(torch, Kind::Torch, None).unwrap();
        let pages = BTreeMap::from([(page.key, page)]);
        let actual = w
            .torch_lit(
                Position {
                    depth: -1,
                    x: origin + dx,
                    y: origin + dy,
                },
                &pages,
            )
            .unwrap();
        if origin >= 0 {
            assert_eq!(actual, row["lit"].as_bool().unwrap());
        } else {
            assert!(
                !row["lit"].as_bool().unwrap(),
                "original finite getAreaTilePositions clips negative tiles"
            );
            assert_eq!(actual, dx.abs() <= 3 && dy.abs() <= 3);
        }
        assert!(
            !w.torch_lit(
                Position {
                    depth: -2,
                    x: origin + dx,
                    y: origin + dy
                },
                &pages
            )
            .unwrap()
        );
    }
    // Original Lantern evidence includes a unit mismatch, not an invented repair.
    assert!(
        oracle()["lanterns"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| !r["tileCall"].as_bool().unwrap())
    );
}

#[test]
fn torch_real_recipe_publication_fault_floor_preservation_and_gc_keep_one_owner() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Torch", 42, 1).unwrap();
    w.visit_layer(-1, -65, -66).unwrap();
    floor(&mut w, -65, -66);
    floor(&mut w, -65, -65);
    let s = w.survival.as_mut().unwrap();
    s.player.direction = Direction::Down;
    s.player.inventory.add(ItemId::Wood, 1);
    s.player.inventory.add(ItemId::Coal, 1);
    s.player.inventory.add_stack(ItemStack::with_water(731));
    s.panel = super::super::survival::Panel::Crafting(None);
    s.selection = s
        .recipes(None)
        .iter()
        .position(|r| r.product_name() == "Torch")
        .unwrap();
    w.survival_action(Action::Confirm).unwrap();
    w.survival_action(Action::Close).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Torch),
        2
    );
    w.survival.as_mut().unwrap().player.active_item = Some(ActiveItem::Stack(ItemId::Torch));
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    let before = fs::read(dir.join("checkpoint.json")).unwrap();
    assert!(w.act_with_entities(Action::Attack).unwrap());
    assert!(
        w.commit_with_hook(|| Err("interrupted torch publication".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), before);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Torch),
        2
    );
    w.survival_action(Action::Attack).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Torch),
        1
    );
    assert_eq!(w.cell(-65, -65).unwrap().data, 731);
    let q = Position {
        depth: -1,
        x: -64,
        y: -64,
    };
    assert!(w.torch_lit(q, &BTreeMap::new()).unwrap());
    w.checkpoint().unwrap();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.torch_lit(q, &BTreeMap::new()).unwrap());
    w.survival_action(Action::Pickup).unwrap();
    for _ in 0..2 {
        w.survival_action(Action::Tick).unwrap();
    }
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Torch),
        2
    );
    assert!(!w.torch_lit(q, &BTreeMap::new()).unwrap());
    assert_eq!(w.cell(-65, -65).unwrap().data, 731);
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
}

#[test]
fn slime_original_health_contact_drop_and_subclass_jump_trajectory_are_real() {
    let oracle = oracle();
    for difficulty in 0..3 {
        for level in 1..=3 {
            assert_eq!(
                u64::from(
                    stats::mob_health_at_level(NaturalMob::Slime, level, difficulty).unwrap()
                ),
                oracle["slimeHealth"][difficulty as usize][level as usize - 1]
                    .as_u64()
                    .unwrap()
            );
        }
    }
    for row in oracle["slimeContact"].as_array().unwrap() {
        assert_eq!(
            10 - u64::from(stats::mob_contact_damage(
                NaturalMob::Slime,
                row["level"].as_u64().unwrap() as u8,
                row["difficulty"].as_u64().unwrap() as u8
            )),
            row["health"].as_u64().unwrap()
        );
        assert_eq!(row["hurt"], 30);
    }
    for row in oracle["slimeDrops"].as_array().unwrap() {
        let mut random = JavaRandom::new(row["seed"].as_i64().unwrap());
        let drops = stats::mob_drops_at_level(NaturalMob::Slime, 1, 1, false, &mut random);
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].item, ItemId::Slime);
        assert_eq!(
            u64::from(drops[0].count),
            row["loot"]["Slime"].as_u64().unwrap()
        );
    }
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "SlimeAI", 42, 4).unwrap();
    w.visit_layer(-1, 32, 32).unwrap();
    for x in 20..64 {
        for y in 20..64 {
            floor(&mut w, x, y);
        }
    }
    for distance in [24i64, 300] {
        let start = Position {
            depth: -1,
            x: 32,
            y: 32,
        };
        let mut page = Page::empty(start.key());
        let mut slime = Animal::of_level(NaturalMob::Slime, 1);
        slime.px = 0;
        slime.py = 0;
        page.insert(start, Kind::Enemy(slime), None).unwrap();
        let mut pages = BTreeMap::from([(page.key, page)]);
        let mut s = State::new(42);
        s.random = JavaRandom::new(42);
        for (tick, expected) in oracle["slimeWalkSeed42MovingObserver"][distance.to_string()]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let page = &pages[&start.key()];
            let e = &page.entities[0];
            let body = e.kind.mover().unwrap();
            let pos = page.position(e);
            let px = pos.x * 16 + i64::from(body.px) + distance;
            let py = pos.y * 16 + i64::from(body.py) + distance;
            s.player.x = px.rem_euclid(16) as i32;
            s.player.y = py.rem_euclid(16) as i32;
            w.tick_animals(
                &mut s,
                &mut pages,
                Position {
                    depth: -1,
                    x: px.div_euclid(16),
                    y: py.div_euclid(16),
                },
            )
            .unwrap();
            let page = &pages[&start.key()];
            let e = &page.entities[0];
            let a = e.kind.mover().unwrap();
            let pos = page.position(e);
            assert_eq!(
                serde_json::json!([
                    pos.x * 16 + i64::from(a.px),
                    pos.y * 16 + i64::from(a.py),
                    a.mob.x_move,
                    a.mob.y_move,
                    a.mob.walk_time,
                    a.mob.walk_distance,
                    a.slime_jump
                ]),
                *expected,
                "distance={distance} first differing tick={}",
                tick + 1
            );
        }
    }
}

#[test]
fn combined_day_night_spawn_stream_matches_actual_original_and_consumes_unsupported_rolls() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Spawn", 42, 16).unwrap();
    for x in [-1, 0] {
        for y in [-1, 0] {
            let key = Key { depth: 0, x, y };
            w.ensure(key).unwrap();
            let c = w.resident.get_mut(&key).unwrap();
            c.cells.fill(Cell {
                ground: Ground::Grass,
                data: 0,
            });
        }
    }
    for row in oracle()["surfaceFirstSpawn"].as_array().unwrap() {
        let mut s = State::new(42);
        s.random = JavaRandom::new(row["seed"].as_i64().unwrap());
        let env = s.environment.as_mut().unwrap();
        env.day_tick = row["phase"].as_u64().unwrap() as u32 * 16200;
        env.past_day_one = row["pastDayOne"].as_bool().unwrap();
        let mut pages: BTreeMap<_, _> = w
            .resident
            .keys()
            .map(|&key| (key, Page::empty(key)))
            .collect();
        w.spawn_environment(
            &mut s,
            &mut pages,
            Position {
                depth: 0,
                x: 0,
                y: 0,
            },
        )
        .unwrap();
        let actual:Vec<_>=pages.values().flat_map(|p|p.entities.iter().filter_map(|e|e.kind.mover().map(|a|{
            let q=p.position(e);serde_json::json!({"kind":format!("{:?}",a.mob.species),"x":q.x*16+i64::from(a.px),"y":q.y*16+i64::from(a.py),"health":a.mob.health,"level":a.mob.level})
        }))).collect();
        let expected: Vec<_> = row["mobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| !matches!(e["kind"].as_str(), Some("Skeleton" | "Creeper")))
            .cloned()
            .collect();
        assert_eq!(
            serde_json::json!(actual),
            serde_json::json!(expected),
            "first spawn differs phase={} past={} seed={}; original ALL mobs={}",
            row["phase"],
            row["pastDayOne"],
            row["seed"],
            row["mobs"]
        );
    }
}

#[test]
fn torch_corruption_rolls_back_calendar_inventory_and_terrain_together() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "CorruptTorch", 42, 1).unwrap();
    w.visit_layer(-2, -65, -66).unwrap();
    floor(&mut w, -65, -66);
    floor(&mut w, -65, -65);
    let s = w.survival.as_mut().unwrap();
    s.player.direction = Direction::Down;
    s.player.inventory.add(ItemId::Torch, 1);
    s.player.active_item = Some(ActiveItem::Stack(ItemId::Torch));
    w.checkpoint().unwrap();
    let before = w.survival().unwrap().environment.clone();
    w.survival
        .as_mut()
        .unwrap()
        .environment
        .as_mut()
        .unwrap()
        .day_tick = 48600;
    w.set_cell(
        -65,
        -65,
        Cell {
            ground: Ground::Dirt,
            data: 23,
        },
    )
    .unwrap();
    w.survival_action(Action::Attack).unwrap();
    let key = Key::at_depth(-2, -65, -65);
    let object = w.find(w.saved.entity_root, key).unwrap().unwrap();
    let dir = w.directory.clone();
    drop(w);
    fs::write(
        objects::path(&dir, objects::Domain::Entities, object),
        b"torn light object",
    )
    .unwrap();
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.recovered_backup());
    assert_eq!(w.survival().unwrap().environment, before);
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Torch),
        1
    );
    assert_eq!(w.cell(-65, -65).unwrap().data, 731);
    assert!(
        !w.torch_lit(
            Position {
                depth: -2,
                x: -65,
                y: -65
            },
            &BTreeMap::new()
        )
        .unwrap()
    );
    assert!(w.collect_garbage().is_err());
}

#[test]
fn torch_overlay_prevents_sheep_grazing_the_preserved_substrate() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "GrazingLight", 42, 4).unwrap();
    w.set_cell(
        0,
        0,
        Cell {
            ground: Ground::Grass,
            data: 0,
        },
    )
    .unwrap();
    let seed = (0..100000)
        .find(|&seed| {
            let mut r = JavaRandom::new(seed);
            r.next_int(40) != 0 && r.next_int(1000) == 0
        })
        .unwrap();
    for torch in [false, true] {
        let pos = Position {
            depth: 0,
            x: 0,
            y: 0,
        };
        let mut p = Page::empty(pos.key());
        let mut sheep = Animal::of(NaturalMob::Sheep);
        sheep.mob.sheared = true;
        p.insert(pos, Kind::Animal(sheep), None).unwrap();
        if torch {
            p.insert(pos, Kind::Torch, None).unwrap();
        }
        let mut pages = BTreeMap::from([(p.key, p)]);
        let mut state = State::new(42);
        state.random = JavaRandom::new(seed);
        let grazes = w
            .tick_animals(
                &mut state,
                &mut pages,
                Position {
                    depth: 0,
                    x: 2,
                    y: 2,
                },
            )
            .unwrap();
        assert_eq!(grazes.len(), usize::from(!torch));
        assert_eq!(
            pages[&pos.key()].entities[0]
                .kind
                .animal()
                .unwrap()
                .mob
                .sheared,
            torch
        );
    }
}

#[test]
fn slime_cross_page_backpressure_and_fatal_contact_keep_light_and_chest_consistent() {
    use crate::experimental::infdev::{entities::dynamic::ItemDrop, survival::Loot};
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "SlimeState", 42, 4).unwrap();
    w.visit_layer(-3, -4, 2).unwrap();
    floor(&mut w, -4, 2);
    for x in -2..=1 {
        for y in -1..=1 {
            floor(&mut w, x, y);
        }
    }
    let from = Key::at_depth(-3, -1, 0);
    let to = Key::at_depth(-3, 0, 0);
    let pos = Position {
        depth: -3,
        x: -1,
        y: 0,
    };
    let mut p = Page::empty(from);
    let mut a = Animal::of_level(NaturalMob::Slime, 3);
    a.px = 15;
    a.knock_x = 6;
    a.mob.hurt_time = 10;
    a.slime_jump = 9;
    p.insert(pos, Kind::Enemy(a), None).unwrap();
    let original = p.entities[0].clone();
    let target = Position {
        depth: -3,
        x: 0,
        y: 0,
    };
    let mut full = Page::empty(to);
    for _ in 0..64 {
        full.insert(
            target,
            Kind::Drop(ItemDrop {
                loot: Loot::Stack(ItemStack::new(ItemId::Wood, 1)),
                delay: 0,
            }),
            None,
        )
        .unwrap();
    }
    let mut pages = BTreeMap::from([(from, p), (to, full)]);
    let mut state = w.survival().unwrap().clone();
    w.tick_animals(
        &mut state,
        &mut pages,
        Position {
            depth: -3,
            x: -4,
            y: 2,
        },
    )
    .unwrap();
    assert!(pages[&from].entities[0] == original);
    pages.insert(to, Page::empty(to));
    w.tick_animals(
        &mut state,
        &mut pages,
        Position {
            depth: -3,
            x: -4,
            y: 2,
        },
    )
    .unwrap();
    assert!(pages[&from].entities.is_empty());
    let moved = pages[&to].entities[0].clone();
    assert_eq!(moved.birth, original.birth);
    assert_eq!(moved.kind.mover().unwrap().slime_jump, 8);
    for (key, page) in pages {
        w.entity_cache.insert(
            key,
            entities::CachedEntities {
                page,
                dirty: true,
                touched: 0,
            },
        );
    }
    w.survival.as_mut().unwrap().entity_pages_present = true;
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert!(w.entity_page(to).unwrap().entities[0] == moved);
    // Real contact update, with a non-solid light at the death site.
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
    let mut page = Page::empty(at.key());
    let mut a = Animal::of_level(NaturalMob::Slime, 1);
    a.py = 2;
    a.mob.y_move = 1;
    page.insert(at, Kind::Enemy(a), None).unwrap();
    page.insert(at, Kind::Torch, None).unwrap();
    w.entity_cache.insert(
        at.key(),
        entities::CachedEntities {
            page,
            dirty: true,
            touched: 0,
        },
    );
    let s = w.survival.as_mut().unwrap();
    s.player.health = 1;
    s.player.inventory.add_stack(ItemStack::with_water(731));
    s.player.inventory.add(ItemId::Torch, 1);
    let inventory = s.player.inventory.clone();
    w.checkpoint().unwrap();
    let before = w.survival().unwrap().environment.clone();
    assert!(w.act_dynamic(Action::Tick).unwrap());
    assert_eq!(w.survival().unwrap().player.health, 0);
    assert!(
        w.commit_with_hook(|| Err("interrupted slime death".into()))
            .is_err()
    );
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert_eq!(w.survival().unwrap().environment, before);
    assert_eq!(w.survival().unwrap().player.inventory, inventory);
    w.survival_action(Action::Tick).unwrap();
    w.unload_all().unwrap();
    drop(w);
    let mut w = World::open(&dir, 4).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    let p = w.entity_page(at.key()).unwrap();
    assert!(p.entities.iter().any(|e| matches!(e.kind, Kind::Torch)));
    assert_eq!(
        p.entities
            .iter()
            .filter(|e| matches!(&e.kind,Kind::DeathChest(c) if c.inventory==inventory))
            .count(),
        1
    );
    w.survival_action(Action::Confirm).unwrap();
    w.visit_layer(-1, at.x, at.y).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival().unwrap().player.inventory.slots(),
        inventory.slots()
    );
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
}
