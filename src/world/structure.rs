use super::{Level, Tile, random::JavaRandom};

pub fn decorate(level: &mut Level, width: usize, height: usize, seed: i64) {
    let mut random = JavaRandom::new(seed);
    match level.depth {
        1 => place_air_wizard_house(level, width, height, &mut random),
        0 => place_villages(level, width, height, &mut random),
        -3..=-1 => place_cave_dungeons(level, width, height, &mut random),
        -4 => {
            place_dungeon_spawners(level, width, height, &mut random);
            place_dungeon_rooms(level, width, height, &mut random);
        }
        _ => {}
    }
}

pub fn link_from_parent(level: &mut Level, width: usize, height: usize, parent_stairs: &[usize]) {
    for &index in parent_stairs {
        let x = index % width;
        let y = index / width;
        match level.depth {
            0 => fill_area(level, width, height, x, y, 1, Tile::HardRock),
            -3..=-1 => fill_area(level, width, height, x, y, 1, Tile::Dirt),
            -4 => {
                dungeon_gate(&mut level.tiles, width, height, x, y);
                dungeon_boss_room(&mut level.tiles, width, height, width / 2, height / 2);
            }
            _ => {}
        }
        set(level, width, height, x as i32, y as i32, Tile::StairsUp);
    }
}

pub fn ornate_lava_pool(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(
        map,
        width,
        height,
        x,
        y,
        &[
            "WWWDWWW", "WOOOOOW", "WOLLLOW", "DOLLLOD", "WOLLLOW", "WOOOOOW", "WWWDWWW",
        ],
        |key| match key {
            'L' => Some(Tile::Lava),
            'W' => Some(Tile::ObsidianWall),
            'O' => Some(Tile::OrnateObsidian),
            'D' => Some(Tile::ObsidianDoor),
            _ => None,
        },
    );
}

#[allow(dead_code)] // Registered by 2.2.4; its current generator uses the ornate variant.
fn lava_pool(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(map, width, height, x, y, &["LL", "LL"], |key| {
        (key == 'L').then_some(Tile::Lava)
    });
}

pub fn dungeon_lock(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(
        map,
        width,
        height,
        x,
        y,
        &["WWWWW", "WOOOW", "WOOOW", "WOOOW", "WWWWW"],
        |key| match key {
            'W' => Some(Tile::ObsidianWall),
            'O' => Some(Tile::ObsidianFloor),
            _ => None,
        },
    );
}

fn dungeon_gate(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(
        map,
        width,
        height,
        x,
        y,
        &["WWDWW", "WOOOW", "DOOOD", "WOOOW", "WWDWW"],
        |key| match key {
            'W' => Some(Tile::ObsidianWall),
            'O' => Some(Tile::ObsidianFloor),
            'D' => Some(Tile::ObsidianDoor),
            _ => None,
        },
    );
}

fn dungeon_boss_room(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(
        map,
        width,
        height,
        x,
        y,
        &[
            "WWWWDWWWW",
            "WOOOOOOOW",
            "WOOOOOOOW",
            "WOOOOOOOW",
            "DOOOOOOOD",
            "WOOOOOOOW",
            "WOOOOOOOW",
            "WOOOOOOOW",
            "WWWWDWWWW",
        ],
        |key| match key {
            'W' => Some(Tile::BossWall),
            'O' => Some(Tile::BossFloor),
            'D' => Some(Tile::BossDoor),
            _ => None,
        },
    );
}

fn place_air_wizard_house(level: &mut Level, width: usize, height: usize, random: &mut JavaRandom) {
    for _ in 0..width * height {
        let x = random.next_int((width - 7) as i32);
        let y = random.next_int((height - 5) as i32);
        if corners_are(level, width, height, x, y, 3, 2, Tile::Cloud) {
            draw(
                &mut level.tiles,
                width,
                height,
                x as usize,
                y as usize,
                &["WWWWWWW", "WFFFFFW", "DFFFFFW", "WFFFFFW", "WWWWWWW"],
                |key| match key {
                    'F' => Some(Tile::WoodFloor),
                    'W' => Some(Tile::WoodWall),
                    'D' => Some(Tile::WoodDoor),
                    _ => None,
                },
            );
            return;
        }
    }
}

