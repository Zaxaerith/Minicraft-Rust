//! Explicit diagnostic: real window/presenter + the same menu, worker and input
//! adapters, driven by scripted Input rather than claiming a human playthrough.
use super::{
    infdev_ui::{self, Action, Browser},
    *,
};
use crate::experimental::infdev::{Cell, Ground, explorer::Explorer, paged, stairs};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct Host {
    window: Window,
    presenter: Presenter,
    assets: Assets,
    screen: Screen,
    deadline: Instant,
}

impl Host {
    fn present(&mut self) -> Result<(), String> {
        if !self.window.is_open() {
            return Err("Window smoke interrupted by window close".into());
        }
        if Instant::now() > self.deadline {
            return Err("Window smoke deadline exceeded".into());
        }
        self.presenter
            .present(&mut self.window, self.screen.pixels())
    }
    fn frame(&mut self, explorer: &mut Explorer) -> Result<(), String> {
        for _ in 0..6 {
            explorer.tick(&Input::default());
        }
        explorer.render(&mut self.screen, &self.assets);
        self.present()
    }
    fn wait(
        &mut self,
        explorer: &mut Explorer,
        check: impl Fn(&Explorer) -> bool,
    ) -> Result<(), String> {
        loop {
            self.frame(explorer)?;
            if check(explorer) {
                return Ok(());
            }
            if explorer.status().contains("FAILED") {
                return Err(explorer.status().into());
            }
        }
    }
    fn walk(&mut self, explorer: &mut Explorer, goal: (i64, i64)) -> Result<(), String> {
        while explorer.position() != goal {
            self.wait(explorer, Explorer::is_ready)?;
            let (x, y) = explorer.position();
            let input = Input {
                left: x > goal.0,
                right: x < goal.0,
                up: x == goal.0 && y > goal.1,
                down: x == goal.0 && y < goal.1,
                ..Default::default()
            };
            let next = (
                x + i64::from(input.right) - i64::from(input.left),
                y + i64::from(input.down) - i64::from(input.up),
            );
            for _ in 0..6 {
                explorer.tick(&Input::default());
            }
            explorer.tick(&input);
            self.wait(explorer, |e| e.position() == next && e.is_ready())?;
            if explorer.visible_chunk_count() > 4 {
                return Err("GUI snapshot exceeded bounded cache".into());
            }
        }
        Ok(())
    }

    fn stairs(&mut self, explorer: &mut Explorer, depth: i8) -> Result<(), String> {
        for _ in 0..6 {
            explorer.tick(&Input::default());
        }
        explorer.tick(&Input {
            pickup: true,
            ..Default::default()
        });
        self.wait(explorer, |e| e.depth() == depth && e.is_ready())
    }
}

fn natural_down(depth: i8, position: (i64, i64)) -> Result<(i64, i64), String> {
    let center = paged::Key::at_depth(depth, position.0, position.1);
    for radius in 0_i64..=8 {
        let mut best = None;
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                if dx.abs().max(dy.abs()) != radius {
                    continue;
                }
                let (cx, cy) = (center.x + dx, center.y + dy);
                for (index, cell) in stairs::generate(42, depth, cx, cy)?.iter().enumerate() {
                    if cell.ground == Ground::StairsDown {
                        let p = (cx * 64 + (index % 64) as i64, cy * 64 + (index / 64) as i64);
                        let distance = p.0.abs_diff(position.0) + p.1.abs_diff(position.1);
                        if best.is_none_or(|(d, _)| distance < d) {
                            best = Some((distance, p));
                        }
                    }
                }
            }
        }
        if let Some((_, p)) = best {
            return Ok(p);
        }
    }
    Err("No naturally generated staircase in bounded diagnostic discovery".into())
}

fn edit_underground(host: &mut Host, explorer: &mut Explorer) -> Result<serde_json::Value, String> {
    host.wait(explorer, |e| {
        [-7, 0, 7].into_iter().all(|dx| {
            [-5, 0, 5].into_iter().all(|dy| {
                e.visible_cell(e.position().0 + dx, e.position().1 + dy)
                    .is_some()
            })
        })
    })?;
    let (px, py) = explorer.position();
    let target = (-5..=5)
        .flat_map(|dy| (-7..=7).map(move |dx| (px + dx, py + dy)))
        .find_map(|(x, y)| {
            explorer
                .visible_cell(x, y)
                .filter(|c| {
                    matches!(
                        c,
                        Cell {
                            ground: Ground::Dirt | Ground::Rock,
                            data: 0
                        }
                    )
                })
                .map(|c| (x, y, c))
        })
        .ok_or("No natural editable underground tile in loaded viewport")?;
    let (x, y, before) = target;
    host.walk(explorer, (x, y - 2))?;
    host.walk(explorer, (x, y - 1))?;
    explorer.tick(&Input {
        attack: true,
        ..Default::default()
    });
    let after = Cell {
        ground: if before.ground == Ground::Dirt {
            Ground::Rock
        } else {
            Ground::Dirt
        },
        data: 0,
    };
    host.wait(explorer, |e| e.visible_cell(x, y) == Some(after))?;
    Ok(
        serde_json::json!({"depth":explorer.depth(),"edited":[x,y],"ground":after.ground as u8,"data":0}),
    )
}

