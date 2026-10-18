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
            "worker timeout: {}",
            explorer.status()
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
            index as i32 * 16,
            0,
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