fn place_villages(level: &mut Level, width: usize, height: usize, random: &mut JavaRandom) {
    let mut last = (0_i32, 0_i32);
    for _ in 0..width / 128 * 2 {
        for _ in 0..10 {
            let x = random.next_int(width as i32);
            let y = random.next_int(height as i32);
            let index = x as usize + y as usize * width;
            if level.tiles[index] != Tile::Grass
                || (x - last.0).abs() <= 16
                || (y - last.1).abs() <= 16
            {
                continue;
            }
            last = (x, y);
            let houses = random.next_int(3) + 2;
            for house in 0..houses {
                let _has_chest = random.next_bool();
                let two_doors = random.next_bool();
                let overlay = random.next_int(2);
                let mut offset_x = if house == 0 || house == 3 { -4 } else { 4 };
                let mut offset_y = if house < 2 { -4 } else { 4 };
                offset_x += random.next_int(5) - 2;
                offset_y += random.next_int(5) - 2;
                village_house(
                    &mut level.tiles,
                    width,
                    height,
                    (x + offset_x) as usize,
                    (y + offset_y) as usize,
                    two_doors,
                );
                ruined_overlay(
                    &mut level.tiles,
                    width,
                    height,
                    (x + offset_x) as usize,
                    (y + offset_y) as usize,
                    overlay,
                );
            }
            break;
        }
    }
}

fn village_house(
    map: &mut [Tile],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    two_doors: bool,
) {
    let rows: &[&str] = if two_doors {
        &["WWWWW", "WFFFW", "DFFFW", "WFFFW", "WWDWW"]
    } else {
        &["WWWWW", "WFFFW", "WFFFD", "WFFFG", "WWWWW"]
    };
    draw(map, width, height, x, y, rows, |key| match key {
        'F' => Some(Tile::WoodFloor),
        'W' => Some(Tile::WoodWall),
        'D' => Some(Tile::WoodDoor),
        'G' => Some(Tile::Grass),
        _ => None,
    });
}

fn ruined_overlay(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize, variant: i32) {
    let rows: &[&str] = if variant == 0 {
        &["**FG*", "F*GG*", "*G**F", "G*G**", "***G*"]
    } else {
        &["F**G*", "*****", "*GG**", "F**G*", "*F**G"]
    };
    draw(map, width, height, x, y, rows, |key| match key {
        'F' => Some(Tile::WoodFloor),
        'G' => Some(Tile::Grass),
        _ => None,
    });
}

fn place_cave_dungeons(level: &mut Level, width: usize, height: usize, random: &mut JavaRandom) {
    let attempts = 18 / (-level.depth as usize) * (width / 128);
    for _ in 0..attempts {
        let _mob_type = random.next_int(5);
        let x3 = random.next_int((16 * width) as i32) as usize / 16;
        let y3 = random.next_int((16 * height) as i32) as usize / 16;
        let mut x = x3.saturating_sub(1);
        let mut y = y3.saturating_sub(1);
        let mut found = false;
        if level.tiles[x3 + y3 * width] != Tile::Dirt {
            continue;
        }
        let horizontal = random.next_bool();
        if horizontal {
            let mut scan = x3;
            while scan < width.saturating_sub(scan) {
                if level.tiles[scan + y3 * width] == Tile::Rock {
                    x = scan.saturating_sub(2);
                    y = y3.saturating_sub(2);
                    found = true;
                }
                scan += 1;
            }
        } else {
            let mut scan = y3;
            while scan < height.saturating_sub(scan) {
                if level.tiles[x3 + scan * width] == Tile::Rock {
                    x = x3.saturating_sub(2);
                    y = scan.saturating_sub(2);
                    found = true;
                }
                scan += 1;
            }
        }
        if !found {
            x = x3.saturating_sub(1);
            y = y3.saturating_sub(1);
        }
        if level.tiles[x + y * width] == Tile::Rock {
            level.tiles[x + y * width] = Tile::Dirt;
        }
        if x < 7 || y < 7 || x + 7 >= width || y + 7 >= height {
            continue;
        }
        cave_room(&mut level.tiles, width, height, x, y, 0);
        if level.tiles[x + (y - 4) * width] == Tile::Dirt {
            cave_room(&mut level.tiles, width, height, x, y - 5, 1);
        }
        if level.tiles[x + (y + 4) * width] == Tile::Dirt {
            cave_room(&mut level.tiles, width, height, x, y + 5, 2);
        }
        if level.tiles[x + 4 + y * width] == Tile::Dirt {
            cave_room(&mut level.tiles, width, height, x + 5, y, 3);
        }
        if level.tiles[x - 4 + y * width] == Tile::Dirt {
            cave_room(&mut level.tiles, width, height, x - 5, y, 4);
        }
        for _ in 0..2 {
            let _chest = random.next_int(2);
        }
    }
}

