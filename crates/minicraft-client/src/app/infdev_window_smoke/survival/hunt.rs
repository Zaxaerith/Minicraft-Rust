//! Actual naturally spawned pigs, earned drops and food through normal Input.
use super::*;
use crate::experimental::infdev::{
    entities::{Kind, dynamic::Birth},
    paged::{self, Key},
};
use std::collections::BTreeSet;
fn pigs(e: &Explorer) -> Vec<(Birth, Point)> {
    e.entity_pages()
        .flat_map(|p| {
            p.entities.iter().filter_map(move |entity| {
                if !matches!(entity.kind, Kind::Pig(_)) {
                    return None;
                }
                let pos = p.position(entity);
                Some((entity.birth?, (pos.x, pos.y)))
            })
        })
        .collect()
}
pub(in crate::app::infdev_window_smoke) fn preflight(root: &Path) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("hunt-expect.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut disk = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    for row in value["pages"].as_array().ok_or("missing entity evidence")? {
        let key: Key = serde_json::from_value(row["key"].clone()).map_err(|e| e.to_string())?;
        if serde_json::to_value(disk.entity_page(key)?).map_err(|e| e.to_string())? != row["page"] {
            return Err("cross-process persistent animal/drop page changed before loading".into());
        }
    }
    if serde_json::to_value(&disk.survival().unwrap().player.inventory)
        .map_err(|e| e.to_string())?
        != value["inventory"]
    {
        return Err("cross-process hunted inventory changed".into());
    }
    println!("PASS pre-GUI restart: complete saved animal/drop pages and inventory exactly equal");
    Ok(())
}
pub(in crate::app::infdev_window_smoke) fn run(
    h: &mut Host,
    e: &mut Explorer,
    root: &Path,
    creating: bool,
) -> Result<(), String> {
    let mut keys = BTreeSet::new();
    let mut earned = 0;
    if creating {
        if e.survival_state().unwrap().player.inventory.used_slots() != 0 {
            return Err("hunt starts with nonempty bag".into());
        }
        // Natural resources also exercise the new paged mining/pickup route.
        let mut samples = Vec::new();
        gather(h, e, Ground::Tree, ItemId::Wood, 20, &mut samples)?;
        craft(h, e, None, "Workbench")?;
        let facing = DIRS
            .into_iter()
            .find(|d| {
                e.visible_cell(e.position().0 + d.0, e.position().1 + d.1)
                    .is_some_and(|c| matches!(c.ground, Ground::Grass | Ground::Dirt))
            })
            .ok_or("no natural workstation position")?;
        let bench = place(h, e, ItemId::Workbench, facing)?;
        craft(h, e, Some(StationKind::Workbench), "Wood Axe")?;
        tool(h, e, ToolKind::Axe)?;
        println!(
            "HUNT earned natural axe at {:?}; bench {bench:?}",
            e.position()
        );
        for attempt in 0_usize..80 {
            keys.extend(e.entity_pages().map(|p| p.key));
            if count(e, ItemId::RawPork) > 0 {
                earned = count(e, ItemId::RawPork);
                break;
            }
            if pigs(e).is_empty() {
                // Exploration is part of the test: do not wait indefinitely
                // for a naturally spawned animal outside this viewport.
                for _ in 0..60 {
                    h.frame(e)?;
                }
                if pigs(e).is_empty() {
                    let d = DIRS[(attempt / 4) % DIRS.len()];
                    let goal = (e.position().0 + d.0 * 16, e.position().1 + d.1 * 16);
                    println!("HUNT exploring natural animal habitat toward {goal:?}");
                    navigate(h, e, goal, &mut samples)?;
                    continue;
                }
            }
            let (_, pig) = pigs(e)
                .into_iter()
                .min_by_key(|(_, p)| p.0.abs_diff(e.position().0) + p.1.abs_diff(e.position().1))
                .unwrap();
            let distance = pig.0.abs_diff(e.position().0) + pig.1.abs_diff(e.position().1);
            if distance == 1 {
                face(h, e, (pig.0 - e.position().0, pig.1 - e.position().1))?;
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
                // Permit the existing tick/auto-pickup to run, never add loot.
                for _ in 0..12 {
                    h.frame(e)?;
                }
            } else {
                let goal = DIRS
                    .into_iter()
                    .map(|d| (pig.0 + d.0, pig.1 + d.1))
                    .filter(|p| {
                        e.visible_cell(p.0, p.1).is_some_and(|c| {
                            !survival::tile(c).solid(c.data) && c.ground != Ground::Water
                        })
                    })
                    .min_by_key(|p| p.0.abs_diff(e.position().0) + p.1.abs_diff(e.position().1))
                    .ok_or("no reachable natural pig approach")?;
                navigate(h, e, goal, &mut samples)?;
            }
            if attempt.is_multiple_of(10) {
                println!("HUNT chasing natural pig {pig:?} from {:?}", e.position());
            }
        }
        if earned == 0 {
            return Err("did not obtain naturally hunted Raw Pork".into());
        }
        h.frame(e)?;
        h.screen.save_png(&root.join("infdev-hunt.png"))?;
        let start = e.position();
        // Cross negative/positive chunk boundaries with collision and mining.
        let edge = start.0.div_euclid(64) * 64;
        let goal = if start.0.rem_euclid(64) < 32 {
            (edge - 1, start.1)
        } else {
            (edge + 64, start.1)
        };
        navigate(h, e, goal, &mut samples)?;
        keys.extend(e.entity_pages().map(|p| p.key));
        println!(
            "HUNT obtained {earned} real pork and crossed to {:?}; continuing natural harvesting until hungry",
            e.position()
        );
        while e.survival_state().unwrap().player.hunger == 10 {
            let more = count(e, ItemId::Wood)
                .checked_add(12)
                .ok_or("harvest count overflow")?;
            gather(h, e, Ground::Tree, ItemId::Wood, more, &mut samples)?;
        }
        h.wait(e, |e| {
            e.survival_state().unwrap().player.attack_time == 0
                && e.survival_state().unwrap().player.stamina > 0
        })?;
        stack(h, e, ItemId::RawPork)?;
        let before = e.survival_state().unwrap().player.hunger;
        let n = count(e, ItemId::RawPork);
        pulse(
            h,
            e,
            Input {
                attack: true,
                ..Default::default()
            },
        )?;
        if count(e, ItemId::RawPork) != n - 1
            || e.survival_state().unwrap().player.hunger != before + 1
        {
            return Err("naturally hunted pork was not consumed by original food action".into());
        }
        println!(
            "PASS naturally hunted {earned} Raw Pork; ate1; crossed to {:?}",
            e.position()
        );
    }
    e.request_save_quit();
    h.wait(e, Explorer::has_finished)?;
    e.finish()?;
    let mut disk = paged::World::open(&root.join("infdev-survival-v4/Window42"), 1)?;
    if creating {
        let mut pages = Vec::new();
        for key in keys {
            pages.push(serde_json::json!({"key":key,"page":disk.entity_page(key)?}));
        }
        fs_write_evidence(
            &root.join("hunt-expect.json"),
            serde_json::json!({"inventory":disk.survival().unwrap().player.inventory,"position":disk.position(),"earnedPork":earned,"atePork":1,"pages":pages}),
        )?;
    }
    disk.collect_garbage()?;
    println!(
        "PASS native hunt {creating}: natural animals/hits/drops/food, chunk exploration, Save/Quit/restart + GC"
    );
    Ok(())
}
