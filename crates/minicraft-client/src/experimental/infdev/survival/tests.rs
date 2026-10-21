use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Owned(PathBuf);
impl Owned {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "infdev-survival-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl std::ops::Drop for Owned {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn act(w: &mut World, a: Action) {
    w.survival_action(a).unwrap();
}
fn ticks(w: &mut World, n: usize) {
    for _ in 0..n {
        act(w, Action::Tick);
    }
}
fn craft(w: &mut World, workbench: bool, name: &str) {
    act(w, Action::Crafting);
    // Select an existing recipe through the same cursor/confirm operations.
    if workbench {
        // The actual placed workbench was left one tile below the player.
        act(w, Action::Close);
        act(w, Action::Interact);
    }
    let i = recipes(workbench.then_some(StationKind::Workbench))
        .iter()
        .position(|r| r.product_name() == name)
        .unwrap();
    act(w, Action::Cursor(i as i32));
    act(w, Action::Confirm);
    act(w, Action::Close);
}
fn select_stack(w: &mut World, id: ItemId) {
    let i = w
        .survival()
        .unwrap()
        .player
        .inventory
        .slots()
        .iter()
        .position(|s| s.item == id)
        .unwrap();
    act(w, Action::Inventory);
    act(w, Action::Cursor(i as i32));
    act(w, Action::Confirm);
}
// Tests discover genuine terrain; visit is explicit fixture positioning, not a
// claim of walking the whole route. The separate real-window driver walks it.
fn target(w: &mut World, g: Ground) -> (i64, i64) {
    for y in -64..64 {
        for x in -64..64 {
            if w.cell_at(0, x, y).unwrap().ground == g && !w.survival().unwrap().station_at(0, x, y)
            {
                let stand = w.cell_at(0, x, y - 1).unwrap();
                if !tile(stand).solid(stand.data)
                    && !matches!(stand.ground, Ground::Water | Ground::Lava | Ground::Hole)
                    && !w.survival().unwrap().station_at(0, x, y - 1)
                {
                    w.visit_layer(0, x, y - 1).unwrap();
                    act(w, Action::Move(0, 1));
                    return (x, y);
                }
            }
        }
    }
    panic!("natural target missing: {g:?}")
}
fn gather(w: &mut World, g: Ground, id: ItemId, count: u16) {
    for _ in 0..100 {
        if w.survival().unwrap().player.inventory.count(id) >= count {
            return;
        }
        let p = target(w, g);
        for _ in 0..150 {
            if w.cell(p.0, p.1).unwrap().ground != g {
                break;
            }
            act(w, Action::Attack);
            ticks(w, 12);
        }
        ticks(w, 2);
    }
    panic!(
        "harvesting did not produce {count} {id:?}; position={:?} session={}",
        w.position(),
        serde_json::to_string(w.survival().unwrap()).unwrap()
    )
}

fn place_station(w: &mut World, id: ItemId) -> (i64, i64) {
    let p = target(w, Ground::Grass);
    select_stack(w, id);
    ticks(w, 60);
    act(w, Action::Attack);
    assert!(w.survival().unwrap().station_at(0, p.0, p.1));
    p
}

fn station_craft(w: &mut World, p: (i64, i64), kind: StationKind, name: &str) {
    w.visit_layer(0, p.0, p.1 - 1).unwrap();
    act(w, Action::Move(0, 1));
    act(w, Action::Interact);
    assert!(
        w.survival().unwrap().panel == Panel::Crafting(Some(kind)),
        "expected {kind:?} at {p:?}"
    );
    let i = recipes(Some(kind))
        .iter()
        .position(|r| r.product_name() == name)
        .unwrap();
    act(w, Action::Cursor(i as i32));
    act(w, Action::Confirm);
    act(w, Action::Close);
}