fn explore_layers(host: &mut Host, explorer: &mut Explorer, root: &Path) -> Result<(), String> {
    let evidence = root.join("gui-expect.json");
    let mut surface: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    surface["depth"] = 0.into();
    let mut edits = vec![surface];
    let mut links = Vec::new();
    for depth in (-4..=-1).rev() {
        let stair = natural_down(explorer.depth(), explorer.position())?;
        println!(
            "Natural v3 stair depth={} position={stair:?}",
            explorer.depth()
        );
        host.walk(explorer, stair)?;
        links.push(stair);
        host.stairs(explorer, depth)?;
        // Explicitly cross an underground chunk boundary through normal input.
        if depth == -1 {
            host.walk(explorer, (stair.0 - 70, stair.1 - 65))?;
        }
        edits.push(edit_underground(host, explorer)?);
        if depth == -3 {
            host.screen.save_png(&root.join("infdev-underground.png"))?;
        }
    }
    for (index, stair) in links.iter().enumerate().rev() {
        host.walk(explorer, *stair)?;
        host.stairs(explorer, -(index as i8))?;
    }
    if explorer.depth() != 0 {
        return Err("Failed to return through complete staircase chain".into());
    }
    // Save underground, after already proving the complete round trip.
    host.stairs(explorer, -1)?;
    let edited = &edits[1];
    let x = edited["edited"][0].as_i64().ok_or("edit x missing")?;
    let y = edited["edited"][1].as_i64().ok_or("edit y missing")?;
    host.walk(explorer, (x, y - 1))?;
    let mut final_state = edited.clone();
    final_state["position"] = serde_json::json!(explorer.position());
    final_state["edits"] = serde_json::json!(edits);
    final_state["stairs"] = serde_json::json!(links);
    fs_write_evidence(&evidence, final_state)
}