fn place_dungeon_spawners(level: &mut Level, width: usize, height: usize, random: &mut JavaRandom) {
    for _ in 0..18 * (width / 128) {
        let _mob_type = random.next_int(2);
        let x3 = random.next_int((16 * width) as i32) as usize / 16;
        let y3 = random.next_int((16 * height) as i32) as usize / 16;
        if level.tiles[x3 + y3 * width] != Tile::ObsidianFloor {
            continue;
        }
        let horizontal = random.next_bool();
        let mut x = x3.saturating_sub(1);
        let mut y = y3.saturating_sub(1);
        let mut found = false;
        if horizontal {
            let mut scan = x3;
            while scan < width.saturating_sub(scan) {
                if level.tiles[scan + y3 * width] == Tile::ObsidianWall {
                    x = scan.saturating_sub(2);
                    y = y3.saturating_sub(2);
                    found = true;
                }
                scan += 1;
            }
        } else {
            let mut scan = y3;
            while scan < height.saturating_sub(scan) {
                if level.tiles[x3 + scan * width] == Tile::ObsidianWall {
                    x = x3.saturating_sub(2);
                    y = scan.saturating_sub(2);
                    found = true;
                }
                scan += 1;
            }
        }
        if !found {
            x = x3.saturating_sub(1);
            y = y3.saturating_sub(1);
        }
        if x < 4 || y < 4 || x + 4 >= width || y + 4 >= height {
            continue;
        }
        if level.tiles[x + y * width] == Tile::ObsidianWall {
            level.tiles[x + y * width] = Tile::Dirt;
        }
        dungeon_spawner(&mut level.tiles, width, height, x, y);
        for _ in 0..2 {
            let _chest = random.next_int(2);
        }
    }
}

fn dungeon_spawner(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(
        map,
        width,
        height,
        x,
        y,
        &[
            "WWWDWWW", "WOOOOOW", "WOFFFOW", "DOFFFOD", "WOFFFOW", "WOOOOOW", "WWWDWWW",
        ],
        |key| match key {
            'F' => Some(Tile::Grass),
            'W' => Some(Tile::ObsidianWall),
            'O' => Some(Tile::OrnateObsidian),
            'D' => Some(Tile::ObsidianDoor),
            _ => None,
        },
    );
}

fn cave_room(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize, variant: usize) {
    let patterns = [
        ["WWBWW", "WBBBW", "BBBBB", "WBBBW", "WWBWW"],
        ["WWWWW", "WBBBB", "BBBBB", "WBBBB", "WWWWW"],
        ["WWWWW", "BBBBW", "BBBBB", "BBBBW", "WWWWW"],
        ["WBBBW", "WBBBW", "WBBBW", "WBBBW", "WWBWW"],
        ["WWBWW", "WBBBW", "WBBBW", "WBBBW", "WBBBW"],
    ];
    draw(
        map,
        width,
        height,
        x,
        y,
        &patterns[variant],
        |key| match key {
            'B' => Some(Tile::StoneFloor),
            'W' => Some(Tile::StoneWall),
            _ => None,
        },
    );
}