#[test]
fn natural_resources_to_furnace_iron_anvil_tools_and_food_restore() {
    let owned = Owned::new();
    let mut w = passive_world(&owned.0, "Smelt", 42, 4).unwrap();
    gather(&mut w, Ground::Tree, ItemId::Wood, 40);
    craft(&mut w, false, "Workbench");
    let bench = place_station(&mut w, ItemId::Workbench);
    station_craft(&mut w, bench, StationKind::Workbench, "Wood Pickaxe");
    gather(&mut w, Ground::Rock, ItemId::Stone, 25);
    station_craft(&mut w, bench, StationKind::Workbench, "Rock Pickaxe");
    station_craft(&mut w, bench, StationKind::Workbench, "Furnace");
    let furnace = place_station(&mut w, ItemId::Furnace);
    let i = w
        .survival()
        .unwrap()
        .player
        .inventory
        .tools()
        .iter()
        .position(|t| t.kind == ToolKind::Pickaxe && t.tier == crate::item::ToolTier::Rock)
        .unwrap();
    let slot = w.survival().unwrap().player.inventory.slots().len() + i;
    act(&mut w, Action::Inventory);
    act(&mut w, Action::Cursor(slot as i32));
    act(&mut w, Action::Confirm);
    // Fixture positioning is explicit; harvesting and crafting are real actions.
    for _ in 0..100 {
        let inv = &w.survival().unwrap().player.inventory;
        if inv.count(ItemId::IronOre) >= 30 {
            break;
        }
        let mut found = None;
        'scan: for y in -64..64 {
            for x in -64..64 {
                if w.cell_at(-1, x, y).unwrap().ground == Ground::IronOre {
                    let stand = w.cell_at(-1, x, y - 1).unwrap();
                    if !tile(stand).solid(stand.data)
                        && !matches!(stand.ground, Ground::Water | Ground::Lava | Ground::Hole)
                    {
                        found = Some((x, y));
                        break 'scan;
                    }
                }
            }
        }
        let p = found.expect("natural underground iron");
        w.visit_layer(-1, p.0, p.1 - 1).unwrap();
        act(&mut w, Action::Move(0, 1));
        for _ in 0..20 {
            if w.cell(p.0, p.1).unwrap().ground != Ground::IronOre {
                break;
            }
            ticks(&mut w, 100);
            act(&mut w, Action::Attack);
        }
    }
    assert!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::IronOre)
            >= 30
    );
    gather(&mut w, Ground::Rock, ItemId::Coal, 10);
    let ore = w
        .survival()
        .unwrap()
        .player
        .inventory
        .count(ItemId::IronOre);
    let coal = w.survival().unwrap().player.inventory.count(ItemId::Coal);
    for _ in 0..10 {
        station_craft(&mut w, furnace, StationKind::Furnace, "Iron");
    }
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::IronOre),
        ore - 30
    );
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Coal),
        coal - 10
    );
    station_craft(&mut w, bench, StationKind::Workbench, "Anvil");
    let anvil = place_station(&mut w, ItemId::Anvil);
    station_craft(&mut w, anvil, StationKind::Anvil, "Iron Pickaxe");
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .count(ItemId::IronIngot),
        0
    );
    assert_eq!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .tools()
            .last()
            .unwrap()
            .tier,
        crate::item::ToolTier::Iron
    );
    // Food comes from actual TreeTile-style hit drops, never test injection.
    gather(&mut w, Ground::Tree, ItemId::Apple, 1);
    assert!(w.survival().unwrap().player.hunger < 10);
    let food = w.survival().unwrap().player.inventory.count(ItemId::Apple);
    let hunger = w.survival().unwrap().player.hunger;
    select_stack(&mut w, ItemId::Apple);
    ticks(&mut w, 60);
    act(&mut w, Action::Attack);
    assert_eq!(
        w.survival().unwrap().player.inventory.count(ItemId::Apple),
        food - 1
    );
    assert_eq!(w.survival().unwrap().player.hunger, hunger + 1);
    let before = serde_json::to_value(w.survival().unwrap()).unwrap();
    w.checkpoint().unwrap();
    let path = w.directory().to_owned();
    w.collect_garbage().unwrap();
    drop(w);
    let w = World::open(&path, 1).unwrap();
    assert_eq!(serde_json::to_value(w.survival().unwrap()).unwrap(), before);
}

