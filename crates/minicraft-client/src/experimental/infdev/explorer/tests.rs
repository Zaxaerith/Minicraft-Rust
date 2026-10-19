use super::*;
use crate::experimental::infdev::{
    Ground,
    paged::{self, World},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

struct Owned(PathBuf);
impl Owned {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "minicraft-infdev-gui-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn directory(&self) -> PathBuf {
        self.0.join(paged::NAMESPACE).join("Explore")
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn wait(explorer: &mut Explorer, condition: impl Fn(&Explorer) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        explorer.tick(&Input::default());
        if condition(explorer) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "worker timeout: {} depth={} position={:?} ready={} snapshots={:?} needed={:?}",
            explorer.status(),
            explorer.depth(),
            explorer.position(),
            explorer.is_ready(),
            explorer.snapshot.chunks.keys().collect::<Vec<_>>(),
            worker::needed_at(explorer.depth(), explorer.position())
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn move_to(explorer: &mut Explorer, direction: Input, position: (i64, i64)) {
    // Release the previous movement cadence without held input.
    for _ in 0..MOVE_TICKS {
        explorer.tick(&Input::default());
    }
    explorer.tick(&direction);
    wait(explorer, |session| {
        session.position() == position && session.is_ready()
    });
    assert!(explorer.visible_chunk_count() <= 4);
}

#[test]
fn gui_worker_crosses_negative_boundaries_edits_and_reopens_actual_store() {
    let owned = Owned::new();
    let mut source = World::create(&owned.0, "Explore", 42, 16).unwrap();
    source
        .set_cell(
            -2,
            0,
            Cell {
                ground: Ground::Grass,
                data: 0,
            },
        )
        .unwrap();
    source.checkpoint().unwrap();
    drop(source);

    let mut explorer = Explorer::open(owned.directory()).unwrap();
    wait(&mut explorer, |e| {
        e.is_ready() && e.visible_chunk_count() == 4
    });
    for (input, position) in [
        (
            Input {
                left: true,
                ..Input::default()
            },
            (-1, 0),
        ),
        (
            Input {
                up: true,
                ..Input::default()
            },
            (-1, -1),
        ),
        (
            Input {
                right: true,
                ..Input::default()
            },
            (0, -1),
        ),
        (
            Input {
                down: true,
                ..Input::default()
            },
            (0, 0),
        ),
        (
            Input {
                left: true,
                ..Input::default()
            },
            (-1, 0),
        ),
    ] {
        move_to(&mut explorer, input, position);
    }
    explorer.tick(&Input {
        attack: true,
        ..Input::default()
    });
    wait(&mut explorer, |e| {
        e.visible_cell(-2, 0)
            .is_some_and(|c| c.ground == Ground::Flower)
    });
    explorer.request_save_quit();
    explorer.tick(&Input::default());
    // Window close racing Escape must join the same successful checkpoint.
    explorer.finish().unwrap();
    assert!(explorer.has_finished());
    let mut reopened = World::open(&owned.directory(), 1).unwrap();
    assert_eq!(reopened.position(), (-1, 0));
    assert_eq!(
        reopened.cell(-2, 0).unwrap(),
        Cell {
            ground: Ground::Flower,
            data: 0
        }
    );
    drop(reopened);
    let mut again = Explorer::open(owned.directory()).unwrap();
    wait(&mut again, |e| e.is_ready());
    assert_eq!(again.position(), (-1, 0));
    again.finish().unwrap();
}

#[test]
fn failed_gui_save_keeps_worker_lock_and_dirty_terrain_for_retry() {
    let owned = Owned::new();
    let mut world = World::create(&owned.0, "Explore", 42, 16).unwrap();
    world
        .set_cell(
            0,
            1,
            Cell {
                ground: Ground::Grass,
                data: 0,
            },
        )
        .unwrap();
    world.checkpoint().unwrap();
    drop(world);
    let mut explorer = Explorer::open(owned.directory()).unwrap();
    wait(&mut explorer, |e| e.is_ready());
    explorer.tick(&Input {
        attack: true,
        ..Input::default()
    });
    wait(&mut explorer, |e| {
        e.visible_cell(0, 1)
            .is_some_and(|c| c.ground == Ground::Flower)
    });
    let checkpoint = owned.directory().join("checkpoint.json");
    let original = fs::read(&checkpoint).unwrap();
    fs::write(&checkpoint, b"{truncated").unwrap();
    explorer.request_save_quit();
    wait(&mut explorer, |e| e.snapshot.save_failed);
    assert!(explorer.finish().is_err());
    assert!(!explorer.has_finished());
    assert!(
        World::open(&owned.directory(), 1).is_err(),
        "failed save must retain session lock"
    );
    assert_eq!(explorer.visible_cell(0, 1).unwrap().ground, Ground::Flower);
    fs::write(&checkpoint, original).unwrap();
    explorer.finish().unwrap();
    let mut reopened = World::open(&owned.directory(), 1).unwrap();
    assert_eq!(reopened.cell(0, 1).unwrap().ground, Ground::Flower);
}

#[test]
fn failed_async_open_is_visible_and_can_exit_without_creating_a_world() {
    let owned = Owned::new();
    let mut explorer = Explorer::open(owned.directory()).unwrap();
    wait(&mut explorer, |e| e.status().starts_with("OPEN FAILED:"));
    assert!(!explorer.is_ready());
    explorer.request_save_quit();
    wait(&mut explorer, Explorer::has_finished);
    assert!(!owned.directory().exists());
}

#[test]
fn bounded_view_and_render_use_real_resources_across_i64_chunk_edges() {
    for position in [
        (0, 0),
        (-1, -1),
        (63, 63),
        (i64::MIN, i64::MAX),
        (i64::MAX, i64::MIN),
    ] {
        let keys = worker::needed(position);
        assert!(keys.len() <= 4);
        assert!(keys.contains(&Key::at(position.0, position.1)));
    }
    let assets = Assets::load(&[]).unwrap();
    let mut screen = Screen::new();
    let grounds = [
        Ground::Grass,
        Ground::Water,
        Ground::Sand,
        Ground::Rock,
        Ground::Tree,
        Ground::Flower,
        Ground::Cactus,
        Ground::StairsDown,
        Ground::Dirt,
        Ground::IronOre,
        Ground::GoldOre,
        Ground::GemOre,
        Ground::Lapis,
        Ground::Lava,
        Ground::ObsidianWall,
        Ground::Obsidian,
        Ground::RawObsidian,
        Ground::OrnateObsidian,
        Ground::StairsUp,
        Ground::ObsidianDoor,
    ];
    let expected = [
        crate::level::tile::Tile::Grass,
        crate::level::tile::Tile::Water,
        crate::level::tile::Tile::Sand,
        crate::level::tile::Tile::Rock,
        crate::level::tile::Tile::Tree,
        crate::level::tile::Tile::Flower230,
        crate::level::tile::Tile::Cactus,
        crate::level::tile::Tile::StairsDown,
        crate::level::tile::Tile::Dirt,
        crate::level::tile::Tile::IronOre,
        crate::level::tile::Tile::GoldOre,
        crate::level::tile::Tile::GemOre,
        crate::level::tile::Tile::LapisOre,
        crate::level::tile::Tile::Lava,
        crate::level::tile::Tile::ObsidianWall,
        crate::level::tile::Tile::ObsidianFloor,
        crate::level::tile::Tile::RawObsidian,
        crate::level::tile::Tile::OrnateObsidian,
        crate::level::tile::Tile::StairsUp,
        crate::level::tile::Tile::ObsidianDoor,
    ];
    for (index, (ground, tile)) in grounds.into_iter().zip(expected).enumerate() {
        let cell = Cell {
            ground,
            data: if ground == Ground::Flower { 11 } else { 0 },
        };
        assert_eq!(render::tile_for(cell), tile);
        assets.render_tile(
            &mut screen,
            tile,
            cell.data,
            (index % 16) as i32 * 16,
            (index / 16) as i32 * 16,
            0,
            0,
            [true; 8],
        );
    }
    assert!(screen.pixels()[..288 * 16].iter().any(|pixel| *pixel != 0));
    let owned = Owned::new();
    let mut explorer = Explorer::create(owned.0.clone(), "Explore".into(), 42).unwrap();
    wait(&mut explorer, |e| {
        e.is_ready() && e.visible_chunk_count() == 4
    });
    explorer.render(&mut screen, &assets);
    // Actual background, character and HUD are drawn by the GUI adapter using
    // a natural world snapshot, with no filesystem or terrain calls in render.
    assert!(
        screen
            .pixels()
            .as_chunks::<288>()
            .0
            .iter()
            .skip(4)
            .take(6)
            .flatten()
            .any(|pixel| *pixel != 0xFF10202A && *pixel != 0xFF101820)
    );
    explorer.finish().unwrap();
}

fn natural_negative_stair() -> (i64, i64) {
    static STAIR: std::sync::OnceLock<(i64, i64)> = std::sync::OnceLock::new();
    *STAIR.get_or_init(|| {
        // This is the naturally discovered Rock-biome Oracle chunk, shifted by
        // the documented Java i32 noise period. The coordinate stair stream is
        // sampled normally; no stair tile or counterpart is fabricated.
        for period in 1..=8_i64 {
            let cx = 945 - period * (1_i64 << 26);
            let cy = 877;
            let cells = crate::experimental::infdev::terrain::generate(42, cx, cy);
            if let Some(index) = cells.iter().position(|c| c.ground == Ground::StairsDown) {
                return (cx * 64 + (index % 64) as i64, cy * 64 + (index / 64) as i64);
            }
        }
        panic!("no natural stair in fixed deterministic discovery samples")
    })
}

fn walk_to(explorer: &mut Explorer, target: (i64, i64)) {
    while explorer.position() != target {
        let (x, y) = explorer.position();
        let (dx, dy) = if x != target.0 {
            ((target.0 - x).signum(), 0)
        } else {
            (0, (target.1 - y).signum())
        };
        move_to(
            explorer,
            Input {
                left: dx < 0,
                right: dx > 0,
                up: dy < 0,
                down: dy > 0,
                ..Input::default()
            },
            (x + dx, y + dy),
        );
    }
}

fn fully_loaded(explorer: &Explorer) -> bool {
    explorer.is_ready()
        && explorer.visible_chunk_count()
            == worker::needed_at(explorer.depth(), explorer.position()).len()
}

#[test]
fn natural_stairs_gui_worker_changes_depth_edits_negative_chunk_and_reopens() {
    let owned = Owned::new();
    let stair = natural_negative_stair();
    let directory = owned.0.join(paged::LAYERED_NAMESPACE).join("Explore");
    let mut source = World::create_layers(&owned.0, "Explore", 42, 16).unwrap();
    assert_eq!(
        source.cell(stair.0, stair.1).unwrap().ground,
        Ground::StairsDown
    );
    let adjacent = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dy)| (stair.0 + dx, stair.1 + dy))
        .find(|&(x, y)| {
            !matches!(
                source.cell(x, y).unwrap().ground,
                Ground::StairsDown | Ground::StairsUp
            )
        })
        .unwrap();
    source.visit(adjacent.0, adjacent.1).unwrap();
    source.checkpoint().unwrap();
    drop(source);
    let mut explorer = Explorer::open(directory.clone()).unwrap();
    wait(&mut explorer, fully_loaded);
    assert!(explorer.is_layered());
    explorer.tick(&Input {
        pickup: true,
        ..Input::default()
    });
    wait(&mut explorer, |e| e.status().starts_with("ACTION FAILED:"));
    assert_eq!(explorer.depth(), 0);
    assert_eq!(explorer.position(), adjacent);
    assert!(explorer.snapshot.chunks.keys().all(|key| key.depth == 0));
    walk_to(&mut explorer, stair);
    explorer.tick(&Input {
        pickup: true,
        ..Input::default()
    });
    wait(&mut explorer, |e| e.depth() == -1 && fully_loaded(e));
    assert_eq!(explorer.position(), stair);
    assert_eq!(explorer.camera_offset, (0, 0));
    assert!(explorer.snapshot.chunks.keys().all(|key| key.depth == -1));
    assert_eq!(
        explorer.visible_cell(stair.0, stair.1).unwrap().ground,
        Ground::StairsUp
    );

    // Walk across a real negative chunk edge; the flight probe can cross rock.
    let boundary = (stair.0.div_euclid(64) * 64 - 1, stair.1);
    walk_to(&mut explorer, boundary);
    assert_ne!(Key::at(stair.0, stair.1), Key::at(boundary.0, boundary.1));
    // Find a naturally generated editable neighbor. Do not introduce resources
    // or alter any stairs merely to satisfy the workflow test.
    let mut edit = None;
    for offset in 0..64 {
        let position = (boundary.0, boundary.1 + offset);
        walk_to(&mut explorer, position);
        for direction in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let target = (position.0 + direction.0, position.1 + direction.1);
            if let Some(cell) = explorer.visible_cell(target.0, target.1)
                && cell.data == 0
                && matches!(cell.ground, Ground::Dirt | Ground::Rock)
            {
                edit = Some((target, direction, cell));
                break;
            }
        }
        if edit.is_some() {
            break;
        }
    }
    let (target, direction, old) = edit.expect("natural cave dirt or rock near boundary");
    explorer.direction = (direction.0 as i32, direction.1 as i32);
    let new = Cell {
        ground: if old.ground == Ground::Dirt {
            Ground::Rock
        } else {
            Ground::Dirt
        },
        data: 0,
    };
    explorer.tick(&Input {
        attack: true,
        ..Input::default()
    });
    wait(&mut explorer, |e| {
        e.visible_cell(target.0, target.1) == Some(new)
    });
    let saved_position = explorer.position();
    explorer.finish().unwrap();

    let mut again = Explorer::open(directory.clone()).unwrap();
    wait(&mut again, |e| e.is_ready());
    assert_eq!(again.depth(), -1);
    assert_eq!(again.position(), saved_position);
    wait(&mut again, |e| {
        e.visible_cell(target.0, target.1) == Some(new)
    });
    walk_to(&mut again, stair);
    again.tick(&Input {
        pickup: true,
        ..Input::default()
    });
    wait(&mut again, |e| e.depth() == 0 && e.is_ready());
    assert_eq!(again.position(), stair);
    assert_eq!(
        again.visible_cell(stair.0, stair.1).unwrap().ground,
        Ground::StairsDown
    );
    again.finish().unwrap();
    let mut disk = World::open(&directory, 1).unwrap();
    assert_eq!(disk.depth(), 0);
    assert_eq!(disk.position(), stair);
    assert_eq!(disk.cell_at(-1, target.0, target.1).unwrap(), new);
    assert_eq!(
        disk.cell_at(0, stair.0, stair.1).unwrap().ground,
        Ground::StairsDown
    );
}