pub(super) fn run(mode: &str, root: PathBuf) -> Result<(), String> {
    if !matches!(mode, "create" | "reopen") {
        return Err("window smoke mode must be create or reopen".into());
    }
    let assets = Assets::load(&[])?;
    let mut window = Window::new(
        "Minicraft Infdev - scripted window smoke",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X4,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| e.to_string())?;
    window.set_target_fps(120);
    let presenter = Presenter::new(&window, false)?;
    let mut host = Host {
        window,
        presenter,
        assets,
        screen: Screen::new(),
        deadline: Instant::now() + Duration::from_secs(90),
    };
    let locale = Localization::load("en-us", &[]);
    let mut title = infdev_ui::title_menu(&locale, 7);
    render_title_menu(
        &mut host.screen,
        &host.assets,
        &locale,
        &title,
        0,
        "Infdev exploration",
    );
    host.present()?;
    let (transition, _) = tick_title_menu_event(
        &mut title,
        &mut 0,
        &Input {
            select: true,
            ..Default::default()
        },
        0,
        &locale,
    );
    if !matches!(transition, Transition::Infdev) {
        return Err("Live title did not select Infdev".into());
    }
    let mut browser = Browser::new(root.clone());
    browser.render(&mut host.screen, &host.assets);
    host.present()?;
    let action = if mode == "create" {
        browser.tick(&Input {
            select: true,
            ..Default::default()
        });
        browser.tick(&Input {
            text: "Window42".chars().collect(),
            ..Default::default()
        });
        browser.tick(&Input {
            cursor_down_pressed: true,
            ..Default::default()
        });
        browser.tick(&Input {
            cursor_down_pressed: true,
            ..Default::default()
        });
        browser.render(&mut host.screen, &host.assets);
        host.present()?;
        browser.tick(&Input {
            select: true,
            ..Default::default()
        })
    } else {
        // New / Refresh / Window42. The driver uses a dedicated empty root.
        browser.tick(&Input {
            cursor_down_pressed: true,
            ..Default::default()
        });
        browser.tick(&Input {
            cursor_down_pressed: true,
            ..Default::default()
        });
        browser.tick(&Input {
            select: true,
            ..Default::default()
        })
    };
    let Action::Start(mut explorer) = action else {
        return Err("Experimental browser did not start a session".into());
    };
    host.wait(&mut explorer, |e| {
        e.is_ready()
            && [-11, 11].into_iter().all(|dx| {
                [-8, 8].into_iter().all(|dy| {
                    e.visible_cell(e.position().0 + dx, e.position().1 + dy)
                        .is_some()
                })
            })
    })?;
    let evidence = root.join("gui-expect.json");
    if mode == "create" {
        for goal in [(66, 0), (-66, 0), (-66, -66), (1, 1), (-1, -1)] {
            host.walk(&mut explorer, goal)?;
        }
        // Select genuinely generated editable ground; do not pre-place terrain
        // or replace the world with a fixture to manufacture a successful edit.
        let (px, py) = explorer.position();
        let mut target = None;
        for dy in -5..=5 {
            for dx in -7..=7 {
                let (x, y) = (px + dx, py + dy);
                if let Some(cell) = explorer.visible_cell(x, y)
                    && matches!(
                        cell,
                        Cell {
                            ground: Ground::Grass | Ground::Flower,
                            data: 0
                        }
                    )
                {
                    target = Some((x, y, cell));
                    break;
                }
            }
            if target.is_some() {
                break;
            }
        }
        let (x, y, before) = target.ok_or("No naturally editable grass/daisy in final viewport")?;
        host.walk(&mut explorer, (x, y - 2))?;
        host.walk(&mut explorer, (x, y - 1))?; // face down, toward actual natural tile
        explorer.tick(&Input {
            attack: true,
            ..Default::default()
        });
        let after = Cell {
            ground: if before.ground == Ground::Grass {
                Ground::Flower
            } else {
                Ground::Grass
            },
            data: 0,
        };
        host.wait(&mut explorer, |e| e.visible_cell(x, y) == Some(after))?;
        host.screen.save_png(&root.join("infdev-window.png"))?;
        fs_write_evidence(
            &evidence,
            serde_json::json!({"position":explorer.position(),"edited":[x,y],"ground":after.ground as u8,"data":after.data}),
        )?;
        explore_layers(&mut host, &mut explorer, &root)?;
    } else {
        let expected: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&evidence).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let array = |key: &str| -> Result<(i64, i64), String> {
            Ok((
                expected[key][0].as_i64().ok_or("missing coordinate")?,
                expected[key][1].as_i64().ok_or("missing coordinate")?,
            ))
        };
        if explorer.position() != array("position")? {
            return Err("Reopened GUI position differs".into());
        }
        if Some(i64::from(explorer.depth())) != expected["depth"].as_i64() {
            return Err("Reopened GUI depth differs".into());
        }
        let (x, y) = array("edited")?;
        host.wait(&mut explorer, |e| e.visible_cell(x, y).is_some())?;
        let actual = explorer
            .visible_cell(x, y)
            .ok_or("edited tile not visible")?;
        if Some(actual.ground as u64) != expected["ground"].as_u64()
            || Some(u64::from(actual.data)) != expected["data"].as_u64()
        {
            return Err("Reopened GUI terrain edit differs".into());
        }
    }
    explorer.tick(&Input {
        exit: true,
        ..Default::default()
    });
    host.wait(&mut explorer, Explorer::has_finished)?;
    explorer.finish()?;
    // The exact GUI-started world is now unlocked and independently openable.
    let mut check = paged::World::open(&root.join("infdev-v3/Window42"), 1)?;
    let evidence: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    for edit in evidence["edits"]
        .as_array()
        .ok_or("missing multilayer edit evidence")?
    {
        let depth = edit["depth"].as_i64().ok_or("missing edit depth")? as i8;
        let x = edit["edited"][0].as_i64().ok_or("missing edit x")?;
        let y = edit["edited"][1].as_i64().ok_or("missing edit y")?;
        let cell = check.cell_at(depth, x, y)?;
        if Some(cell.ground as u64) != edit["ground"].as_u64()
            || Some(u64::from(cell.data)) != edit["data"].as_u64()
        {
            return Err("A saved layer edit differs".into());
        }
    }
    println!(
        "PASS real window {mode}: {}; five persisted layer edits checked; depth={} position={:?}",
        if mode == "create" {
            "legal stairs down to -4 and back; scripted negative/positive exploration"
        } else {
            "restored underground position and terrain; Save & Quit"
        },
        check.depth(),
        check.position()
    );
    Ok(())
}

fn fs_write_evidence(path: &Path, value: serde_json::Value) -> Result<(), String> {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
