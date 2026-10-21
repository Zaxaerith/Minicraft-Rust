use super::{tests::Owned, *};
use crate::{
    experimental::infdev::{
        entities::{Kind, Position},
        survival::{Action, Panel, StationKind, StationRules},
    },
    item::{ItemId, ItemStack},
    world::{ActiveItem, Direction},
};

fn setup(w: &mut World, depth: i8, x: i64, y: i64, k: StationKind) {
    w.visit_layer(depth, x, y).unwrap();
    for (x, y) in [(x, y), (x, y + 1)] {
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
    let s = w.survival.as_mut().unwrap();
    s.player.direction = Direction::Down;
    s.player.x = 8;
    s.player.y = 8;
    s.player.attack_time = 0;
    s.player.inventory.add(k.item(), 1);
    s.player.active_item = Some(ActiveItem::Stack(k.item()));
}
fn at(w: &World) -> Position {
    let (x, y) = w.position();
    Position {
        depth: w.depth(),
        x,
        y: y + 1,
    }
}

#[test]
fn paged_stations_use_original_actions_recipes_and_survive_depths_eviction_gc() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Stations", 42, 1).unwrap();
    let mut positions = Vec::new();
    for (i, k) in [
        StationKind::Workbench,
        StationKind::Furnace,
        StationKind::Anvil,
        StationKind::DyeVat,
    ]
    .into_iter()
    .enumerate()
    {
        let x = if i % 2 == 0 {
            -65 - i as i64 * 64
        } else {
            64 + i as i64 * 64
        };
        setup(&mut w, -(i as i8), x, -65, k);
        let p = at(&w);
        w.survival_action(Action::Attack).unwrap();
        assert_eq!(w.paged_station_at(p.depth, p.x, p.y).unwrap(), Some(k));
        assert!(w.survival().unwrap().stations.is_empty());
        assert_eq!(w.survival().unwrap().player.inventory.count(k.item()), 0);
        w.survival_action(Action::Interact).unwrap();
        assert!(w.survival().unwrap().panel == Panel::Crafting(Some(k)));
        let product = match k {
            StationKind::Workbench => "Wood Axe",
            StationKind::Furnace => "Iron",
            StationKind::Anvil => "Iron Pickaxe",
            StationKind::DyeVat => "Red Dye",
        };
        // Execute a real recipe when ingredients are absent: no material or output changes.
        let s = w.survival.as_mut().unwrap();
        s.selection = s
            .recipes(Some(k))
            .iter()
            .position(|r| r.product_name().contains(product))
            .expect("shared station recipe must be present");
        let inventory = s.player.inventory.clone();
        assert!(w.survival_action(Action::Confirm).is_err());
        assert_eq!(w.survival().unwrap().player.inventory, inventory);
        w.survival_action(Action::Close).unwrap();
        positions.push((p, k));
    }
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    for (p, k) in positions {
        assert_eq!(w.paged_station_at(p.depth, p.x, p.y).unwrap(), Some(k));
        assert!(w.entity_cache.len() <= 1);
    }
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
    assert!(w.survival().unwrap().stations.is_empty());
}

#[test]
fn workstation_placement_and_pickup_faults_keep_exactly_one_owner() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Fault", 42, 1).unwrap();
    setup(&mut w, -1, -65, -66, StationKind::Furnace);
    let p = at(&w);
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    let bytes = fs::read(dir.join("checkpoint.json")).unwrap();
    assert!(w.act_with_entities(Action::Attack).unwrap());
    assert!(
        w.commit_with_hook(|| Err("interrupted before station pointer".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), bytes);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(w.paged_station_at(p.depth, p.x, p.y).unwrap(), None);
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Furnace),
        1
    );
    w.survival_action(Action::Attack).unwrap();
    let bytes = fs::read(dir.join("checkpoint.json")).unwrap();
    assert!(w.act_with_entities(Action::Pickup).unwrap());
    assert!(
        w.commit_with_hook(|| Err("interrupted before station pickup".into()))
            .is_err()
    );
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), bytes);
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    assert_eq!(
        w.paged_station_at(p.depth, p.x, p.y).unwrap(),
        Some(StationKind::Furnace)
    );
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Furnace),
        0
    );
    w.survival_action(Action::Pickup).unwrap();
    for _ in 0..3 {
        w.survival_action(Action::Tick).unwrap();
    }
    assert_eq!(w.paged_station_at(p.depth, p.x, p.y).unwrap(), None);
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Furnace),
        1
    );
    w.survival_action(Action::Pickup).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Furnace),
        1
    );
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let w = World::open(&dir, 1).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Furnace),
        1
    );
}

