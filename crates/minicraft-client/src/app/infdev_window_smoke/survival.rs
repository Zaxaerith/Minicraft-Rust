//! Diagnostic autopilot: sends only the public GUI Input adapter. No inventory
//! injection, teleports, terrain edits, or alternate game-action implementation.
use super::*;
use crate::{
    experimental::infdev::survival::{self, Loot},
    item::{ItemId, ToolKind},
    world::ActiveItem,
};
use std::collections::{BTreeMap, BinaryHeap};

fn pulse(h: &mut Host, e: &mut Explorer, input: Input) -> Result<(), String> {
    h.wait(e, Explorer::is_ready)?;
    let before = e.action_serial();
    e.tick(&input);
    h.wait(e, |e| e.action_serial() != before && e.is_ready())
}
fn face(h: &mut Host, e: &mut Explorer, d: (i64, i64)) -> Result<(), String> {
    pulse(
        h,
        e,
        Input {
            left: d.0 < 0,
            right: d.0 > 0,
            up: d.1 < 0,
            down: d.1 > 0,
            ..Default::default()
        },
    )
}
fn count(e: &Explorer, id: ItemId) -> u16 {
    e.survival_state().unwrap().player.inventory.count(id)
}
fn select(h: &mut Host, e: &mut Explorer, selection: usize) -> Result<(), String> {
    pulse(
        h,
        e,
        Input {
            menu: true,
            ..Default::default()
        },
    )?;
    for _ in 0..selection {
        pulse(
            h,
            e,
            Input {
                cursor_down_pressed: true,
                ..Default::default()
            },
        )?;
    }
    pulse(
        h,
        e,
        Input {
            attack: true,
            ..Default::default()
        },
    )
}
fn tool(h: &mut Host, e: &mut Explorer, kind: ToolKind) -> Result<(), String> {
    let s = e.survival_state().unwrap();
    let i = s
        .player
        .inventory
        .tools()
        .iter()
        .enumerate()
        .filter(|(_, t)| t.kind == kind)
        .max_by_key(|(_, t)| t.tier.level())
        .map(|(i, _)| s.player.inventory.slots().len() + i);
    if let Some(i) = i {
        select(h, e, i)?;
    }
    Ok(())
}
fn stack(h: &mut Host, e: &mut Explorer, id: ItemId) -> Result<(), String> {
    let i = e
        .survival_state()
        .unwrap()
        .player
        .inventory
        .slots()
        .iter()
        .position(|s| s.item == id)
        .ok_or("earned stack missing")?;
    select(h, e, i)
}
fn craft(h: &mut Host, e: &mut Explorer, bench: bool, name: &str) -> Result<(), String> {
    pulse(
        h,
        e,
        if bench {
            Input {
                select: true,
                ..Default::default()
            }
        } else {
            Input {
                craft: true,
                ..Default::default()
            }
        },
    )?;
    if e.survival_state().unwrap().panel != survival::Panel::Crafting(bench) {
        return Err("actual workbench menu did not open".into());
    }
    let i = survival::recipes(bench)
        .iter()
        .position(|r| r.product_name() == name)
        .ok_or("existing recipe missing")?;
    for _ in 0..i {
        pulse(
            h,
            e,
            Input {
                cursor_down_pressed: true,
                ..Default::default()
            },
        )?;
    }
    pulse(
        h,
        e,
        Input {
            attack: true,
            ..Default::default()
        },
    )?;
    if e.status().contains("FAILED") {
        return Err(e.status().into());
    }
    pulse(
        h,
        e,
        Input {
            exit: true,
            ..Default::default()
        },
    )
}

