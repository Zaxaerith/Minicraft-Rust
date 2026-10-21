//! Native Input-only battle/death chain. Every enemy and item is naturally earned.
use super::*;
use crate::experimental::infdev::{
    entities::{Kind, dynamic::Birth},
    paged::Key,
};
use crate::item::Inventory;
use std::collections::BTreeSet;
pub(super) fn zombies(e: &Explorer) -> Vec<(Birth, Point)> {
    e.entity_pages()
        .flat_map(|p| {
            p.entities.iter().filter_map(move |entity| {
                if !matches!(entity.kind, Kind::Enemy(_)) {
                    return None;
                }
                let at = p.position(entity);
                Some((entity.birth?, (at.x, at.y)))
            })
        })
        .collect()
}
// Transfer creates a new bag: slot_revision is an invalidation token, not
// an item field. Check every persisted item/tool field and capacity; after
// transfer, save/reload must preserve the COMPLETE new inventory including its token.
fn same_items(a: &Inventory, b: &Inventory) -> bool {
    a.capacity() == b.capacity() && a.slots() == b.slots() && a.tools() == b.tools()
}
fn corpse(e: &Explorer, at: Point) -> Option<Inventory> {
    e.entity_pages()
        .flat_map(|p| {
            p.entities.iter().filter_map(move |entity| {
                let q = p.position(entity);
                if let Kind::DeathChest(c) = &entity.kind
                    && q.depth == e.depth()
                    && (q.x, q.y) == at
                {
                    return Some(c.inventory.clone());
                }
                None
            })
        })
        .next()
}
pub(super) fn advance(
    h: &mut Host,
    e: &mut Explorer,
    samples: &mut Vec<(i8, Point)>,
) -> Result<Option<Point>, String> {
    if let Some((_, q)) = zombies(e)
        .into_iter()
        .min_by_key(|(_, q)| q.0.abs_diff(e.position().0) + q.1.abs_diff(e.position().1))
    {
        let distance = q.0.abs_diff(e.position().0) + q.1.abs_diff(e.position().1);
        if distance == 1 {
            face(h, e, (q.0 - e.position().0, q.1 - e.position().1))?;
            return Ok(Some(q));
        }
        let goal = DIRS
            .into_iter()
            .map(|d| (q.0 + d.0, q.1 + d.1))
            .filter(|q| {
                e.visible_cell(q.0, q.1)
                    .is_some_and(|c| !survival::tile(c).solid(c.data) && c.ground != Ground::Water)
            })
            .min_by_key(|q| q.0.abs_diff(e.position().0) + q.1.abs_diff(e.position().1))
            .unwrap_or(q);
        let (points, _) = path(e, Some(goal), None)?;
        walk(h, e, &points[..points.len().min(2)], samples)?;
    } else {
        for _ in 0..30 {
            h.frame(e)?;
        }
    }
    Ok(None)
}
// The route miner must defend itself using real Input, never mine a wall
// indefinitely while a natural enemy is biting from the side.
pub(super) fn defend(h: &mut Host, e: &mut Explorer) -> Result<bool, String> {
    let Some((_, q)) = zombies(e).into_iter().find(|(_, q)| {
        q.0.abs_diff(e.position().0)
            .max(q.1.abs_diff(e.position().1))
            <= 1
    }) else {
        return Ok(false);
    };
    if e.survival_state().unwrap().player.health == 0 {
        return Err("native defense died".into());
    }
    if !h.saw_enemy_hit {
        // Intentional standing Input to observe a real first hit; never alter HP.
        let before = e.survival_state().unwrap().player.health;
        for _ in 0..60 {
            h.frame(e)?;
            if e.survival_state().unwrap().player.health < before {
                h.saw_enemy_hit = true;
                break;
            }
        }
        return Ok(true);
    }
    if q.0 != e.position().0 && q.1 != e.position().1 {
        // Align with the target through a free cardinal neighbor. Repeatedly
        // retreating along X can pin the diagnostic against a cave wall forever.
        let p = e.position();
        let direction = [(q.0 - p.0, 0), (0, q.1 - p.1)].into_iter().find(|d| {
            e.visible_cell(p.0 + d.0, p.1 + d.1)
                .is_some_and(|c| !survival::tile(c).solid(c.data))
        });
        if let Some(d) = direction {
            let before = (
                p,
                e.survival_state().unwrap().player.x,
                e.survival_state().unwrap().player.y,
            );
            face(h, e, d)?;
            let player = &e.survival_state().unwrap().player;
            return Ok((e.position(), player.x, player.y) != before);
        }
        return Ok(false);
    }
    if q == e.position() {
        // Existing tile-target melee requires stepping out of the shared tile.
        // This remains ordinary collision-respecting player input.
        let a = e
            .entity_pages()
            .find_map(|p| {
                p.entities.iter().find_map(|v| {
                    let pos = p.position(v);
                    if (pos.x, pos.y) == q {
                        v.kind.mover()
                    } else {
                        None
                    }
                })
            })
            .ok_or("lost combat target")?;
        let player = &e.survival_state().unwrap().player;
        let dx = i32::from(a.px) - player.x;
        let dy = i32::from(a.py) - player.y;
        let d = if dx == 0 && dy == 0 {
            (1, 0)
        } else if dx.abs() > dy.abs() {
            (-i64::from(dx.signum()), 0)
        } else {
            (0, -i64::from(dy.signum()))
        };
        face(h, e, d)?;
        return Ok(true);
    }
    let player = &e.survival_state().unwrap().player;
    if player.attack_time > 0 || player.stamina == 0 {
        return Ok(false);
    }
    let sword = matches!(player.active_item,Some(ActiveItem::Tool(i)) if player.inventory.tools().get(i).is_some_and(|t|t.kind==ToolKind::Sword));
    if !sword {
        tool(h, e, ToolKind::Sword)?;
    }
    face(h, e, (q.0 - e.position().0, q.1 - e.position().1))?;
    pulse(
        h,
        e,
        Input {
            attack: true,
            ..Default::default()
        },
    )?;
    Ok(false)
}
pub(in crate::app::infdev_window_smoke) fn preflight(root: &Path) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("combat-expect.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut w = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    for row in value["pages"].as_array().ok_or("missing combat pages")? {
        let key: Key = serde_json::from_value(row["key"].clone()).map_err(|e| e.to_string())?;
        if serde_json::to_value(w.entity_page(key)?).map_err(|e| e.to_string())? != row["page"] {
            return Err("combat restart entity page changed before ticks".into());
        }
    }
    if serde_json::to_value(&w.survival().unwrap().player.inventory).map_err(|e| e.to_string())?
        != value["inventory"]
    {
        return Err("combat inventory changed before ticks".into());
    }
    println!("PASS pre-GUI combat restart: exact complete pages and inventory");
    Ok(())
}
pub(in crate::app::infdev_window_smoke) fn run(
    h: &mut Host,
    e: &mut Explorer,
    root: &Path,
    mode: &str,
) -> Result<(), String> {
    h.auto_combat = mode == "combat-create";
    let mut samples = Vec::new();
    let mut keys = BTreeSet::new();
    if mode == "combat-create" {
        if e.survival_state().unwrap().player.inventory.used_slots() != 0 {
            return Err("combat begins with nonempty inventory".into());
        }
        gather(h, e, Ground::Tree, ItemId::Wood, 30, &mut samples)?;
        craft(h, e, None, "Workbench")?;
        let d = DIRS
            .into_iter()
            .find(|d| {
                e.visible_cell(e.position().0 + d.0, e.position().1 + d.1)
                    .is_some_and(|c| matches!(c.ground, Ground::Grass | Ground::Dirt))
            })
            .ok_or("no bench floor")?;
        place(h, e, ItemId::Workbench, d)?;
        for name in ["Wood Axe", "Wood Pickaxe", "Wood Sword", "Wood Sword"] {
            craft(h, e, Some(StationKind::Workbench), name)?;
        }
        navigate(h, e, (-32, -20), &mut samples)?;
        pulse(
            h,
            e,
            Input {
                pickup: true,
                ..Default::default()
            },
        )?;
        if e.depth() != -1 {
            return Err("combat natural stairs failed".into());
        }
        tool(h, e, ToolKind::Sword)?;
        let initial = e.survival_state().unwrap().player.health;
        let mut hurt = false;
        for step in 0usize..600 {
            hurt |= h.saw_enemy_hit || e.survival_state().unwrap().player.health < initial;
            keys.extend(e.entity_pages().map(|p| p.key));
            if count(e, ItemId::Cloth) > 0 && hurt {
                break;
            }
            if let Some(q) = advance(h, e, &mut samples)? {
                tool(h, e, ToolKind::Sword)?;
                if !hurt {
                    for _ in 0..90 {
                        h.frame(e)?;
                        if e.survival_state().unwrap().player.health < initial {
                            hurt = true;
                            break;
                        }
                    }
                }
                h.wait(e, |e| {
                    e.survival_state().unwrap().player.attack_time == 0
                        && e.survival_state().unwrap().player.stamina > 1
                })?;
                pulse(
                    h,
                    e,
                    Input {
                        attack: true,
                        ..Default::default()
                    },
                )?;
                for _ in 0..3 {
                    h.frame(e)?;
                }
                println!(
                    "COMBAT attack near {q:?}, health={} cloth={}",
                    e.survival_state().unwrap().player.health,
                    count(e, ItemId::Cloth)
                );
            }
            if e.survival_state().unwrap().player.health == 0 {
                return Err("combat died before first earned victory".into());
            }
            if step.is_multiple_of(50) {
                println!(
                    "COMBAT searching natural Zombie, pos={:?}, known={}",
                    e.position(),
                    zombies(e).len()
                );
            }
        }
        if !hurt || count(e, ItemId::Cloth) == 0 {
            return Err("no real wounded battle and Zombie cloth".into());
        }
        // Retreat along the already opened route using normal movement and stairs.
        // Defense must not freeze movement while new enemies keep approaching.
        navigate(h, e, (-32, -20), &mut samples)?;
        pulse(
            h,
            e,
            Input {
                pickup: true,
                ..Default::default()
            },
        )?;
        if e.depth() != 0 {
            return Err("combat return stairs failed".into());
        }
        let p = e.position();
        let (x, y) = (p.0.div_euclid(64) * 64, p.1.div_euclid(64) * 64);
        let goal = [(x - 1, p.1), (x + 64, p.1), (p.0, y - 1), (p.0, y + 64)]
            .into_iter()
            .min_by_key(|q| q.0.abs_diff(p.0) + q.1.abs_diff(p.1))
            .unwrap();
        navigate(h, e, goal, &mut samples)?;
        keys.extend(e.entity_pages().map(|p| p.key));
        h.frame(e)?;
        h.screen.save_png(&root.join("infdev-combat.png"))?;
        e.request_save_quit();
        h.wait(e, Explorer::has_finished)?;
        e.finish()?;
        let mut disk = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
        let mut pages = Vec::new();
        for key in keys {
            pages.push(serde_json::json!({"key":key,"page":disk.entity_page(key)?}));
        }
        fs_write_evidence(
            &root.join("combat-expect.json"),
            serde_json::json!({"inventory":disk.survival().unwrap().player.inventory,"pages":pages,"position":disk.position(),"hurt":hurt,"cloth":disk.survival().unwrap().player.inventory.count(ItemId::Cloth)}),
        )?;
        disk.collect_garbage()?;
        println!(
            "PASS native combat create: earned sword, natural Zombie, real injury/kill/cloth, boundary crossing and Save/Quit"
        );
        return Ok(());
    }
    if mode == "combat-death" {
        h.watch_enemy_death = true;
        let outcome = (|| -> Result<(), String> {
            if e.depth() == 0 {
                navigate(h, e, (-32, -20), &mut samples)?;
                pulse(
                    h,
                    e,
                    Input {
                        pickup: true,
                        ..Default::default()
                    },
                )?;
            }
            for step in 0usize..600 {
                if e.survival_state().unwrap().player.health == 0 {
                    return Ok(());
                }
                if advance(h, e, &mut samples)?.is_some() {
                    h.wait(e, |e| {
                        e.survival_state().unwrap().player.health == 0
                            && e.survival_state().unwrap().death_recorded
                    })?;
                    return Ok(());
                }
                if step.is_multiple_of(50) {
                    println!(
                        "COMBAT looking for natural fatal contact at {:?}",
                        e.position()
                    );
                }
            }
            Err("no natural enemy contact".into())
        })();
        h.watch_enemy_death = false;
        if e.survival_state().unwrap().player.health != 0
            || !e.survival_state().unwrap().death_recorded
        {
            return outcome.and(Err("no committed natural death".into()));
        }
        let before = h.last_alive.clone().ok_or("no observed living inventory")?;
        let at = e.position();
        if e.depth() != -1
            || e.visible_cell(at.0, at.1)
                .is_none_or(|c| matches!(c.ground, Ground::Water | Ground::Lava | Ground::Hole))
            || !zombies(e)
                .iter()
                .any(|(_, q)| q.0.abs_diff(at.0).max(q.1.abs_diff(at.1)) <= 1)
        {
            return Err("fatal scene is not a dry natural Zombie contact".into());
        }
        if e.survival_state().unwrap().player.inventory.used_slots() != 0
            || corpse(e, at) != Some(before.clone())
        {
            return Err(
                "observed last living inventory does not equal the actual death chest".into(),
            );
        }
        fs_write_evidence(
            &root.join("combat-death-expect.json"),
            serde_json::json!({"inventory":before,"position":at,"depth":e.depth(),"lastAliveObserved":true}),
        )?;
        h.frame(e)?;
        h.screen.save_png(&root.join("infdev-combat-death.png"))?;
        println!("READY_FOR_FORCED_COMBAT_TERMINATION");
        loop {
            h.frame(e)?;
        }
    }
    let v: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("combat-death-expect.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let at: Point = serde_json::from_value(v["position"].clone()).map_err(|e| e.to_string())?;
    let inventory: Inventory =
        serde_json::from_value(v["inventory"].clone()).map_err(|e| e.to_string())?;
    if e.depth() != -1
        || e.position() != at
        || e.survival_state().unwrap().player.health != 0
        || corpse(e, at) != Some(inventory.clone())
    {
        return Err("enemy death checkpoint did not restore".into());
    }
    pulse(
        h,
        e,
        Input {
            select: true,
            ..Default::default()
        },
    )?;
    navigate(h, e, (-32, -20), &mut samples)?;
    pulse(
        h,
        e,
        Input {
            pickup: true,
            ..Default::default()
        },
    )?;
    for _ in 0..400 {
        if same_items(&e.survival_state().unwrap().player.inventory, &inventory) {
            break;
        }
        let d = (at.0 - e.position().0, at.1 - e.position().1);
        if d.0.abs() + d.1.abs() == 1 {
            // DeathChest is a container, not a one-tile automatic Drop.
            // Use the ordinary facing + Enter interaction, including across pages.
            face(h, e, d)?;
            pulse(
                h,
                e,
                Input {
                    select: true,
                    ..Default::default()
                },
            )?;
            h.frame(e)?;
            continue;
        }
        if e.position() == at {
            h.frame(e)?;
            continue;
        }
        let (points, _) = path(e, Some(at), None)?;
        walk(h, e, &points[..points.len().min(2)], &mut samples)?;
    }
    let restored = e.survival_state().unwrap().player.inventory.clone();
    let restored_revision = restored.slot_revision();
    for _ in 0..3 {
        h.frame(e)?;
    }
    if !same_items(&restored, &inventory)
        || e.survival_state().unwrap().player.inventory != restored
        || corpse(e, at).is_some()
    {
        fs_write_evidence(
            &root.join("combat-failure.json"),
            serde_json::json!({"expected":inventory,"actual":e.survival_state().unwrap().player.inventory,"health":e.survival_state().unwrap().player.health,"deathPosition":at,"position":e.position(),"corpse":corpse(e,at),"status":e.status()}),
        )?;
        return Err(
            "enemy death retrieval duplicated/lost inventory; see combat-failure.json".into(),
        );
    }
    e.request_save_quit();
    h.wait(e, Explorer::has_finished)?;
    e.finish()?;
    let mut disk = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    if disk.survival().unwrap().player.inventory != restored {
        return Err("post enemy death disk inventory mismatch".into());
    }
    disk.collect_garbage()?;
    fs_write_evidence(
        &root.join("combat-result.json"),
        serde_json::json!({"enemyDeath":true,"retrieved":true,"sourceSlotRevision":inventory.slot_revision(),"restoredSlotRevision":restored_revision,"position":disk.position(),"inventory":disk.survival().unwrap().player.inventory}),
    )?;
    println!(
        "PASS native enemy death: real contact, forced termination, restore, respawn, stair return and exact inventory retrieval/GC"
    );
    Ok(())
}