#[test]
fn smelting_failures_and_station_pickup_do_not_consume_or_duplicate_items() {
    let owned = Owned::new();
    let mut w = passive_world(&owned.0, "FullSmelt", 42, 2).unwrap();
    let mut s = w.survival().unwrap().clone();
    s.player.inventory.add(ItemId::IronOre, 31);
    s.player.inventory.add(ItemId::Coal, 11);
    for _ in 0..25 {
        s.player
            .inventory
            .add_tool(ToolItem::new(
                ToolKind::Pickaxe,
                crate::item::ToolTier::Wood,
            ))
            .unwrap();
    }
    s.panel = Panel::Crafting(Some(StationKind::Furnace));
    s.selection = recipes(Some(StationKind::Furnace))
        .iter()
        .position(|r| r.product_name() == "Iron")
        .unwrap();
    let before = serde_json::to_value(&s).unwrap();
    assert!(s.confirm().is_err());
    assert_eq!(serde_json::to_value(&s).unwrap(), before);
    s.player.inventory.take_tool(0).unwrap();
    s.confirm().unwrap();
    assert_eq!(s.player.inventory.count(ItemId::IronIngot), 1);
    assert_eq!(s.player.inventory.count(ItemId::Coal), 10);
    assert_eq!(s.player.inventory.count(ItemId::IronOre), 28);
    s.player.inventory.remove(ItemId::Coal, 10);
    let before = serde_json::to_value(&s).unwrap();
    assert!(s.confirm().is_err());
    assert_eq!(serde_json::to_value(&s).unwrap(), before);
    s.panel = Panel::Closed;
    for kind in [StationKind::Furnace, StationKind::Anvil] {
        let p = s.target(&w).unwrap();
        s.stations.push(Station {
            depth: w.depth(),
            x: p.0,
            y: p.1,
            kind,
        });
        s.act(&mut w, Action::Pickup).unwrap();
        assert!(s.stations.is_empty());
        assert!(
            matches!(s.drops.last().unwrap().loot,Loot::Stack(st) if st.item==kind.item() && st.count==1)
        );
    }
    s.validate().unwrap();
}

#[test]
fn food_metabolism_pause_heal_starve_and_legacy_session_contracts() {
    let owned = Owned::new();
    let mut w = passive_world(&owned.0, "Food", 42, 2).unwrap();
    let mut s = w.survival().unwrap().clone();
    s.player.inventory.add(ItemId::Apple, 2);
    s.player.active_item = Some(ActiveItem::Stack(ItemId::Apple));
    let before = serde_json::to_value(&s).unwrap();
    s.act(&mut w, Action::Attack).unwrap();
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        before,
        "full hunger retains food and stamina"
    );
    s.player.hunger = 8;
    s.player.stamina = 0;
    s.act(&mut w, Action::Attack).unwrap();
    assert_eq!(s.player.inventory.count(ItemId::Apple), 2);
    s.player.stamina = 1;
    s.act(&mut w, Action::Attack).unwrap();
    assert_eq!(s.player.hunger, 9);
    assert_eq!(s.player.stamina, 0);
    assert_eq!(s.player.inventory.count(ItemId::Apple), 1);
    s.panel = Panel::Inventory;
    let before = serde_json::to_value(&s).unwrap();
    for _ in 0..121 {
        s.act(&mut w, Action::Tick).unwrap();
    }
    assert_eq!(serde_json::to_value(&s).unwrap(), before);
    s.player.health = 9;
    s.player.hunger = 10;
    s.player.hunger_charge_delay = 0;
    for tick in 1..=80 {
        metabolism::tick(&mut s.player, tick);
    }
    assert_eq!(s.player.health, 9);
    metabolism::tick(&mut s.player, 81);
    assert_eq!(s.player.health, 10);
    s.player.hunger = 0;
    s.player.health = 4;
    s.player.hunger_starve_delay = 0;
    for tick in 1..=119 {
        metabolism::tick(&mut s.player, tick);
    }
    assert_eq!(s.player.health, 4);
    metabolism::tick(&mut s.player, 120);
    for tick in 121..=1000 {
        metabolism::tick(&mut s.player, tick);
    }
    assert_eq!(s.player.health, 3, "Normal starvation floor, not Hardcore");
    s.entity_rules = crate::experimental::infdev::survival::EntityRules::Legacy;
    s.food_rules = FoodRules::Legacy;
    s.death_rule = crate::experimental::infdev::survival::DeathRule::KeepInventory;
    s.panel = Panel::Closed;
    s.stations.push(Station {
        depth: 0,
        x: 1,
        y: 1,
        kind: StationKind::Workbench,
    });
    let old = serde_json::to_value(&s).unwrap();
    assert!(old.get("food_rules").is_none());
    assert!(old["stations"][0].get("kind").is_none());
    let mut restored: State = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        old,
        "byte-semantic old fields stay canonical"
    );
    let hunger = restored.player.hunger;
    for _ in 0..3000 {
        restored.act(&mut w, Action::Tick).unwrap();
    }
    assert_eq!(
        restored.player.hunger, hunger,
        "legacy v4 must not silently enable hunger"
    );
    restored.player.health = 0;
    let inventory = serde_json::to_value(&restored.player.inventory).unwrap();
    restored.act(&mut w, Action::Confirm).unwrap();
    assert_eq!(
        serde_json::to_value(&restored.player.inventory).unwrap(),
        inventory,
        "legacy death retains every instance"
    );
}

