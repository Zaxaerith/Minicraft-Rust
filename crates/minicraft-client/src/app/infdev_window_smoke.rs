//! Explicit diagnostic: real window/presenter + the same menu, worker and input
//! adapters, driven by scripted Input rather than claiming a human playthrough.
use super::{
    infdev_ui::{self, Action, Browser},
    *,
};
use crate::experimental::infdev::{Cell, Ground, explorer::Explorer};
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
        e.is_ready() && e.visible_chunk_count() == 4
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
    let check =
        crate::experimental::infdev::paged::World::open(&root.join("infdev-v2/Window42"), 1)?;
    println!(
        "PASS real window {mode}: title/browser, natural surface, scripted Input, negative/positive chunk traversal, edit, Save & Quit; position={:?}",
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
