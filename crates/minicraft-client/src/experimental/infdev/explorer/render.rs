use super::Explorer;
use crate::{
    assets::Assets,
    experimental::infdev::{Cell, Ground},
    gfx::{HEIGHT, Screen, WIDTH},
    level::tile::{self, Tile},
};

pub(super) fn tile_for(cell: Cell) -> Tile {
    match cell.ground {
        Ground::Grass => Tile::Grass,
        Ground::Water => Tile::Water,
        Ground::Sand => Tile::Sand,
        Ground::Rock => Tile::Rock,
        Ground::Tree => Tile::Tree,
        // Infdev's first twelve FlowerVariant ordinals equal the registered
        // dev2 Flower230 order, not the old two-shape Flower encoding.
        Ground::Flower => Tile::Flower230,
        Ground::Cactus => Tile::Cactus,
        Ground::StairsDown => Tile::StairsDown,
        Ground::Dirt => Tile::Dirt,
        Ground::IronOre => Tile::IronOre,
        Ground::GoldOre => Tile::GoldOre,
        Ground::GemOre => Tile::GemOre,
        Ground::Lapis => Tile::LapisOre,
        Ground::Lava => Tile::Lava,
        Ground::ObsidianWall => Tile::ObsidianWall,
        Ground::Obsidian => Tile::ObsidianFloor,
        Ground::RawObsidian => Tile::RawObsidian,
        Ground::OrnateObsidian => Tile::OrnateObsidian,
        Ground::StairsUp => Tile::StairsUp,
        Ground::ObsidianDoor => Tile::ObsidianDoor,
    }
}

fn mask(explorer: &Explorer, x: i64, y: i64, tile: Tile) -> [bool; 8] {
    [
        (0, -1),
        (0, 1),
        (-1, 0),
        (1, 0),
        (-1, -1),
        (-1, 1),
        (1, -1),
        (1, 1),
    ]
    .map(|(dx, dy)| {
        x.checked_add(dx)
            .zip(y.checked_add(dy))
            .and_then(|(x, y)| explorer.snapshot.cell(x, y))
            .is_some_and(|neighbor| tile::connects(tile, tile_for(neighbor)))
    })
}

pub(super) fn draw(explorer: &Explorer, screen: &mut Screen, assets: &Assets) {
    screen.clear(0xFF10202A);
    let (center_x, center_y) = explorer.position();
    for dy in -7_i64..=7 {
        for dx in -10_i64..=10 {
            let Some((x, y)) = center_x.checked_add(dx).zip(center_y.checked_add(dy)) else {
                continue;
            };
            let draw_x = WIDTH as i32 / 2 - 8 + dx as i32 * 16 + explorer.camera_offset.0;
            let draw_y = HEIGHT as i32 / 2 - 8 + dy as i32 * 16 + explorer.camera_offset.1;
            let Some(cell) = explorer.snapshot.cell(x, y) else {
                screen.frame(draw_x, draw_y, 16, 16, 0xFF1C3542);
                continue;
            };
            let tile = tile_for(cell);
            for layer in tile::underlay_layers(tile, cell.data, explorer.depth())
                .into_iter()
                .flatten()
                .chain(std::iter::once(tile))
            {
                let data = if layer == tile { cell.data } else { 0 };
                assets.render_tile(
                    screen,
                    layer,
                    data,
                    draw_x,
                    draw_y,
                    explorer.ticks as i32,
                    explorer.animation_enter_millis,
                    mask(explorer, x, y, layer),
                );
            }
        }
    }
    if explorer.snapshot.ready {
        let (source_x, flip) = match explorer.direction {
            (0, -1) => (16, false),
            (-1, 0) => (32, true),
            (1, 0) => (32, false),
            _ => (0, false),
        };
        screen.blit_region(
            assets.skin(),
            WIDTH as i32 / 2 - 8,
            HEIGHT as i32 / 2 - 8,
            source_x,
            assets.skin_row,
            16,
            16,
            flip,
        );
        // A small cyan flight marker distinguishes this from the Survival player.
        screen.rect(
            WIDTH as i32 / 2 - 3,
            HEIGHT as i32 / 2 + 9,
            6,
            1,
            0xFF50DDED,
        );
    }
    screen.rect(0, 0, WIDTH as i32, 24, 0xFF101820);
    screen.text(assets.font(), "INFDEV EXPERIMENT / FLIGHT", 4, 1);
    let position = format!("D:{} X:{center_x} Y:{center_y}", explorer.depth());
    // Two full i64 coordinates can exceed one text row. Keep both visible.
    if position.len() <= 35 {
        screen.text(assets.font(), &position, 4, 12);
    } else {
        screen.rect(0, 0, WIDTH as i32, 32, 0xFF101820);
        screen.text(assets.font(), &format!("X:{center_x}"), 4, 10);
        screen.text(
            assets.font(),
            &format!("D:{} Y:{center_y}", explorer.depth()),
            4,
            20,
        );
    }
    screen.rect(0, HEIGHT as i32 - 32, WIDTH as i32, 32, 0xFF101820);
    screen.text(
        assets.font(),
        if explorer.depth() < 0 {
            "MOVE / ATTACK: DIRT-ROCK"
        } else {
            "MOVE / ATTACK: GRASS-DAISY"
        },
        4,
        HEIGHT as i32 - 29,
    );
    screen.text(
        assets.font(),
        if explorer.is_layered() {
            "PICKUP: STAIRS / ESC: SAVE & QUIT"
        } else {
            "V2 SURFACE / ESC: SAVE & QUIT"
        },
        4,
        HEIGHT as i32 - 19,
    );
    let status = if explorer.closing {
        "SAVING... PLEASE WAIT"
    } else {
        explorer.status()
    };
    let ascii: String = status.chars().filter(char::is_ascii).take(35).collect();
    screen.text(assets.font(), &ascii, 4, HEIGHT as i32 - 9);
    if !explorer.closing
        && (!explorer.snapshot.ready || explorer.snapshot.save_failed)
        && explorer.status().contains("FAILED")
    {
        screen.rect(4, 40, WIDTH as i32 - 8, 98, 0xFF281820);
        let chars: Vec<_> = explorer.status().chars().filter(char::is_ascii).collect();
        for (i, line) in chars.chunks(33).take(8).enumerate() {
            screen.text_colored(
                assets.font(),
                &line.iter().collect::<String>(),
                8,
                44 + i as i32 * 9,
                0xff9999,
            );
        }
        screen.text(assets.font(), "ESC: RETRY SAVE / RETURN", 8, 123);
    }
}