#[test]
fn invalid_saved_stair_counterpart_cannot_change_gui_layer_or_regenerate_it() {
    let owned = Owned::new();
    let stair = natural_negative_stair();
    let directory = owned.0.join(paged::LAYERED_NAMESPACE).join("Explore");
    let mut source = World::create_layers(&owned.0, "Explore", 42, 16).unwrap();
    source.visit(stair.0, stair.1).unwrap();
    assert_eq!(
        source.cell_at(-1, stair.0, stair.1).unwrap().ground,
        Ground::StairsUp
    );
    let blocked = Cell {
        ground: Ground::Dirt,
        data: 0,
    };
    source.set_cell_at(-1, stair.0, stair.1, blocked).unwrap();
    source.checkpoint().unwrap();
    drop(source);
    let mut explorer = Explorer::open(directory.clone()).unwrap();
    wait(&mut explorer, fully_loaded);
    explorer.tick(&Input {
        pickup: true,
        ..Input::default()
    });
    wait(&mut explorer, |e| e.status().starts_with("ACTION FAILED:"));
    assert!(explorer.status().contains("counterpart differs"));
    assert_eq!(explorer.depth(), 0);
    assert_eq!(explorer.position(), stair);
    assert!(explorer.is_ready());
    assert!(explorer.snapshot.chunks.keys().all(|key| key.depth == 0));
    assert_eq!(
        explorer.visible_cell(stair.0, stair.1).unwrap().ground,
        Ground::StairsDown
    );
    explorer.finish().unwrap();
    let mut disk = World::open(&directory, 1).unwrap();
    assert_eq!(disk.cell_at(-1, stair.0, stair.1).unwrap(), blocked);
}
