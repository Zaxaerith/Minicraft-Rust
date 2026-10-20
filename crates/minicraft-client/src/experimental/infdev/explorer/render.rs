use super::Explorer;
use crate::{
    assets::Assets,
    experimental::infdev::{Cell, Ground},
    gfx::{HEIGHT, Screen, WIDTH},
    level::tile::{self, Tile},
};

pub(crate) fn tile_for(cell: Cell) -> Tile {
    match cell.ground {
        Ground::Hole => Tile::Hole,
        Ground::WoodFloor => Tile::WoodFloor,
        Ground::RawStone => Tile::RawStone,
        Ground::StoneFloor => Tile::StoneFloor,
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
        if !explorer.is_survival() {
            screen.rect(
                WIDTH as i32 / 2 - 3,
                HEIGHT as i32 / 2 + 9,
                6,
                1,
                0xFF50DDED,
            );
        }
    }
    if let Some(s) = &explorer.snapshot.survival {
        draw_survival(explorer, s, screen, assets);
        return;
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

fn draw_survival(
    explorer: &Explorer,
    s: &super::super::survival::State,
    screen: &mut Screen,
    assets: &Assets,
) {
    use super::super::survival::{Loot, Panel, recipes};
    use crate::{
        item::{RecipeOutput, recipe_can_craft},
        world::{GameMode, hud, inventory_ui},
    };
    let (cx, cy) = explorer.position();
    let point = |x: i64, y: i64| {
        x.checked_sub(cx)
            .zip(y.checked_sub(cy))
            .filter(|(dx, dy)| dx.unsigned_abs() <= 11 && dy.unsigned_abs() <= 8)
            .map(|(dx, dy)| {
                (
                    WIDTH as i32 / 2 - 8 + dx as i32 * 16 + explorer.camera_offset.0,
                    HEIGHT as i32 / 2 - 8 + dy as i32 * 16 + explorer.camera_offset.1,
                )
            })
    };
    for station in &s.stations {
        if station.depth == explorer.depth()
            && let Some((x, y)) = point(station.x, station.y)
        {
            assets.furniture_linked(station.kind.furniture()).render(
                assets.sheets(),
                screen,
                x,
                y,
                0,
                false,
                0,
            );
        }
    }
    for page in explorer.entity_pages() {
        for entity in &page.entities {
            let at = page.position(entity);
            if let Some((x, y)) = point(at.x, at.y) {
                use super::super::entities::Kind;
                match &entity.kind {
                    Kind::DeathChest(chest) => {
                        assets
                            .furniture_linked(crate::entity::furniture::FurnitureKind::Chest)
                            .render(assets.sheets(), screen, x, y, 0, false, 0);
                        screen.text_colored(
                            assets.font(),
                            &format!("{}S", chest.remaining / 60),
                            x,
                            y - 8,
                            if chest.remaining < 1800 {
                                0xff7777
                            } else {
                                0xffffff
                            },
                        );
                    }
                    Kind::Pig(pig) => {
                        let d = if pig.mob.x_move < 0 {
                            2
                        } else if pig.mob.x_move > 0 {
                            3
                        } else if pig.mob.y_move < 0 {
                            1
                        } else {
                            0
                        };
                        assets
                            .mob_linked(
                                crate::entity::mob::NaturalMob::Pig,
                                d,
                                ((pig.mob.walk_distance >> 3) % 2) as usize,
                                false,
                            )
                            .render(
                                assets.sheets(),
                                screen,
                                x + i32::from(pig.px) - 8,
                                y + i32::from(pig.py) - 11,
                                0,
                                pig.mob.hurt_time > 0,
                                0,
                            );
                    }
                    Kind::Drop(drop) => {
                        screen.blit(
                            match drop.loot {
                                Loot::Stack(s) => assets.item_stack(s),
                                Loot::Tool(t) => assets.tool(t),
                            },
                            x + 4,
                            y + 4,
                        );
                    }
                }
            }
        }
    }
    for drop in &s.drops {
        if drop.depth == explorer.depth()
            && let Some((x, y)) = point(drop.x, drop.y)
        {
            screen.blit(
                match drop.loot {
                    Loot::Stack(st) => assets.item_stack(st),
                    Loot::Tool(t) => assets.tool(t),
                },
                x + 4,
                y + 4,
            );
        }
    }
    screen.rect(0, 0, WIDTH as i32, 24, 0xFF101820);
    screen.text(
        assets.font(),
        if s.food_rules == super::super::survival::FoodRules::Legacy {
            "INFDEV / LEGACY HUNGER OFF"
        } else {
            "INFDEV / SURVIVAL CHECKPOINT"
        },
        4,
        1,
    );
    screen.text(
        assets.font(),
        &format!("D:{} X:{cx} Y:{cy}", explorer.depth()),
        4,
        12,
    );
    screen.rect(0, HEIGHT as i32 - 40, WIDTH as i32, 40, 0xFF101820);
    screen.text(
        assets.font(),
        "X: BAG Z: CRAFT V/P: STAIRS",
        4,
        HEIGHT as i32 - 39,
    );
    let status = if explorer.closing {
        "SAVING..."
    } else {
        explorer.status()
    };
    screen.text(
        assets.font(),
        &status
            .chars()
            .filter(char::is_ascii)
            .take(35)
            .collect::<String>(),
        4,
        HEIGHT as i32 - 29,
    );
    hud::render_hud(
        screen,
        assets,
        &s.player,
        GameMode::Survival,
        0,
        explorer.depth(),
        0,
        1,
    );
    match s.panel {
        Panel::Inventory => {
            let mut entries = s
                .player
                .inventory
                .slots()
                .iter()
                .map(|st| {
                    (
                        assets.item_stack(*st),
                        format!(" {} {}", st.count, st.item.display_name()),
                    )
                })
                .collect::<Vec<_>>();
            entries.extend(
                s.player
                    .inventory
                    .tools()
                    .iter()
                    .map(|t| (assets.tool(*t), format!(" {}", t.display_name()))),
            );
            inventory_ui::render_inventory_panel(
                screen,
                assets,
                &entries,
                "Inventory",
                9,
                Some(s.selection),
            );
            if let Some(t) = s
                .player
                .inventory
                .tools()
                .get(s.selection.saturating_sub(s.player.inventory.slots().len()))
                && s.selection >= s.player.inventory.slots().len()
            {
                screen.text(
                    assets.font(),
                    &format!("Durability {}/{}", t.durability, t.max_durability),
                    12,
                    115,
                );
            }
            screen.text(assets.font(), "C: EQUIP / ESC: CLOSE", 12, 138);
        }
        Panel::Crafting(b) => {
            let recipes = recipes(b);
            let entries = recipes
                .iter()
                .map(|r| {
                    (
                        match r.output {
                            RecipeOutput::Stack(st) => assets.item_stack(st),
                            RecipeOutput::Tool(t) => assets.tool(t),
                        },
                        r.label(),
                    )
                })
                .collect::<Vec<_>>();
            inventory_ui::render_inventory_panel(
                screen,
                assets,
                &entries,
                b.map_or("Hand Crafting", |k| k.item().display_name()),
                9,
                Some(s.selection),
            );
            if let Some(r) = recipes.get(s.selection) {
                for (row, cost) in r.costs.iter().take(3).enumerate() {
                    screen.text(
                        assets.font(),
                        &format!(
                            "{} {}/{}",
                            cost.item.display_name(),
                            s.player.inventory.count(cost.item),
                            cost.count
                        ),
                        12,
                        110 + row as i32 * 9,
                    );
                }
                screen.text(
                    assets.font(),
                    if recipe_can_craft(r, &s.player.inventory, false) {
                        "C: CRAFT / ESC: CLOSE"
                    } else {
                        "MISSING MATERIALS"
                    },
                    12,
                    145,
                );
            }
        }
        Panel::Closed => (),
    }
    if s.panel == Panel::Closed
        && let Some(at) = s.death_site
    {
        screen.text(
            assets.font(),
            &format!("LAST DEATH D:{} X:{} Y:{}", at.depth, at.x, at.y)
                .chars()
                .take(35)
                .collect::<String>(),
            4,
            HEIGHT as i32 - 49,
        );
    }
    if s.player.health == 0 {
        screen.rect(8, 65, WIDTH as i32 - 16, 40, 0xFF281820);
        screen.text(assets.font(), "DEAD - ENTER TO RESPAWN", 12, 72);
        screen.text(
            assets.font(),
            if s.death_rule == super::super::survival::DeathRule::KeepInventory || s.death_retained
            {
                "INVENTORY SAFELY RETAINED"
            } else {
                "ITEMS IN DEATH CHEST"
            },
            12,
            85,
        );
    }
}