#[test]
fn full_station_page_refuses_without_consuming_items_or_overwriting_entities() {
    use crate::experimental::infdev::{
        entities::{Page, dynamic::ItemDrop},
        survival::Loot,
    };
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Full", 42, 1).unwrap();
    setup(&mut w, -2, 64, -66, StationKind::Anvil);
    let p = at(&w);
    let mut page = Page::empty(p.key());
    for _ in 0..64 {
        page.insert(
            p,
            Kind::Drop(ItemDrop {
                loot: Loot::Stack(ItemStack::new(ItemId::Wood, 1)),
                delay: 0,
            }),
            None,
        )
        .unwrap();
    }
    w.entity_cache.insert(
        p.key(),
        entities::CachedEntities {
            page: page.clone(),
            dirty: true,
            touched: 0,
        },
    );
    w.survival.as_mut().unwrap().entity_pages_present = true;
    w.checkpoint().unwrap();
    let inv = w.survival().unwrap().player.inventory.clone();
    assert!(w.survival_action(Action::Attack).is_err());
    assert_eq!(w.survival().unwrap().player.inventory, inv);
    assert!(w.entity_page(p.key()).unwrap() == page);
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
}

#[test]
fn legacy_station_rule_is_omitted_and_paged_session_duplicates_are_rejected() {
    use crate::experimental::infdev::survival::{State, Station};
    let mut s = State::new(42);
    s.station_rules = StationRules::Session;
    s.stations.push(Station {
        depth: -1,
        x: -65,
        y: 64,
        kind: StationKind::Furnace,
    });
    let value = serde_json::to_value(&s).unwrap();
    assert!(value.get("station_rules").is_none());
    let old: State = serde_json::from_value(value).unwrap();
    old.validate().unwrap();
    assert_eq!(old.station_rules, StationRules::Session);
    assert!(old.station_at(-1, -65, 64));
    s.station_rules = StationRules::PagedV1;
    assert!(s.validate().is_err());
    let mut bad = serde_json::to_value(&old).unwrap();
    bad["station_rules"] = "UnknownFuture".into();
    assert!(serde_json::from_value::<State>(bad).is_err());
}

#[test]
fn corrupt_station_page_recovers_inventory_and_map_together_and_pins_gc() {
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Corrupt", 42, 1).unwrap();
    setup(&mut w, -3, -65, -66, StationKind::Workbench);
    let p = at(&w);
    w.checkpoint().unwrap();
    w.survival_action(Action::Attack).unwrap();
    let object = w.find(w.saved.entity_root, p.key()).unwrap().unwrap();
    let dir = w.directory.clone();
    drop(w);
    fs::write(
        objects::path(&dir, objects::Domain::Entities, object),
        b"torn station page",
    )
    .unwrap();
    let mut w = World::open(&dir, 1).unwrap();
    assert!(w.recovered_backup());
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Workbench),
        1
    );
    assert_eq!(w.paged_station_at(p.depth, p.x, p.y).unwrap(), None);
    assert_eq!(w.cell(p.x, p.y).unwrap().ground, Ground::Dirt);
    assert!(w.collect_garbage().is_err());
    assert!(dir.join("gc-recovery.hold").exists());
}

#[test]
fn paged_station_blocks_player_and_full_bag_reclamation_preserves_one_drop() {
    use crate::item::{ToolItem, ToolKind, ToolTier};
    let owned = Owned::new();
    let mut w = World::create_survival(&owned.0, "Collision", 42, 4).unwrap();
    setup(&mut w, -1, -65, -66, StationKind::Workbench);
    let p = at(&w);
    w.survival_action(Action::Attack).unwrap();
    for _ in 0..16 {
        w.survival_action(Action::Move(0, 1)).unwrap();
    }
    assert_ne!(w.position(), (p.x, p.y));
    let original = w.survival().unwrap().player.inventory.clone();
    assert_eq!(original.used_slots(), 0);
    for _ in 0..27 {
        w.survival
            .as_mut()
            .unwrap()
            .player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
            .unwrap();
    }
    let inv = w.survival().unwrap().player.inventory.clone();
    w.survival_action(Action::Pickup).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory, inv);
    assert_eq!(w.paged_station_at(p.depth, p.x, p.y).unwrap(), None);
    let page = w.entity_page(p.key()).unwrap();
    assert_eq!(
        page.entities
            .iter()
            .filter(|e| matches!(e.kind, Kind::Drop(_)))
            .count(),
        1
    );
    w.checkpoint().unwrap();
    let dir = w.directory.clone();
    drop(w);
    let mut w = World::open(&dir, 1).unwrap();
    w.survival.as_mut().unwrap().player.inventory.take_tool(0);
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Workbench),
        1
    );
    w.survival_action(Action::Pickup).unwrap();
    w.survival_action(Action::Tick).unwrap();
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::Workbench),
        1
    );
    assert_eq!(
        w.survival().unwrap().player.inventory.tools(),
        &inv.tools()[1..]
    );
    assert!(
        w.entity_page(p.key())
            .unwrap()
            .entities
            .iter()
            .all(|e| { !matches!(e.kind, Kind::Station(_) | Kind::Drop(_)) }),
        "reclaimed furniture has exactly one inventory owner; naturally spawned mobs remain"
    );
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
}
