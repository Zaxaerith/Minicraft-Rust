//! Real-time native window exercise. No time skips, inventory/enemy injection,
//! teleportation, or bypass of the normal worker/Input/container transaction.
use super::*;
use crate::experimental::infdev::{
    entities::{Entity, Kind, Position},
    environment::Environment,
};

fn torches(e: &Explorer) -> Vec<(Position, Entity)> {
    e.entity_pages()
        .flat_map(|p| {
            p.entities
                .iter()
                .filter(|v| matches!(v.kind, Kind::Torch))
                .map(move |v| (p.position(v), v.clone()))
        })
        .collect()
}
fn put_torch(h: &mut Host, e: &mut Explorer) -> Result<(Position, Entity), String> {
    let d = DIRS
        .into_iter()
        .find(|d| {
            e.visible_cell(e.position().0 + d.0, e.position().1 + d.1)
                .is_some_and(|c| matches!(c.ground, Ground::Dirt | Ground::Grass | Ground::Sand))
        })
        .ok_or("no natural torch substrate")?;
    stack(h, e, ItemId::Torch)?;
    face(h, e, d)?;
    h.wait(e, |e| e.survival_state().unwrap().player.attack_time == 0)?;
    let at = Position {
        depth: e.depth(),
        x: e.position().0 + d.0,
        y: e.position().1 + d.1,
    };
    let before = count(e, ItemId::Torch);
    pulse(
        h,
        e,
        Input {
            attack: true,
            ..Default::default()
        },
    )?;
    h.wait(e, |e| torches(e).iter().any(|(p, _)| *p == at))?;
    if count(e, ItemId::Torch) != before - 1 {
        return Err("torch did not consume exactly one earned item".into());
    }
    torches(e)
        .into_iter()
        .find(|(p, _)| *p == at)
        .ok_or("missing natural placed torch".into())
}
pub(in crate::app::infdev_window_smoke) fn preflight(root: &Path) -> Result<(), String> {
    let expected: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("environment-expect.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut w = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    if serde_json::to_value(w.survival().unwrap().environment.clone()).map_err(|e| e.to_string())?
        != expected["environment"]
        || serde_json::to_value(&w.survival().unwrap().player.inventory)
            .map_err(|e| e.to_string())?
            != expected["inventory"]
    {
        return Err("pre-tick environment/inventory changed on restart".into());
    }
    for row in expected["torches"]
        .as_array()
        .ok_or("missing torch evidence")?
    {
        let at: Position = serde_json::from_value(row[0].clone()).map_err(|e| e.to_string())?;
        let entity: Entity = serde_json::from_value(row[1].clone()).map_err(|e| e.to_string())?;
        if !w.entity_page(at.key())?.entities.contains(&entity) {
            return Err("torch identity/position was lost".into());
        }
    }
    println!("PASS environment pre-GUI restart: exact clock, inventory and paged torch identities");
    Ok(())
}
pub(in crate::app::infdev_window_smoke) fn run(
    h: &mut Host,
    e: &mut Explorer,
    root: &Path,
    creating: bool,
) -> Result<(), String> {
    let mut samples = Vec::new();
    let mut lights = Vec::new();
    if creating {
        h.auto_combat = true;
        h.saw_enemy_hit = true;
        if e.survival_state().unwrap().player.inventory.used_slots() != 0 {
            return Err("environment requires naturally empty start".into());
        }
        gather(h, e, Ground::Tree, ItemId::Wood, 40, &mut samples)?;
        craft(h, e, None, "Workbench")?;
        let d = DIRS
            .into_iter()
            .find(|d| {
                e.visible_cell(e.position().0 + d.0, e.position().1 + d.1)
                    .is_some_and(|c| matches!(c.ground, Ground::Grass | Ground::Dirt))
            })
            .ok_or("no earned bench floor")?;
        place(h, e, ItemId::Workbench, d)?;
        for name in ["Wood Axe", "Wood Pickaxe", "Wood Sword", "Wood Sword"] {
            craft(h, e, Some(StationKind::Workbench), name)?;
        }
        gather(h, e, Ground::Rock, ItemId::Coal, 2, &mut samples)?;
        craft(h, e, None, "Torch")?;
        craft(h, e, None, "Torch")?;
        if count(e, ItemId::Torch) != 4 {
            return Err("shared Torch recipe did not produce four real torches".into());
        }
        println!(
            "PASS environment: naturally earned wood/coal, workbench, tools/swords and four torches"
        );
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
            return Err("natural stairs did not descend".into());
        }
        lights.push(put_torch(h, e)?);
        tool(h, e, ToolKind::Sword)?;
        for step in 0usize..600 {
            if count(e, ItemId::Slime) > 0 && count(e, ItemId::Cloth) > 0 {
                break;
            }
            if super::combat::defend(h, e)? {
                continue;
            }
            let _ = super::combat::advance(h, e, &mut samples)?;
            if step.is_multiple_of(50) {
                println!(
                    "ENV natural battle at {:?}, Slime={} Cloth={}",
                    e.position(),
                    count(e, ItemId::Slime),
                    count(e, ItemId::Cloth)
                );
            }
        }
        if count(e, ItemId::Slime) == 0 || count(e, ItemId::Cloth) == 0 {
            return Err("natural Slime/Zombie loot was not earned".into());
        }
        h.frame(e)?;
        h.screen
            .save_png(&root.join("environment-underground.png"))?;
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
            return Err("natural stairs did not return".into());
        }
        lights.push(put_torch(h, e)?);
        h.auto_combat = false;
        let before = e
            .survival_state()
            .unwrap()
            .environment
            .as_ref()
            .unwrap()
            .clone();
        if before.night() {
            return Err("day capture unexpectedly already night".into());
        }
        h.frame(e)?;
        h.screen.save_png(&root.join("environment-day.png"))?;
        let mut reported = before.day_tick;
        println!(
            "ENV real-time wait for first night, tick={}; no calendar injection/fast-forward",
            before.day_tick
        );
        while !e
            .survival_state()
            .unwrap()
            .environment
            .as_ref()
            .unwrap()
            .night()
        {
            h.frame(e)?;
            let tick = e
                .survival_state()
                .unwrap()
                .environment
                .as_ref()
                .unwrap()
                .day_tick;
            if tick.saturating_sub(reported) >= 3600 {
                println!("ENV real-time calendar tick={tick}");
                reported = tick;
            }
            if e.survival_state().unwrap().player.health == 0 {
                return Err("real-time clock wait died".into());
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        h.frame(e)?;
        h.screen.save_png(&root.join("environment-night.png"))?;
        println!("PASS environment: real-time surface day/evening/night, no debug time changes");
    } else {
        let start = e.survival_state().unwrap().environment.clone();
        pulse(
            h,
            e,
            Input {
                menu: true,
                ..Default::default()
            },
        )?;
        let paused = e.survival_state().unwrap().environment.clone();
        for _ in 0..120 {
            h.frame(e)?;
        }
        if e.survival_state().unwrap().environment != paused
            || !start.as_ref().is_some_and(Environment::night)
        {
            return Err("calendar advanced in menu or night failed to restore".into());
        }
        pulse(
            h,
            e,
            Input {
                exit: true,
                ..Default::default()
            },
        )?;
    }
    e.request_save_quit();
    h.wait(e, Explorer::has_finished)?;
    e.finish()?;
    let mut w = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    if creating {
        fs_write_evidence(
            &root.join("environment-expect.json"),
            serde_json::json!({"environment":w.survival().unwrap().environment,"inventory":w.survival().unwrap().player.inventory,"torches":lights,"naturalSlime":true,"naturalZombie":true,"realTimeNight":true}),
        )?;
    } else {
        w.collect_garbage()?;
        fs_write_evidence(
            &root.join("environment-result.json"),
            serde_json::json!({"restoredNight":true,"menuPaused":true,"gc":true,"environment":w.survival().unwrap().environment}),
        )?;
    }
    Ok(())
}