type Point = (i64, i64);
const DIRS: [Point; 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
fn path(
    e: &Explorer,
    goal: Option<Point>,
    resource: Option<Ground>,
) -> Result<(Vec<Point>, Option<Point>), String> {
    let start = e.position();
    let mut costs = BTreeMap::from([(start, 0_u32)]);
    let mut previous = BTreeMap::new();
    let mut queue = BinaryHeap::from([(std::cmp::Reverse(0_u32), start)]);
    let mut best = start;
    while let Some((std::cmp::Reverse(cost), p)) = queue.pop() {
        if costs.get(&p) != Some(&cost) {
            continue;
        }
        if Some(p) == goal {
            best = p;
            break;
        }
        if let Some(g) = goal
            && p.0.abs_diff(g.0) + p.1.abs_diff(g.1) < best.0.abs_diff(g.0) + best.1.abs_diff(g.1)
        {
            best = p;
        }
        if let Some(g) = resource {
            for (dx, dy) in DIRS {
                if e.visible_cell(p.0 + dx, p.1 + dy)
                    .is_some_and(|c| c.ground == g)
                {
                    let mut result = vec![p];
                    let mut back = p;
                    while let Some(&parent) = previous.get(&back) {
                        result.push(parent);
                        back = parent;
                    }
                    result.reverse();
                    return Ok((result, Some((dx, dy))));
                }
            }
        }
        for (dx, dy) in DIRS {
            let next = (p.0 + dx, p.1 + dy);
            let Some(c) = e.visible_cell(next.0, next.1) else {
                continue;
            };
            if e.survival_state()
                .unwrap()
                .station_at(e.depth(), next.0, next.1)
            {
                continue;
            }
            let weight = match c.ground {
                Ground::Tree => 16,
                Ground::Rock => 40,
                Ground::Water if resource.is_none() => 8,
                Ground::Water
                | Ground::Lava
                | Ground::Hole
                | Ground::Cactus
                | Ground::IronOre
                | Ground::GoldOre
                | Ground::GemOre
                | Ground::Lapis => continue,
                _ if survival::tile(c).solid(c.data) => continue,
                _ => 1,
            };
            let next_cost = cost + weight;
            if costs.get(&next).is_none_or(|&old| next_cost < old) {
                costs.insert(next, next_cost);
                previous.insert(next, p);
                queue.push((std::cmp::Reverse(next_cost), next));
            }
        }
    }
    if resource.is_some() {
        return Err(format!(
            "no naturally reachable {resource:?} in current cached terrain at {:?}",
            e.position()
        ));
    }
    if best == start && Some(start) != goal {
        return Err("no visible progress toward natural destination".into());
    }
    let mut result = vec![best];
    while let Some(&p) = previous.get(result.last().unwrap()) {
        result.push(p);
    }
    result.reverse();
    Ok((result, None))
}
fn mine(
    h: &mut Host,
    e: &mut Explorer,
    d: Point,
    g: Ground,
    samples: &mut Vec<(i8, Point)>,
) -> Result<(), String> {
    tool(
        h,
        e,
        if g == Ground::Tree {
            ToolKind::Axe
        } else {
            ToolKind::Pickaxe
        },
    )?;
    face(h, e, d)?;
    let p = (e.position().0 + d.0, e.position().1 + d.1);
    for _ in 0..200 {
        h.wait(e, |e| e.visible_cell(p.0, p.1).is_some())?;
        if e.visible_cell(p.0, p.1).unwrap().ground != g {
            samples.push((e.depth(), p));
            return Ok(());
        }
        h.wait(e, |e| {
            let p = &e.survival_state().unwrap().player;
            let needed = if matches!(p.active_item, Some(ActiveItem::Tool(_))) {
                7
            } else {
                1
            };
            p.stamina >= needed && p.attack_time == 0
        })?;
        pulse(
            h,
            e,
            Input {
                attack: true,
                ..Default::default()
            },
        )?;
        if e.survival_state().unwrap().player.health == 0 {
            return Err("autopilot died during normal traversal".into());
        }
    }
    Err("normal mining did not terminate".into())
}
fn walk(
    h: &mut Host,
    e: &mut Explorer,
    points: &[Point],
    samples: &mut Vec<(i8, Point)>,
) -> Result<(), String> {
    for &next in points.iter().skip(1) {
        h.wait(e, |e| e.visible_cell(next.0, next.1).is_some())?;
        let d = (next.0 - e.position().0, next.1 - e.position().1);
        let c = e.visible_cell(next.0, next.1).unwrap();
        if matches!(c.ground, Ground::Tree | Ground::Rock) {
            mine(h, e, d, c.ground, samples)?;
        }
        loop {
            let p = &e.survival_state().unwrap().player;
            let (x, y) = e.position();
            if (x, y) == next && p.x == 8 && p.y == 8 {
                break;
            }
            let d = if x < next.0 || (x == next.0 && p.x < 8) {
                (1, 0)
            } else if x > next.0 || p.x > 8 {
                (-1, 0)
            } else if y < next.1 || (y == next.1 && p.y < 8) {
                (0, 1)
            } else {
                (0, -1)
            };
            face(h, e, d)?;
            if e.survival_state().unwrap().player.health == 0 {
                return Err("autopilot died while walking".into());
            }
        }
    }
    Ok(())
}
fn navigate(
    h: &mut Host,
    e: &mut Explorer,
    goal: Point,
    samples: &mut Vec<(i8, Point)>,
) -> Result<(), String> {
    for _ in 0..32 {
        if e.position() == goal {
            return Ok(());
        }
        h.wait(e, |e| {
            [-11, 11].into_iter().all(|dx| {
                [-8, 8].into_iter().all(|dy| {
                    e.visible_cell(e.position().0 + dx, e.position().1 + dy)
                        .is_some()
                })
            })
        })?;
        let (steps, _) = path(e, Some(goal), None)?;
        walk(h, e, &steps, samples)?;
    }
    Err("bounded native path navigation exhausted".into())
}
fn gather(
    h: &mut Host,
    e: &mut Explorer,
    g: Ground,
    id: ItemId,
    amount: u16,
    samples: &mut Vec<(i8, Point)>,
) -> Result<(), String> {
    for _ in 0..40 {
        if count(e, id) >= amount {
            return Ok(());
        }
        h.wait(e, |e| {
            [-11, 11].into_iter().all(|dx| {
                [-8, 8].into_iter().all(|dy| {
                    e.visible_cell(e.position().0 + dx, e.position().1 + dy)
                        .is_some()
                })
            })
        })?;
        let (steps, dir) = path(e, None, Some(g))?;
        walk(h, e, &steps, samples)?;
        mine(h, e, dir.unwrap(), g, samples)?;
        let before = count(e, id);
        h.wait(e, |e| {
            count(e, id) > before
                || e.survival_state().unwrap().drops.iter().all(|d| {
                    d.depth != e.depth() || !matches!(d.loot,Loot::Stack(st) if st.item==id)
                })
        })?;
    }
    Err("naturally earned resource target was not reached".into())
}

pub(super) fn run(
    h: &mut Host,
    e: &mut Explorer,
    root: &Path,
    creating: bool,
) -> Result<(), String> {
    let evidence = root.join("survival-expect.json");
    let directory = root.join("infdev-survival-v4/Window42");
    let mut samples = Vec::new();
    if creating {
        if e.survival_state().unwrap().player.inventory.used_slots() != 0 {
            return Err("Survival did not start with an empty inventory".into());
        }
        gather(h, e, Ground::Tree, ItemId::Wood, 20, &mut samples)?;
        println!(
            "PASS native Survival: naturally earned {} wood",
            count(e, ItemId::Wood)
        );
        craft(h, e, false, "Workbench")?;
        println!("Survival window stage: earned workbench crafted");
        // Use the actual just-cleared tree site, leaving the player beside it.
        let d = match e.survival_state().unwrap().player.direction {
            crate::world::Direction::Down => (0, 1),
            crate::world::Direction::Up => (0, -1),
            crate::world::Direction::Left => (-1, 0),
            crate::world::Direction::Right => (1, 0),
        };
        let bench = (e.position().0 + d.0, e.position().1 + d.1);
        let stand = e.position();
        stack(h, e, ItemId::Workbench)?;
        h.wait(e, |e| {
            let p = &e.survival_state().unwrap().player;
            p.attack_time == 0 && p.stamina > 0
        })?;
        pulse(
            h,
            e,
            Input {
                attack: true,
                ..Default::default()
            },
        )?;
        if !e.survival_state().unwrap().station_at(0, bench.0, bench.1) {
            return Err("earned workbench placement failed".into());
        }
        craft(h, e, true, "Wood Axe")?;
        println!("Survival window stage: wood axe crafted");
        craft(h, e, true, "Wood Pickaxe")?;
        println!("Survival window stage: wood pickaxe crafted");
        gather(h, e, Ground::Rock, ItemId::Stone, 5, &mut samples)?;
        println!("Survival window stage: stone naturally mined");
        gather(h, e, Ground::Tree, ItemId::Wood, 5, &mut samples)?;
        println!("Survival window stage: additional wood earned");
        navigate(h, e, stand, &mut samples)?;
        face(h, e, d)?;
        craft(h, e, true, "Rock Pickaxe")?;
        println!("PASS native Survival: real workbench, wood tools, stone mining and Rock Pickaxe");
        // Same natural v3/v4 Seed 42 stair; normal pixel/collision movement only.
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
            return Err("natural Survival stairs did not descend".into());
        }
        gather(h, e, Ground::IronOre, ItemId::IronOre, 1, &mut samples)?;
        h.screen.save_png(&root.join("infdev-survival.png"))?;
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
            return Err("Survival stairs did not return to surface".into());
        }
        stack(h, e, ItemId::IronOre)?;
        let before = count(e, ItemId::IronOre);
        pulse(
            h,
            e,
            Input {
                drop_one: true,
                ..Default::default()
            },
        )?;
        h.wait(e, |e| count(e, ItemId::IronOre) == before)?;
        println!("PASS native Survival: down mine / surface return / drop and repickup");
    } else {
        let expected: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&evidence).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let s = e.survival_state().ok_or("reopened world is not Survival")?;
        if serde_json::to_value(&s.player.inventory).map_err(|e| e.to_string())?
            != expected["inventory"]
            || serde_json::json!(e.position()) != expected["position"]
            || serde_json::json!(e.depth()) != expected["depth"]
            || serde_json::json!(s.player.health) != expected["health"]
        {
            return Err("native Survival player/inventory did not restore".into());
        }
        if !s
            .player
            .inventory
            .tools()
            .iter()
            .any(|t| t.kind == ToolKind::Pickaxe && t.durability < t.max_durability)
        {
            return Err("earned tool durability missing".into());
        }
    }
    e.request_save_quit();
    h.wait(e, Explorer::has_finished)?;
    e.finish()?;
    let mut check = paged::World::open(&directory, 1)?;
    if creating {
        let s = check.survival().unwrap();
        let mut value = serde_json::json!({"inventory":s.player.inventory,"health":s.player.health,"stamina":s.player.stamina,"position":check.position(),"depth":check.depth(),"stations":s.stations.len()});
        let mut terrain = Vec::new();
        samples.sort();
        samples.dedup();
        for (depth, (x, y)) in samples {
            let c = check.cell_at(depth, x, y)?;
            terrain.push(serde_json::json!({"depth":depth,"x":x,"y":y,"ground":c.ground as u8,"data":c.data}));
        }
        value["terrain"] = terrain.into();
        fs_write_evidence(&evidence, value)?;
    } else {
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&evidence).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        for sample in value["terrain"]
            .as_array()
            .ok_or("no real mined terrain evidence")?
        {
            let c = check.cell_at(
                sample["depth"].as_i64().unwrap() as i8,
                sample["x"].as_i64().unwrap(),
                sample["y"].as_i64().unwrap(),
            )?;
            if serde_json::json!(c.ground as u8) != sample["ground"]
                || serde_json::json!(c.data) != sample["data"]
            {
                return Err("a naturally mined resource regenerated after process restart".into());
            }
        }
        if serde_json::json!(check.survival().unwrap().stations.len()) != value["stations"] {
            return Err("real workbench did not restore".into());
        }
    }
    if creating
        && !matches!(
            check.survival().unwrap().player.active_item,
            Some(ActiveItem::Stack(ItemId::IronOre))
        )
    {
        return Err("active stack was not preserved".into());
    }
    println!(
        "PASS real Survival window {}: empty bag -> natural wood -> workbench -> tools -> rock -> underground ore -> return -> Save/Quit/restart; no fixture injection",
        if creating { "create" } else { "reopen" }
    );
    Ok(())
}