#[test]
fn natural_empty_inventory_harvest_workbench_tools_and_disk_restore() {
    let owned = Owned::new();
    let mut w = passive_world(&owned.0, "Play", 42, 4).unwrap();
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
    gather(&mut w, Ground::Tree, ItemId::Wood, 24);
    craft(&mut w, false, "Workbench");
    // Place on a naturally walkable target; no terrain is planted by the test.
    let _ = target(&mut w, Ground::Grass);
    let bench = (w.position().0, w.position().1 + 1);
    select_stack(&mut w, ItemId::Workbench);
    ticks(&mut w, 12);
    act(&mut w, Action::Attack);
    assert!(w.survival().unwrap().station_at(0, bench.0, bench.1));
    craft(&mut w, true, "Wood Pickaxe");
    gather(&mut w, Ground::Rock, ItemId::Stone, 5);
    let mined = target(&mut w, Ground::Rock);
    ticks(&mut w, 40);
    act(&mut w, Action::Attack); // preserve real partial rock damage
    let cell = w.cell(mined.0, mined.1).unwrap();
    assert!(cell.data > 0);
    let before = serde_json::to_value(w.survival().unwrap()).unwrap();
    let position = w.position();
    let path = w.directory().to_path_buf();
    w.checkpoint().unwrap();
    w.collect_garbage().unwrap();
    drop(w);
    let mut w = World::open(&path, 1).unwrap();
    assert_eq!(w.position(), position);
    assert_eq!(w.cell(mined.0, mined.1).unwrap(), cell);
    assert_eq!(serde_json::to_value(w.survival().unwrap()).unwrap(), before);
    assert!(
        w.survival()
            .unwrap()
            .player
            .inventory
            .tools()
            .iter()
            .any(|t| t.kind == ToolKind::Pickaxe && t.durability < t.max_durability)
    );
    assert!(w.survival().unwrap().station_at(0, bench.0, bench.1));
}

#[test]
fn inventory_and_tool_transactions_preserve_full_bags_and_last_durability() {
    let owned = Owned::new();
    let mut w = passive_world(&owned.0, "Full", 42, 4).unwrap();
    let tree = target(&mut w, Ground::Tree);
    let mut s = w.survival().unwrap().clone();
    let mut axe = ToolItem::new(ToolKind::Axe, crate::item::ToolTier::Wood);
    axe.durability = 1;
    s.player.inventory.add_tool(axe).unwrap();
    s.player.active_item = Some(ActiveItem::Tool(0));
    s.act(&mut w, Action::Attack).unwrap();
    assert_eq!(s.player.inventory.tools().len(), 0);
    assert!(
        w.cell(tree.0, tree.1).unwrap().data >= 10,
        "last use still damages real tree"
    );
    for _ in 0..27 {
        s.player
            .inventory
            .add_tool(ToolItem::new(ToolKind::Shovel, crate::item::ToolTier::Wood))
            .unwrap();
    }
    s.add_drop(&w, w.position(), Loot::Tool(axe), 0);
    s.add_drop(
        &w,
        w.position(),
        Loot::Stack(ItemStack::new(ItemId::Wood, 731)),
        0,
    );
    s.act(&mut w, Action::Tick).unwrap();
    assert_eq!(s.drops.len(), 2);
    s.player.active_item = Some(ActiveItem::Tool(0));
    s.act(&mut w, Action::Drop(true)).unwrap();
    assert_eq!(s.player.inventory.tools().len(), 26);
    s.act(&mut w, Action::Tick).unwrap();
    assert!(s.player.inventory.tools().iter().any(|t| t.durability == 1));
    assert_eq!(
        s.drops
            .iter()
            .filter_map(|d| match d.loot {
                Loot::Stack(st) => Some(st.count),
                _ => None,
            })
            .sum::<u16>(),
        731
    );
    s.validate().unwrap();
}

