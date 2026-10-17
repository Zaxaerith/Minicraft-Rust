//! Fixed Java dev2 Level constructor assembly. Historical generators stay separate.
//! Source: World.java / Level.java / Structure.java at 4dc00332 (GPL-3.0).
use super::*;

pub(super) fn build(
    seed: i64,
    spec: WorldSpec,
    options: PlayOptions,
    mut observe: impl FnMut(&Level, &str, u64, u32),
) -> World {
    let size = spec.size;
    let mut derivation = crate::level::noise::JavaRandom::new(seed);
    let mut levels: Vec<Level> = Vec::with_capacity(6);
    let mut seeds = [0; 6];
    for depth in (-4..=1).rev() {
        let layer_seed = derivation.next_i64();
        seeds[java_index(depth)] = layer_seed;
        let generated = generation::level_with_version(
            size,
            size,
            depth,
            layer_seed,
            spec,
            GenerationVersion::Java230World,
        );
        let mut level = Level {
            depth,
            tiles: generated.tiles,
            data: generated.data,
            max_mob_count: spawn::max_mob_count(depth, options.difficulty.min(2)),
            pending_spawns: Vec::new(),
            entities: EntityArena::default(),
        };
        // Level.random is independent of LevelGen.random. Spawner's transient
        // timer RNG is also separate and must never consume this assembly RNG.
        let mut random = crate::level::noise::JavaRandom::new(layer_seed);
        let mut count = 0;
        observe(&level, "raw", random.state(), count);
        if depth < 0 {
            spawners(&mut level, size, &mut random, &mut count);
            observe(&level, "spawners", random.state(), count);
        }
        if depth == 0 {
            villages(&mut level, size, &mut random);
            observe(&level, "villages", random.state(), count);
        }
        if depth == -4 {
            dungeons(&mut level, size, &mut random);
            observe(&level, "dungeons", random.state(), count);
        }
        if let Some(parent) = levels.last() {
            for y in 0..size {
                for x in 0..size {
                    if parent.tiles[x + y * size] != Tile::StairsDown {
                        continue;
                    }
                    match depth {
                        -4 => {
                            stamp(&mut level, size, x as i32, y as i32, GATE);
                            level.entities.spawn_furniture(
                                FurnitureKind::IronLantern,
                                (x as i32 - 1) * 16 + 8,
                                (y as i32 - 1) * 16 + 8,
                            );
                            boss_room(&mut level, size);
                            level
                                .entities
                                .import_statue(
                                    size as i32 / 2 * 16 + 8,
                                    size as i32 / 2 * 16 + 8,
                                    5000,
                                )
                                .expect("fixed statue health");
                        }
                        _ => {
                            for yy in y as i32 - 1..=y as i32 + 1 {
                                for xx in x as i32 - 1..=x as i32 + 1 {
                                    if !matches!(
                                        tile(&level, size, xx, yy),
                                        Some(Tile::StairsUp | Tile::StairsDown)
                                    ) {
                                        set(
                                            &mut level,
                                            size,
                                            xx,
                                            yy,
                                            if depth == 0 {
                                                Tile::HardRock
                                            } else {
                                                Tile::Dirt
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                    set(&mut level, size, x as i32, y as i32, Tile::StairsUp);
                }
            }
        } else {
            loop {
                let x = random.next_int(size as i32 - 7);
                let y = random.next_int(size as i32 - 5);
                if [(-3, -2), (3, -2), (-3, 2), (3, 2)]
                    .into_iter()
                    .all(|(dx, dy)| tile(&level, size, x + dx, y + dy) == Some(Tile::Cloud))
                {
                    stamp(&mut level, size, x, y, HOUSE);
                    level.entities.spawn_furniture(
                        FurnitureKind::GoldLantern,
                        (x - 2) * 16 + 8,
                        y * 16 + 8,
                    );
                    level.entities.spawn_furniture(
                        FurnitureKind::Enchanter,
                        x * 16 + 8,
                        y * 16 + 8,
                    );
                    break;
                }
            }
        }
        observe(&level, "connections", random.state(), count);
        if depth == -4 {
            // Original constructor calls checkChestCount(false): ten *additional*
            // chests, not ten minus already queued spawner/template chests.
            for _ in 0..10 * (size / 128) {
                let storage = loot(&mut random, "dungeonchest");
                loop {
                    let x = random.next_int(size as i32);
                    let y = random.next_int(size as i32);
                    if tile(&level, size, x, y) != Some(Tile::Grass) {
                        continue;
                    }
                    let (mut px, mut py) = (0, 0);
                    if random.next_bool() {
                        let mut s = x;
                        while s < size as i32 - s {
                            if matches!(
                                tile(&level, size, s, y),
                                Some(Tile::ObsidianWall | Tile::OrnateObsidian)
                            ) {
                                px = s * 20 - 16;
                                py = y * 24 - 14;
                            }
                            s += 1;
                        }
                    } else {
                        let mut s = y;
                        while s < size as i32 - s {
                            if matches!(
                                tile(&level, size, x, s),
                                Some(Tile::ObsidianWall | Tile::OrnateObsidian)
                            ) {
                                px = x * 23 - 14;
                                py = s * 21 - 16;
                            }
                            s += 1;
                        }
                    }
                    if px == 0 && py == 0 {
                        px = x * 16 - 8;
                        py = y * 16 - 8;
                    }
                    // Java getEntitiesInTiles only sees the committed entity
                    // set, empty throughout this constructor, not pending adds.
                    if tile(&level, size, px >> 4, py >> 4) == Some(Tile::ObsidianWall) {
                        set(&mut level, size, px >> 4, py >> 4, Tile::RawObsidian);
                    }
                    chest(&mut level, FurnitureKind::DungeonChest, px, py, storage);
                    count += 1;
                    break;
                }
            }
        }
        observe(&level, "chests", random.state(), count);
        if depth == 1 {
            level
                .entities
                .spawn_mob_level(
                    spawn::NaturalMob::AirWizard,
                    1,
                    options.difficulty.min(2) as u8,
                    false,
                    size as i32 / 2 * 16 + 8,
                    size as i32 / 2 * 16 + 8,
                )
                .expect("fixed boss level");
        }
        observe(&level, "final", random.state(), count);
        levels.push(level);
    }
    let mut player_random = crate::level::noise::JavaRandom::new(seed);
    let positions: Vec<_> = levels[1]
        .tiles
        .iter()
        .enumerate()
        .filter_map(|(i, t)| (*t == Tile::Grass).then_some(i))
        .collect();
    // Standard dev2 maps have grass. Keep the official fallback for extreme themes.
    let positions = if positions.is_empty() {
        levels[1]
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.may_spawn().then_some(i))
            .collect()
    } else {
        positions
    };
    let positions = if positions.is_empty() {
        levels[1]
            .tiles
            .iter()
            .zip(&levels[1].data)
            .enumerate()
            .filter_map(|(i, (t, d))| (!t.solid(*d)).then_some(i))
            .collect()
    } else {
        positions
    };
    let (x, y) = if positions.is_empty() {
        let x = player_random.next_int((size / 4) as i32) + size as i32 * 3 / 8;
        let y = player_random.next_int((size / 4) as i32) + size as i32 * 3 / 8;
        set(&mut levels[1], size, x, y, Tile::Grass);
        (x, y)
    } else {
        let i = positions[player_random.next_int(positions.len() as i32) as usize];
        ((i % size) as i32, (i / size) as i32)
    };
    let mut world = World::with_levels(
        seed,
        size,
        options,
        GenerationVersion::Java230World,
        levels,
        1,
        (x, y),
    );
    world.java_level_seeds = seeds.to_vec();
    world
}

pub(super) const fn java_index(depth: i8) -> usize {
    match depth {
        -4 => 5,
        _ => (depth + 3) as usize,
    }
}
fn tile(level: &Level, size: usize, x: i32, y: i32) -> Option<Tile> {
    (x >= 0 && y >= 0 && x < size as i32 && y < size as i32)
        .then(|| level.tiles[x as usize + y as usize * size])
}
fn set(level: &mut Level, size: usize, x: i32, y: i32, t: Tile) {
    if x >= 0 && y >= 0 && x < size as i32 && y < size as i32 {
        let i = x as usize + y as usize * size;
        level.tiles[i] = t;
        level.data[i] = 0;
    }
}

// Structure.setData deliberately transposes ASCII row/column. Preserve its
// width/height offsets even for the non-square sky house, including data reset.
fn stamp(level: &mut Level, size: usize, x: i32, y: i32, rows: &[&str]) {
    for (row, line) in rows.iter().enumerate() {
        for (column, key) in line.bytes().enumerate() {
            let t = match key {
                b'W' => Tile::ObsidianWall,
                b'O' => Tile::OrnateObsidian,
                b'D' => Tile::ObsidianDoor,
                b'G' => Tile::Grass,
                b'F' => Tile::Grass,
                b'P' => Tile::Flower230,
                b'B' => Tile::StoneFloor,
                b'S' => Tile::StoneWall,
                b'f' => Tile::WoodFloor,
                b'w' => Tile::WoodWall,
                b'd' => Tile::WoodDoor,
                b'b' => Tile::BossFloor,
                b'c' => Tile::BossWall,
                b'e' => Tile::BossDoor,
                b'o' => Tile::ObsidianFloor,
                b'*' => continue,
                _ => unreachable!("bundled structure key"),
            };
            set(
                level,
                size,
                x - rows[0].len() as i32 / 2 + row as i32,
                y - rows.len() as i32 / 2 + column as i32,
                t,
            );
        }
    }
}
const GATE: &[&str] = &["WWDWW", "WoooW", "DoooD", "WoooW", "WWDWW"];
const HOUSE: &[&str] = &["wwwwwww", "wfffffw", "dfffffw", "wfffffw", "wwwwwww"];
const NORMAL_HOUSE: &[&str] = &["wwwww", "wfffw", "wfffd", "wfffG", "wwwww"];
const TWO_HOUSE: &[&str] = &["wwwww", "wfffw", "dfffw", "wfffw", "wwdww"];
const OVERLAY1: &[&str] = &["**fG*", "f*GG*", "*G**f", "G*G**", "***G*"];
const OVERLAY2: &[&str] = &["f**G*", "*****", "*GG**", "f**G*", "*f**G"];
const ROOM: &[&str] = &[
    "WWWDWWW", "WOOOOOW", "WOFFFOW", "DOFFFOD", "WOFFFOW", "WOOOOOW", "WWWDWWW",
];
const GARDEN: &[&str] = &[
    "WWWDWWW", "WOOOOOW", "WOPPPOW", "DOPPPOD", "WOPPPOW", "WOOOOOW", "WWWDWWW",
];
const CAVES: [&[&str]; 5] = [
    &["SSBSS", "SBBBS", "BBBBB", "SBBBS", "SSBSS"],
    &["SSSSS", "SBBBB", "BBBBB", "SBBBB", "SSSSS"],
    &["SSSSS", "BBBBS", "BBBBB", "BBBBS", "SSSSS"],
    &["SBBBS", "SBBBS", "SBBBS", "SBBBS", "SSBSS"],
    &["SSBSS", "SBBBS", "SBBBS", "SBBBS", "SBBBS"],
];

pub(super) fn boss_room(level: &mut Level, size: usize) {
    stamp(
        level,
        size,
        size as i32 / 2,
        size as i32 / 2,
        &[
            "ccccecccc",
            "cbbbbbbbc",
            "cbbbbbbbc",
            "cbbbbbbbc",
            "ebbbbbbbe",
            "cbbbbbbbc",
            "cbbbbbbbc",
            "cbbbbbbbc",
            "ccccecccc",
        ],
    );
}
fn chest(level: &mut Level, kind: FurnitureKind, x: i32, y: i32, contents: Inventory) {
    let id = level.entities.spawn_furniture(kind, x, y);
    *level
        .entities
        .entities_mut_for_storage(id)
        .expect("new container") = contents;
}
fn spawners(
    level: &mut Level,
    size: usize,
    random: &mut crate::level::noise::JavaRandom,
    count: &mut u32,
) {
    let dungeon = level.depth == -4;
    let attempts = if dungeon {
        18
    } else {
        18 / -i32::from(level.depth)
    } * (size as i32 / 128);
    for _ in 0..attempts {
        let r = random.next_int(if dungeon { 2 } else { 5 });
        let species = if dungeon {
            if r == 1 {
                spawn::NaturalMob::Knight
            } else {
                spawn::NaturalMob::Snake
            }
        } else if r == 1 {
            spawn::NaturalMob::Skeleton
        } else if r == 0 || r == 2 {
            spawn::NaturalMob::Slime
        } else {
            spawn::NaturalMob::Zombie
        };
        let x = random.next_int(size as i32 * 16) / 16;
        let y = random.next_int(size as i32 * 16) / 16;
        if tile(level, size, x, y)
            != Some(if dungeon {
                Tile::ObsidianFloor
            } else {
                Tile::Dirt
            })
        {
            continue;
        }
        let wall = if dungeon {
            Tile::ObsidianWall
        } else {
            Tile::Rock
        };
        let (mut px, mut py) = (0, 0);
        if random.next_bool() {
            let mut s = x;
            while s < size as i32 - s {
                if tile(level, size, s, y) == Some(wall) {
                    px = s * 16 - 24;
                    py = y * 16 - 24;
                }
                s += 1;
            }
        } else {
            let mut s = y;
            while s < size as i32 - s {
                if tile(level, size, x, s) == Some(wall) {
                    px = x * 16 - 24;
                    py = s * 16 - 24;
                }
                s += 1;
            }
        }
        if px == 0 && py == 0 {
            px = x * 16 - 8;
            py = y * 16 - 8;
        }
        let (tx, ty) = (px >> 4, py >> 4);
        if tile(level, size, tx, ty) == Some(wall) {
            set(level, size, tx, ty, Tile::Dirt);
        }
        if dungeon {
            stamp(level, size, tx, ty, ROOM);
        } else {
            stamp(level, size, tx, ty, CAVES[0]);
            for (dx, dy, draw_x, draw_y, variant) in [
                (0, -4, 0, -5, 1),
                (0, 4, 0, 5, 2),
                (4, 0, 5, 0, 3),
                (-4, 0, -5, 0, 4),
            ] {
                if tile(level, size, tx + dx, ty + dy) == Some(Tile::Dirt) {
                    stamp(level, size, tx + draw_x, ty + draw_y, CAVES[variant]);
                }
            }
        }
        let kind = match species {
            spawn::NaturalMob::Skeleton => FurnitureKind::SkeletonSpawner,
            spawn::NaturalMob::Slime => FurnitureKind::SlimeSpawner,
            spawn::NaturalMob::Zombie => FurnitureKind::ZombieSpawner,
            spawn::NaturalMob::Knight => FurnitureKind::KnightSpawner,
            _ => FurnitureKind::SnakeSpawner,
        };
        let mut timer_random =
            crate::level::noise::JavaRandom::new(i64::from(px) ^ (i64::from(py) << 32));
        level
            .entities
            .import_spawner(
                kind,
                px,
                py,
                if dungeon { 2 } else { (-level.depth) as u8 },
                &mut timer_random,
            )
            .expect("fixed spawner configuration");
        for rpt in 0..2 {
            if random.next_int(2) != 0 {
                continue;
            }
            let storage = loot(
                random,
                if dungeon {
                    "dungeonchest"
                } else {
                    "minidungeon"
                },
            );
            if dungeon {
                *count += 1;
            }
            chest(
                level,
                if dungeon {
                    FurnitureKind::DungeonChest
                } else {
                    FurnitureKind::Chest
                },
                px - 16 + rpt * 32,
                py - 16,
                storage,
            );
        }
    }
}
fn villages(level: &mut Level, size: usize, random: &mut crate::level::noise::JavaRandom) {
    let (mut last_x, mut last_y) = (0i32, 0i32);
    for _ in 0..size / 128 * 2 {
        for _ in 0..10 {
            let x = random.next_int(size as i32);
            let y = random.next_int(size as i32);
            if tile(level, size, x, y) != Some(Tile::Grass)
                || (x - last_x).abs() <= 16
                || (y - last_y).abs() <= 16
            {
                continue;
            }
            last_x = x;
            last_y = y;
            let houses = random.next_int(3) + 2;
            for house in 0..houses {
                let has_chest = random.next_bool();
                let two = random.next_bool();
                let overlay = random.next_int(2);
                let xo = if house == 0 || house == 3 { -4 } else { 4 } + random.next_int(5) - 2;
                let yo = if house < 2 { -4 } else { 4 } + random.next_int(5) - 2;
                stamp(
                    level,
                    size,
                    x + xo,
                    y + yo,
                    if two { TWO_HOUSE } else { NORMAL_HOUSE },
                );
                stamp(
                    level,
                    size,
                    x + xo,
                    y + yo,
                    if overlay == 0 { OVERLAY1 } else { OVERLAY2 },
                );
                if has_chest {
                    let storage = loot(random, "villagehouse");
                    let px = (x + random.next_int(2) + xo) * 16;
                    let py = (y + random.next_int(2) + yo) * 16;
                    chest(level, FurnitureKind::Chest, px, py, storage);
                }
            }
            break;
        }
    }
}
fn dungeons(level: &mut Level, size: usize, random: &mut crate::level::noise::JavaRandom) {
    // Java `i < Math.sqrt(w)` rounds UP, relevant at128/512 (not floor).
    let mut i = 0;
    while (i as f64) < (size as f64).sqrt() {
        i += 1;
        let x = random.next_int(size as i32 - 2) + 1;
        let y = random.next_int(size as i32 - 2) + 1;
        if x <= 8 || y <= 8 || x >= size as i32 - 8 || y >= size as i32 - 8 {
            continue;
        }
        if random.next_int(2) == 1 {
            stamp(level, size, x, y, GARDEN);
        } else {
            stamp(level, size, x, y, ROOM);
            let storage = loot(random, "dungeonchest");
            chest(
                level,
                FurnitureKind::DungeonChest,
                x * 16 + 8,
                y * 16 + 8,
                storage,
            );
        }
    }
}
fn loot(random: &mut crate::level::noise::JavaRandom, table: &str) -> Inventory {
    let text = match table {
        "villagehouse" => {
            include_str!("../../../../assets/resources/data/chestloot/villagehouse.txt")
        }
        "minidungeon" => {
            include_str!("../../../../assets/resources/data/chestloot/minidungeon.txt")
        }
        "dungeonchest" => {
            include_str!("../../../../assets/resources/data/chestloot/dungeonchest.txt")
        }
        _ => unreachable!("bundled loot table"),
    };
    let mut inventory = Inventory::new(27);
    for line in text.lines() {
        if let Some(fallback) = line.strip_prefix(':') {
            if inventory.used_slots() == 0 {
                for entry in fallback.split(':') {
                    let (name, count) = entry.split_once(',').expect("bundled fallback");
                    add_loot(
                        &mut inventory,
                        name,
                        count.parse().expect("bundled quantity"),
                    );
                }
            }
            continue;
        }
        let fields: Vec<_> = line.split(',').collect();
        let chance = fields[0].parse::<i32>().expect("bundled chance");
        let count = fields
            .get(2)
            .map_or(1, |v| v.parse::<u16>().expect("bundled count"));
        // Only tools have multiple independent rolls for quantity>1 in these
        // tables. Armor and potions are Java StackableItem, not tool instances.
        if tool(fields[1]).is_some() {
            for _ in 0..count {
                if random.next_int(chance) == 0 {
                    add_loot(&mut inventory, fields[1], 1);
                }
            }
        } else if random.next_int(chance) == 0 {
            // dev2 WoolItem.copy constructs a fresh count1 item. Official
            // Chest.populateInvRandom therefore awards1, although the table
            // says12. Restrict this historical behavior to NEW Java-world loot;
            // never reset an existing inventory/transfer/imported wool stack.
            let count = if fields[1].eq_ignore_ascii_case("wool") {
                1
            } else {
                count
            };
            add_loot(&mut inventory, fields[1], count);
        }
    }
    inventory
}
fn tool(name: &str) -> Option<ToolItem> {
    ToolKind::ALL
        .into_iter()
        .flat_map(|kind| {
            ToolTier::ALL
                .into_iter()
                .map(move |tier| ToolItem::new(kind, tier))
        })
        .find(|t| t.display_name().eq_ignore_ascii_case(name))
}
fn add_loot(inventory: &mut Inventory, name: &str, count: u16) {
    if let Some(t) = tool(name) {
        for _ in 0..count {
            inventory.add_tool(t).expect("bundled table fits chest");
        }
    } else {
        let item = crate::item::items::get(name).expect("bundled loot item");
        assert_eq!(inventory.add(item, count), 0, "bundled table fits chest");
    }
}

/// Activation-only room repair. Saved maps and level switches never call this.
/// Preserve player-installed non-arena tiles and every entity/container. This
/// explicit safety deviation avoids Java's blanket replacement of player work.
pub(super) fn repair_boss_room(level: &mut Level, size: usize) {
    if level.depth != -4 {
        return;
    }
    let mut protected = Vec::new();
    for y in size / 2 - 4..=size / 2 + 4 {
        for x in size / 2 - 4..=size / 2 + 4 {
            let i = x + y * size;
            if !matches!(
                level.tiles[i],
                Tile::BossWall | Tile::BossFloor | Tile::BossDoor
            ) {
                protected.push((i, level.tiles[i], level.data[i]));
            }
        }
    }
    boss_room(level, size);
    for (i, tile, data) in protected {
        level.tiles[i] = tile;
        level.data[i] = data;
    }
}
