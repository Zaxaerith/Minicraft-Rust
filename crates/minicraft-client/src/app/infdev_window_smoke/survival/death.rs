//! Real menus/actions and naturally earned inventory. Only the fatal damage is
//! controlled. Python force-kills the first window AFTER the death checkpoint.
use super::*;
use crate::{
    experimental::infdev::{entities::Kind, paged::Key},
    item::Inventory,
};

fn corpse(e: &Explorer, at: Point) -> Option<Inventory> {
    e.entity_pages()
        .flat_map(|p| {
            p.entities.iter().filter_map(move |entity| {
                let pos = p.position(entity);
                let Kind::DeathChest(chest) = &entity.kind else {
                    return None;
                };
                ((pos.x, pos.y) == at && pos.depth == e.depth()).then(|| chest.inventory.clone())
            })
        })
        .next()
}
pub(super) fn run(h: &mut Host, e: &mut Explorer, root: &Path, mode: &str) -> Result<(), String> {
    let evidence = root.join("death-expect.json");
    let directory = root.join("infdev-survival-v4/Window42");
    let mut samples = Vec::new();
    if mode == "death-create" {
        if e.survival_state().unwrap().player.inventory.used_slots() != 0 {
            return Err("nonempty initial inventory".into());
        }
        gather(h, e, Ground::Tree, ItemId::Wood, 20, &mut samples)?;
        craft(h, e, None, "Workbench")?;
        let d = match e.survival_state().unwrap().player.direction {
            crate::world::Direction::Down => (0, 1),
            crate::world::Direction::Up => (0, -1),
            crate::world::Direction::Left => (-1, 0),
            crate::world::Direction::Right => (1, 0),
        };
        place(h, e, ItemId::Workbench, d)?;
        craft(h, e, Some(StationKind::Workbench), "Wood Axe")?;
        craft(h, e, Some(StationKind::Workbench), "Wood Pickaxe")?;
        gather(h, e, Ground::Rock, ItemId::Stone, 5, &mut samples)?;
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
            return Err("natural underground stair failed".into());
        }
        gather(h, e, Ground::IronOre, ItemId::IronOre, 1, &mut samples)?;
        let before = e.survival_state().unwrap().player.inventory.clone();
        let at = e.position();
        fs_write_evidence(
            &evidence,
            serde_json::json!({"inventory":before,"position":at,"depth":e.depth()}),
        )?;
        let serial = e.action_serial();
        e.diagnostic_death();
        h.wait(e, |e| {
            e.action_serial() != serial
                && e.survival_state().unwrap().player.health == 0
                && e.survival_state().unwrap().death_recorded
        })?;
        if e.status().contains("FAILED")
            || e.survival_state().unwrap().player.inventory.used_slots() != 0
            || corpse(e, at) != Some(before)
        {
            return Err(
                "native death did not move exactly one inventory into the persisted chest".into(),
            );
        }
        h.frame(e)?;
        h.screen.save_png(&root.join("infdev-death.png"))?;
        println!("READY_FOR_FORCED_DEATH_TERMINATION");
        loop {
            h.frame(e)?;
        } // Only Python's owned child is terminated; no Save/Quit.
    }
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let at: Point = serde_json::from_value(value["position"].clone()).map_err(|e| e.to_string())?;
    let expected: Inventory =
        serde_json::from_value(value["inventory"].clone()).map_err(|e| e.to_string())?;
    if mode == "death-recover" {
        if e.depth() != -1
            || e.position() != at
            || e.survival_state().unwrap().player.health != 0
            || e.survival_state().unwrap().player.inventory.used_slots() != 0
            || corpse(e, at) != Some(expected.clone())
        {
            return Err("forced-termination restart did not restore a single chest owner".into());
        }
        pulse(
            h,
            e,
            Input {
                select: true,
                ..Default::default()
            },
        )?;
        if e.depth() != 0 || e.survival_state().unwrap().player.inventory.used_slots() != 0 {
            return Err("respawn remotely duplicated/retrieved the underground chest".into());
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
        navigate(h, e, (at.0, at.1 - 1), &mut samples)?;
        face(h, e, (0, 1))?;
        h.wait(e, |e| {
            e.survival_state().unwrap().drops.iter().all(|d| {
                d.depth != e.depth()
                    || d.x.abs_diff(e.position().0) > 1
                    || d.y.abs_diff(e.position().1) > 1
            })
        })?;
        let mut combined = e.survival_state().unwrap().player.inventory.clone();
        for &s in expected.slots() {
            if combined.add_stack(s) != 0 {
                return Err("return trip inventory unexpectedly full".into());
            }
        }
        for &t in expected.tools() {
            combined.add_tool(t).ok_or("return trip tool slots full")?;
        }
        pulse(
            h,
            e,
            Input {
                select: true,
                ..Default::default()
            },
        )?;
        if e.survival_state().unwrap().player.inventory != combined || corpse(e, at).is_some() {
            return Err("real container retrieval lost/duplicated instance state".into());
        }
        pulse(
            h,
            e,
            Input {
                select: true,
                ..Default::default()
            },
        )?;
        if e.survival_state().unwrap().player.inventory != combined {
            return Err("duplicate interact granted inventory twice".into());
        }
        h.frame(e)?;
        h.screen.save_png(&root.join("infdev-recovered.png"))?;
        let mut result = value.clone();
        result["recovered"] = serde_json::to_value(&combined).map_err(|e| e.to_string())?;
        result["recovered_position"] = serde_json::json!(e.position());
        fs_write_evidence(&evidence, result)?;
    } else {
        let inventory: Inventory =
            serde_json::from_value(value["recovered"].clone()).map_err(|e| e.to_string())?;
        if e.survival_state().unwrap().player.inventory != inventory || corpse(e, at).is_some() {
            return Err("post-retrieval restart duplicated chest or lost inventory".into());
        }
    }
    e.request_save_quit();
    h.wait(e, Explorer::has_finished)?;
    e.finish()?;
    let mut disk = paged::World::open(&directory, 1)?;
    if !disk
        .entity_page(Key::at_depth(-1, at.0, at.1))?
        .entities
        .is_empty()
    {
        return Err("retrieved chest remained in persisted chunk".into());
    }
    disk.collect_garbage()?;
    println!(
        "PASS native death {mode}: natural resources/tools -> underground death -> forced termination -> respawn -> return/real container -> duplicate refusal -> Save/Quit/restart + GC"
    );
    Ok(())
}