#[test]
fn collision_blocks_solids_and_missing_materials_do_not_create_items() {
    let owned = Owned::new();
    let mut w = passive_world(&owned.0, "Collision", 42, 4).unwrap();
    let p = target(&mut w, Ground::Tree);
    let start = w.position();
    for _ in 0..32 {
        act(&mut w, Action::Move(0, 1));
    }
    assert_eq!(w.position(), start);
    assert_eq!(w.cell(p.0, p.1).unwrap().ground, Ground::Tree);
    act(&mut w, Action::Crafting);
    let before = serde_json::to_value(w.survival().unwrap()).unwrap();
    assert!(w.survival_action(Action::Confirm).is_err());
    assert_eq!(serde_json::to_value(w.survival().unwrap()).unwrap(), before);
    assert_eq!(w.survival().unwrap().player.inventory.used_slots(), 0);
}

#[test]
fn exhausted_stamina_recharges_and_native_drop_instances_round_trip() {
    let owned = Owned::new();
    let mut w = passive_world(&owned.0, "Drop", 42, 2).unwrap();
    let mut s = w.survival().unwrap().clone();
    s.player.stamina = 0;
    for _ in 0..55 {
        s.act(&mut w, Action::Tick).unwrap();
    }
    assert!(
        s.player.stamina > 0,
        "last recovery-delay tick must start recharge"
    );
    let mut tool = ToolItem::new(ToolKind::Pickaxe, crate::item::ToolTier::Rock);
    tool.durability = 23;
    s.player.inventory.add_tool(tool).unwrap();
    s.player.active_item = Some(ActiveItem::Tool(0));
    s.act(&mut w, Action::Drop(true)).unwrap();
    assert_eq!(s.player.inventory.used_slots(), 0);
    let bytes = serde_json::to_vec(&s).unwrap();
    let mut restored: State = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    for _ in 0..61 {
        restored.act(&mut w, Action::Tick).unwrap();
    }
    assert_eq!(restored.player.inventory.tools(), &[tool]);
    assert!(restored.drops.is_empty());
    restored.player.health = 0;
    w.set_cell_at(
        0,
        restored.spawn.0,
        restored.spawn.1,
        Cell {
            ground: Ground::Hole,
            data: 0,
        },
    )
    .unwrap();
    let before = serde_json::to_value(&restored).unwrap();
    assert!(restored.act(&mut w, Action::Confirm).is_err());
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        before,
        "unsafe spawn cannot reset life or owned instances"
    );
    w.set_cell(
        w.position().0,
        w.position().1,
        Cell {
            ground: Ground::Water,
            data: 0,
        },
    )
    .unwrap();
    restored.player.health = 10;
    restored.player.stamina_recharge = 7;
    restored.act(&mut w, Action::Tick).unwrap();
    assert_eq!(
        restored.player.stamina_recharge, 0,
        "swimming cannot bank partial land recharge"
    );
}

// Preserve the exact historical v4 fixture rule, independent of new-world defaults.
fn passive_world(
    root: &std::path::Path,
    name: &str,
    seed: i64,
    capacity: usize,
) -> Result<World, String> {
    let mut w = World::create_survival(root, name, seed, capacity)?;
    w.passive_fixture();
    w.checkpoint()?;
    Ok(w)
}