fn place_dungeon_rooms(level: &mut Level, width: usize, height: usize, random: &mut JavaRandom) {
    for _ in 0..(width as f64).sqrt() as usize {
        let x = random.next_int(width as i32 - 2) as usize + 1;
        let y = random.next_int(height as i32 - 2) as usize + 1;
        if x <= 8 || y <= 8 || x >= width - 8 || y >= height - 8 {
            continue;
        }
        let floor = if random.next_bool() {
            Tile::Flower
        } else {
            Tile::Grass
        };
        draw(
            &mut level.tiles,
            width,
            height,
            x,
            y,
            &[
                "WWWDWWW", "WOOOOOW", "WOFFFOW", "DOFFFOD", "WOFFFOW", "WOOOOOW", "WWWDWWW",
            ],
            |key| match key {
                'F' => Some(floor),
                'W' => Some(Tile::ObsidianWall),
                'O' => Some(Tile::OrnateObsidian),
                'D' => Some(Tile::ObsidianDoor),
                _ => None,
            },
        );
    }
}

fn draw(
    map: &mut [Tile],
    width: usize,
    height: usize,
    center_x: usize,
    center_y: usize,
    rows: &[&str],
    tile_for: impl Fn(char) -> Option<Tile>,
) {
    let pattern_width = rows.first().map_or(0, |row| row.chars().count());
    let pattern_height = rows.len();
    // This intentionally preserves Structure#setData's 2.2.4 rotation.
    for (row, line) in rows.iter().enumerate() {
        for (column, key) in line.chars().enumerate() {
            let Some(tile) = tile_for(key) else { continue };
            let x = center_x as i32 - pattern_width as i32 / 2 + row as i32;
            let y = center_y as i32 - pattern_height as i32 / 2 + column as i32;
            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                map[x as usize + y as usize * width] = tile;
            }
        }
    }
}

fn fill_area(
    level: &mut Level,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    radius: usize,
    tile: Tile,
) {
    for yy in y.saturating_sub(radius)..=(y + radius).min(height - 1) {
        for xx in x.saturating_sub(radius)..=(x + radius).min(width - 1) {
            set(level, width, height, xx as i32, yy as i32, tile);
        }
    }
}

fn set(level: &mut Level, width: usize, height: usize, x: i32, y: i32, tile: Tile) {
    if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
        let index = x as usize + y as usize * width;
        level.tiles[index] = tile;
        level.data[index] = 0;
    }
}

#[allow(clippy::too_many_arguments)]
fn corners_are(
    level: &Level,
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    tile: Tile,
) -> bool {
    [
        (x - dx, y - dy),
        (x + dx, y - dy),
        (x - dx, y + dy),
        (x + dx, y + dy),
    ]
    .into_iter()
    .all(|(xx, yy)| {
        xx >= 0
            && yy >= 0
            && xx < width as i32
            && yy < height as i32
            && level.tiles[xx as usize + yy as usize * width] == tile
    })
}

#[cfg(test)]
mod tests {
    use super::{Level, Tile, dungeon_boss_room, dungeon_spawner, lava_pool, ornate_lava_pool};

    #[test]
    fn canonical_structures_draw_expected_materials() {
        let mut map = vec![Tile::Dirt; 21 * 21];
        ornate_lava_pool(&mut map, 21, 21, 10, 10);
        assert!(map.contains(&Tile::Lava));
        assert!(map.contains(&Tile::OrnateObsidian));
        lava_pool(&mut map, 21, 21, 3, 3);
        dungeon_spawner(&mut map, 21, 21, 16, 16);
        dungeon_boss_room(&mut map, 21, 21, 10, 10);
        assert!(map.contains(&Tile::BossWall));
        assert!(map.contains(&Tile::BossDoor));

        let level = Level {
            depth: -4,
            tiles: map,
            data: vec![0; 21 * 21],
            max_mob_count: 200,
            pending_spawns: Vec::new(),
        };
        assert_eq!(level.tiles.len(), level.data.len());
    }
}
