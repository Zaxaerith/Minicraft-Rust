use super::{Level, Tile, random::JavaRandom};

pub fn tick_random_tiles(level: &mut Level, width: usize, height: usize, random: &mut JavaRandom) {
    for _ in 0..width * height / 50 {
        let x = random.next_int(width as i32) as usize;
        let y = random.next_int(height as i32) as usize;
        tick(level, width, height, x, y, random);
    }
}

fn tick(
    level: &mut Level,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    random: &mut JavaRandom,
) {
    let index = x + y * width;
    let tile = level.tiles[index];
    match tile {
        Tile::Grass | Tile::Flower => {
            let chance = if tile == Tile::Grass { 40 } else { 30 };
            if random.next_int(chance) == 0
                && let Some(adjacent) = random_adjacent(width, height, x, y, random)
                && level.tiles[adjacent] == Tile::Dirt
            {
                set(level, adjacent, Tile::Grass, 0);
            }
        }
        Tile::Water | Tile::Lava => {
            if let Some(adjacent) = random_adjacent(width, height, x, y, random)
                && level.tiles[adjacent] == Tile::Hole
            {
                set(level, adjacent, tile, 0);
            }
            if tile == Tile::Water {
                for adjacent in cardinal(width, height, x, y) {
                    if level.tiles[adjacent] == Tile::Lava {
                        set(level, adjacent, Tile::RawObsidian, 0);
                    }
                }
            }
        }
        Tile::TreeSapling | Tile::CactusSapling => {
            let age = level.data[index].saturating_add(1);
            if age > 110 {
                let grown = if tile == Tile::TreeSapling {
                    Tile::Tree
                } else {
                    Tile::Cactus
                };
                set(level, index, grown, 0);
            } else {
                level.data[index] = age;
            }
        }
        Tile::Farmland => tick_farmland(level, width, height, x, y, random),
        Tile::Wheat
        | Tile::Potato
        | Tile::Tomato
        | Tile::Carrot
        | Tile::HeavenlyBerries
        | Tile::HellishBerries => tick_crop(level, width, height, x, y, random),
        Tile::Rock
        | Tile::Tree
        | Tile::HardRock
        | Tile::Cactus
        | Tile::WoodWall
        | Tile::StoneWall
        | Tile::ObsidianWall => {
            level.data[index] = level.data[index].saturating_sub(1);
        }
        _ => {}
    }
}

fn tick_farmland(
    level: &mut Level,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    random: &mut JavaRandom,
) {
    let index = x + y * width;
    let moisture = level.data[index] & 7;
    if has_water(level, width, height, x, y, 4) {
        if moisture < 7 && random.next_int(10) == 0 {
            level.data[index] = moisture + 1;
        }
    } else if moisture > 0 && random.next_int(10) == 0 {
        level.data[index] = moisture - 1;
    } else if moisture == 0 && random.next_int(10) == 0 {
        set(level, index, Tile::Dirt, 0);
    }
}

fn tick_crop(
    level: &mut Level,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    random: &mut JavaRandom,
) {
    let index = x + y * width;
    let mut data = level.data[index];
    let moisture = data & 7;
    if has_water(level, width, height, x, y, 4) {
        if moisture < 7 && random.next_int(10) == 0 {
            // 2.2.4 assigns the value of moisture++ (the old value).
            data = (data & !7) | moisture;
        }
    } else if moisture > 0 && random.next_int(10) == 0 {
        // Likewise, moisture-- stores the old value in the Java baseline.
        data = (data & !7) | moisture;
    }

    let stage = (data >> 3) & 7;
    let fertilization = data >> 7;
    if stage < 7 {
        let mut points = if moisture > 0 { 4.0 } else { 2.0 };
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let Some(adjacent) = at(width, height, x, y, dx, dy) else {
                    continue;
                };
                if is_farm(level.tiles[adjacent]) {
                    points += if level.data[adjacent] & 7 > 0 {
                        0.75
                    } else {
                        0.25
                    };
                }
            }
        }

        let same = |dx, dy| {
            at(width, height, x, y, dx, dy)
                .is_some_and(|adjacent| level.tiles[adjacent] == level.tiles[index])
        };
        let up = same(0, -1);
        let down = same(0, 1);
        let left = same(-1, 0);
        let right = same(1, 0);
        let up_left = same(-1, -1);
        let down_left = same(-1, 1);
        let up_right = same(1, -1);
        let down_right = same(1, 1);
        if up && down && left && right && up_left && down_left && up_right && down_right {
            points /= 2.0;
        } else {
            if up && down && left && right {
                points *= 0.75;
            }
            if (up && down && (left || right)) || (up && left && right) || (down && left && right) {
                points *= 0.85;
            }
            if (up_left && (down_right || down_left || up_right))
                || (down_left && (up_right || down_right))
                || (up_right && down_right)
            {
                points *= 0.9;
            }
            for diagonal in [up_left, down_left, up_right, down_right] {
                if diagonal {
                    points *= 0.98125;
                }
            }
        }

        let bound = (100.0 / points) as i32 + 1;
        if random.next_int(bound) < (fertilization / 30 + 1) as i32 {
            data = (data & !(7 << 3)) | ((stage + 1) << 3);
        }
    }
    if fertilization > 0 {
        data = (data & 0x3f) | ((fertilization - 1) << 7);
    }
    level.data[index] = data;
}

