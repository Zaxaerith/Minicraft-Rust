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
fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
// Read only after World::open has validated the complete immutable tree.
// Inspect every referenced page, not just the GUI's currently visible pages.
fn snapshot(w: &paged::World, root: &Path) -> Result<serde_json::Value, String> {
    let dir = root.join("infdev-survival-v4/Window42");
    let checkpoint = read_json(&dir.join("checkpoint.json"))?;
    let mut pending = vec![checkpoint["entity_root"].clone()];
    let mut pages = std::collections::BTreeMap::new();
    let object_path = |reference: &serde_json::Value, domain: &str, suffix: &str| {
        let hash = reference["hash"].as_u64().ok_or("missing object hash")?;
        let slot = reference["slot"].as_u64().ok_or("missing object slot")?;
        Ok::<_, String>(
            dir.join(domain)
                .join(format!("{:02x}", hash >> 56))
                .join(format!("{hash:016x}-{slot:04x}.{suffix}")),
        )
    };
    while let Some(reference) = pending.pop() {
        if reference.is_null() {
            continue;
        }
        let node = read_json(&object_path(&reference, "nodes", "node")?)?;
        pending.extend([node["left"].clone(), node["right"].clone()]);
        let page = read_json(&object_path(&node["chunk"], "entities", "entity")?)?;
        let key = serde_json::to_string(&page["key"]).map_err(|e| e.to_string())?;
        pages.insert(key, page);
    }
    let s = w.survival().ok_or("missing saved Survival")?;
    Ok(
        serde_json::json!({"environment":s.environment,"inventory":s.player.inventory,
        "position":w.position(),"depth":w.depth(),"pages":pages}),
    )
}
fn expected_file(mode: &str) -> &str {
    match mode {
        "environment-reopen" => "environment-expect.json",
        "environment-night" => "environment-result.json",
        "environment-night-reopen" => "environment-night-expect.json",
        _ => unreachable!("environment mode checked by caller"),
    }
}
pub(in crate::app::infdev_window_smoke) fn preflight(
    root: &Path,
    mode: &str,
) -> Result<(), String> {
    let expected = read_json(&root.join(expected_file(mode)))?;
    let w = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    let actual = snapshot(&w, root)?;
    if actual != expected["snapshot"] {
        return Err(
            "pre-tick clock/inventory/position/complete entity pages changed on restart".into(),
        );
    }
    println!(
        "PASS {mode} pre-GUI: exact clock, inventory, position and ALL {} saved entity pages",
        actual["pages"].as_object().unwrap().len()
    );
    Ok(())
}
fn save_quit(h: &mut Host, e: &mut Explorer, root: &Path, mode: &str) -> Result<(), String> {
    // Ordinary Escape Input through Explorer; no alternate checkpoint writer.
    e.tick(&Input {
        exit: true,
        ..Default::default()
    });
    h.wait(e, Explorer::has_finished)?;
    e.finish()?;
    let mut w = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    let state = snapshot(&w, root)?;
    w.collect_garbage()?;
    if snapshot(&w, root)? != state {
        return Err("GC changed saved state".into());
    }
    let filename = match mode {
        "environment-create" => "environment-expect.json",
        "environment-reopen" => "environment-result.json",
        "environment-night" => "environment-night-expect.json",
        "environment-night-reopen" => "environment-night-result.json",
        _ => unreachable!(),
    };
    fs_write_evidence(
        &root.join(filename),
        serde_json::json!({"mode":mode,"snapshot":state,
        "normalEscapeSaveQuit":true,"gc":true}),
    )?;
    println!("PASS {mode}: normal Escape Save & Quit, disk reopen and GC");
    Ok(())
}
fn wait_night(h: &mut Host, e: &mut Explorer, root: &Path) -> Result<(), String> {
    let before = e
        .survival_state()
        .unwrap()
        .environment
        .as_ref()
        .unwrap()
        .clone();
    if before.night() {
        return Err("night wait started after night; not a day/night test".into());
    }
    h.frame(e)?;
    h.screen.save_png(&root.join("environment-day.png"))?;
    let start = Instant::now();
    let mut reported = before.day_tick;
    let mut evening = false;
    println!(
        "ENV separate real-time wait: start={}, first night=48600; NO time injection/fast-forward",
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
        let tick = e
            .survival_state()
            .unwrap()
            .environment
            .as_ref()
            .unwrap()
            .day_tick;
        h.window.set_title(&format!(
            "Minicraft TEST - waiting NIGHT: {tick}/48600 (real time)"
        ));
        h.frame(e)?;
        if !evening && tick >= 32400 {
            h.screen.save_png(&root.join("environment-evening.png"))?;
            evening = true;
        }
        if tick.saturating_sub(reported) >= 3600 {
            println!(
                "ENV real-time tick={tick} elapsed_ms={}",
                start.elapsed().as_millis()
            );
            reported = tick;
        }
        if e.survival_state().unwrap().player.health == 0 {
            return Err("night wait died".into());
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    h.frame(e)?;
    h.screen.save_png(&root.join("environment-night.png"))?;
    let after = e
        .survival_state()
        .unwrap()
        .environment
        .as_ref()
        .unwrap()
        .clone();
    let elapsed = start.elapsed().as_millis();
    if u128::from(after.day_tick - before.day_tick) > elapsed * 60 / 1000 + 120 {
        return Err("observed calendar faster than the normal 60Hz clock".into());
    }
    fs_write_evidence(
        &root.join("environment-night-time.json"),
        serde_json::json!({
        "before":before,"after":after,"elapsedMs":elapsed,"eveningObserved":evening,
        "naturalClock":true}),
    )?;
    println!("PASS independent real-time day/evening/night, elapsed_ms={elapsed}");
    Ok(())
}
pub(in crate::app::infdev_window_smoke) fn run(
    h: &mut Host,
    e: &mut Explorer,
    root: &Path,
    mode: &str,
) -> Result<(), String> {
    let creating = mode == "environment-create";
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
        h.frame(e)?;
        h.screen.save_png(&root.join("environment-day.png"))?;
        println!(
            "PASS short acquisition/combat: Slime={} Cloth={}, two naturally crafted/placed lights; no night wait",
            count(e, ItemId::Slime),
            count(e, ItemId::Cloth)
        );
    } else if mode == "environment-night" {
        wait_night(h, e, root)?;
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
            || mode == "environment-night-reopen" && !start.as_ref().is_some_and(Environment::night)
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
    save_quit(h, e, root, mode)?;
    if creating {
        let mut w = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
        for (at, entity) in lights {
            if !w.entity_page(at.key())?.entities.contains(&entity) {
                return Err("created Torch identity lost at Save & Quit".into());
            }
        }
    }
    Ok(())
}