fn is_farm(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::Farmland
            | Tile::Wheat
            | Tile::Potato
            | Tile::Tomato
            | Tile::Carrot
            | Tile::HeavenlyBerries
            | Tile::HellishBerries
    )
}

fn at(width: usize, height: usize, x: usize, y: usize, dx: i32, dy: i32) -> Option<usize> {
    let xx = x as i32 + dx;
    let yy = y as i32 + dy;
    (xx >= 0 && yy >= 0 && xx < width as i32 && yy < height as i32)
        .then_some(xx as usize + yy as usize * width)
}

fn has_water(
    level: &Level,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    radius: usize,
) -> bool {
    let left = x.saturating_sub(radius);
    let right = (x + radius).min(width - 1);
    let top = y.saturating_sub(radius);
    let bottom = (y + radius).min(height - 1);
    (top..=bottom).any(|yy| (left..=right).any(|xx| level.tiles[xx + yy * width] == Tile::Water))
}

fn random_adjacent(
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    random: &mut JavaRandom,
) -> Option<usize> {
    let (next_x, next_y) = if random.next_bool() {
        (x as i32 + if random.next_bool() { 1 } else { -1 }, y as i32)
    } else {
        (x as i32, y as i32 + if random.next_bool() { 1 } else { -1 })
    };
    (next_x >= 0 && next_y >= 0 && next_x < width as i32 && next_y < height as i32)
        .then_some(next_x as usize + next_y as usize * width)
}

fn cardinal(width: usize, height: usize, x: usize, y: usize) -> Vec<usize> {
    [(-1, 0), (1, 0), (0, -1), (0, 1)]
        .into_iter()
        .filter_map(|(dx, dy)| {
            let xx = x as i32 + dx;
            let yy = y as i32 + dy;
            (xx >= 0 && yy >= 0 && xx < width as i32 && yy < height as i32)
                .then_some(xx as usize + yy as usize * width)
        })
        .collect()
}

fn set(level: &mut Level, index: usize, tile: Tile, data: u16) {
    level.tiles[index] = tile;
    level.data[index] = data;
}

#[cfg(test)]
mod tests {
    use super::{Level, Tile, tick};
    use crate::world::random::JavaRandom;

    #[test]
    fn water_solidifies_neighboring_lava() {
        let mut level = Level {
            depth: 0,
            tiles: vec![Tile::Grass; 9],
            data: vec![0; 9],
            max_mob_count: 200,
            pending_spawns: Vec::new(),
        };
        level.tiles[4] = Tile::Water;
        level.tiles[5] = Tile::Lava;
        tick(&mut level, 3, 3, 1, 1, &mut JavaRandom::new(1));
        assert_eq!(level.tiles[5], Tile::RawObsidian);
    }

    #[test]
    fn saplings_preserve_and_advance_tile_data() {
        let mut level = Level {
            depth: 0,
            tiles: vec![Tile::TreeSapling],
            data: vec![110],
            max_mob_count: 200,
            pending_spawns: Vec::new(),
        };
        tick(&mut level, 1, 1, 0, 0, &mut JavaRandom::new(1));
        assert_eq!(level.tiles[0], Tile::Tree);
        assert_eq!(level.data[0], 0);
    }

    #[test]
    fn crop_data_keeps_java_stage_and_fertilizer_bit_layout() {
        let mut level = Level {
            depth: 0,
            tiles: vec![Tile::Wheat],
            data: vec![(7 << 3) | (2 << 7)],
            max_mob_count: 200,
            pending_spawns: Vec::new(),
        };
        tick(&mut level, 1, 1, 0, 0, &mut JavaRandom::new(1));
        assert_eq!(level.data[0] & 7, 0);
        assert_eq!((level.data[0] >> 3) & 7, 7);
        assert_eq!(level.data[0] >> 7, 1);
    }
}
