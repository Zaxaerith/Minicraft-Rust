use super::{TerrainType, Theme, Tile, WorldSpec, random::JavaRandom};

const STAIR_RADIUS: i32 = 15;

struct Noise {
    width: usize,
    height: usize,
    values: Vec<f64>,
}

impl Noise {
    fn new(width: usize, height: usize, feature_size: usize, random: &mut JavaRandom) -> Self {
        let mut noise = Self {
            width,
            height,
            values: vec![0.0; width * height],
        };
        for y in (0..height).step_by(feature_size) {
            for x in (0..width).step_by(feature_size) {
                noise.set(x as i32, y as i32, random.next_float() * 2.0 - 1.0);
            }
        }

        let mut step_size = feature_size;
        let mut scale = 2.0 / width as f64;
        let mut scale_modifier = 1.0;
        while step_size > 1 {
            let half = step_size / 2;
            for y in (0..height).step_by(step_size) {
                for x in (0..width).step_by(step_size) {
                    let average = (noise.sample(x as i32, y as i32)
                        + noise.sample((x + step_size) as i32, y as i32)
                        + noise.sample(x as i32, (y + step_size) as i32)
                        + noise.sample((x + step_size) as i32, (y + step_size) as i32))
                        / 4.0;
                    let value =
                        average + (random.next_float() * 2.0 - 1.0) * step_size as f64 * scale;
                    noise.set((x + half) as i32, (y + half) as i32, value);
                }
            }
            for y in (0..height).step_by(step_size) {
                for x in (0..width).step_by(step_size) {
                    let x = x as i32;
                    let y = y as i32;
                    let step = step_size as i32;
                    let half = half as i32;
                    let horizontal = (noise.sample(x, y)
                        + noise.sample(x + step, y)
                        + noise.sample(x + half, y + half)
                        + noise.sample(x + half, y - half))
                        / 4.0
                        + (random.next_float() * 2.0 - 1.0) * step_size as f64 * scale * 0.5;
                    let vertical = (noise.sample(x, y)
                        + noise.sample(x, y + step)
                        + noise.sample(x + half, y + half)
                        + noise.sample(x - half, y + half))
                        / 4.0
                        + (random.next_float() * 2.0 - 1.0) * step_size as f64 * scale * 0.5;
                    noise.set(x + half, y, horizontal);
                    noise.set(x, y + half, vertical);
                }
            }
            step_size /= 2;
            scale *= scale_modifier + 0.8;
            scale_modifier *= 0.3;
        }
        noise
    }

    fn sample(&self, x: i32, y: i32) -> f64 {
        let x = x.rem_euclid(self.width as i32) as usize;
        let y = y.rem_euclid(self.height as i32) as usize;
        self.values[x + y * self.width]
    }

    fn set(&mut self, x: i32, y: i32, value: f64) {
        let x = x.rem_euclid(self.width as i32) as usize;
        let y = y.rem_euclid(self.height as i32) as usize;
        self.values[x + y * self.width] = value;
    }
}

pub fn surface(width: usize, height: usize, seed: i64) -> Vec<Tile> {
    surface_with_spec(width, height, seed, WorldSpec::default())
}

pub fn surface_with_spec(width: usize, height: usize, seed: i64, spec: WorldSpec) -> Vec<Tile> {
    let mut random = JavaRandom::new(seed);
    for _ in 0..100 {
        let map = create_surface(width, height, spec, &mut random);
        let rock = count(&map, Tile::Rock);
        let sand = count(&map, Tile::Sand);
        let grass = count(&map, Tile::Grass);
        let trees = count(&map, Tile::Tree);
        let stairs = count(&map, Tile::StairsDown);
        if rock >= 100 && sand >= 100 && grass >= 100 && trees >= 100 && stairs >= width / 21 {
            return map;
        }
    }
    // A guard absent from the Java loop: malformed settings can no longer hang startup forever.
    create_surface(width, height, spec, &mut random)
}

pub fn level(width: usize, height: usize, depth: i8, seed: i64, spec: WorldSpec) -> Vec<Tile> {
    match depth {
        1 => sky(width, height, seed),
        0 => surface_with_spec(width, height, seed, spec),
        -3..=-1 => underground(width, height, -depth as usize, seed),
        -4 => dungeon(width, height, seed),
        _ => panic!("invalid Minicraft+ depth {depth}"),
    }
}

fn underground(width: usize, height: usize, depth: usize, seed: i64) -> Vec<Tile> {
    let mut random = JavaRandom::new(seed);
    for _ in 0..100 {
        let map = create_underground(width, height, depth, &mut random);
        let ore = [Tile::IronOre, Tile::GoldOre, Tile::GemOre][depth - 1];
        let enough_stairs = depth == 3 || count(&map, Tile::StairsDown) >= width / 32;
        if count(&map, Tile::Rock) >= 100
            && count(&map, Tile::Dirt) >= 100
            && count(&map, ore) >= 20
            && enough_stairs
        {
            return map;
        }
    }
    create_underground(width, height, depth, &mut random)
}

fn create_underground(
    width: usize,
    height: usize,
    depth: usize,
    random: &mut JavaRandom,
) -> Vec<Tile> {
    let moisture_1 = Noise::new(width, height, 16, random);
    let moisture_2 = Noise::new(width, height, 16, random);
    let moisture_3 = Noise::new(width, height, 16, random);
    let dirt_1 = Noise::new(width, height, 16, random);
    let dirt_2 = Noise::new(width, height, 16, random);
    let dirt_3 = Noise::new(width, height, 16, random);
    let water_1 = Noise::new(width, height, 16, random);
    let water_2 = Noise::new(width, height, 16, random);
    let water_3 = Noise::new(width, height, 16, random);
    let noise_1 = Noise::new(width, height, 32, random);
    let noise_2 = Noise::new(width, height, 32, random);
    let mut map = vec![Tile::Rock; width * height];

    for y in 0..height {
        for x in 0..width {
            let index = x + y * width;
            let mut value = (noise_1.values[index] - noise_2.values[index]).abs() * 3.0 - 2.0;
            let moisture = ((moisture_1.values[index] - moisture_2.values[index]).abs()
                - moisture_3.values[index])
                .abs()
                * 3.0
                - 2.0;
            let dirt = ((dirt_1.values[index] - dirt_2.values[index]).abs() - dirt_3.values[index])
                .abs()
                * 3.0
                - 2.0;
            let water = ((water_1.values[index] - water_2.values[index]).abs()
                - water_3.values[index])
                .abs()
                * 3.0
                - 2.0;
            let x_distance = (x as f64 / (width - 1) as f64 * 2.0 - 1.0).abs();
            let y_distance = (y as f64 / (height - 1) as f64 * 2.0 - 1.0).abs();
            value += 1.0 - x_distance.max(y_distance).powi(8) * 20.0;
            map[index] = if value > -1.0 && water < -1.0 + depth as f64 / 2.0 * 3.0 {
                match depth {
                    1 => Tile::Dirt,
                    2 => Tile::Water,
                    _ => Tile::Lava,
                }
            } else if value > -2.0 && (moisture < -1.7 || dirt < -1.4) {
                Tile::Dirt
            } else {
                Tile::Rock
            };
        }
    }

    let ore = [Tile::IronOre, Tile::GoldOre, Tile::GemOre][depth - 1];
    for _ in 0..width * height / 400 {
        let origin_x = random.next_int(width as i32);
        let origin_y = random.next_int(height as i32);
        for _ in 0..30 {
            let x = origin_x + random.next_int(5) - random.next_int(5);
            let y = origin_y + random.next_int(5) - random.next_int(5);
            if x >= 2 && y >= 2 && x < width as i32 - 2 && y < height as i32 - 2 {
                replace_if(&mut map, width, height, x, y, Tile::Rock, ore);
            }
        }
        for _ in 0..10 {
            let x = origin_x + random.next_int(3) - random.next_int(2);
            let y = origin_y + random.next_int(3) - random.next_int(2);
            if x >= 2 && y >= 2 && x < width as i32 - 2 && y < height as i32 - 2 {
                replace_if(&mut map, width, height, x, y, Tile::Rock, Tile::LapisOre);
            }
        }
    }
    if depth < 3 {
        place_stairs(&mut map, width, height, width / 32, 10, Tile::Rock, random);
    } else {
        place_stairs(&mut map, width, height, 1, 10, Tile::Rock, random);
    }
    map
}

fn sky(width: usize, height: usize, seed: i64) -> Vec<Tile> {
    let mut random = JavaRandom::new(seed);
    for _ in 0..100 {
        let map = create_sky(width, height, &mut random);
        if count(&map, Tile::Cloud) >= 2000 && count(&map, Tile::StairsDown) >= width / 64 {
            return map;
        }
    }
    create_sky(width, height, &mut random)
}

fn create_sky(width: usize, height: usize, random: &mut JavaRandom) -> Vec<Tile> {
    let noise_1 = Noise::new(width, height, 8, random);
    let noise_2 = Noise::new(width, height, 8, random);
    let mut map = vec![Tile::InfiniteFall; width * height];
    for y in 0..height {
        for x in 0..width {
            let index = x + y * width;
            let mut value = -(noise_1.values[index] - noise_2.values[index]).abs() * 3.0;
            let x_distance = (x as f64 / (width - 1) as f64 * 2.0 - 1.0).abs();
            let y_distance = (y as f64 / (height - 1) as f64 * 2.0 - 1.0).abs();
            value -= 0.2;
            value += 1.0 - x_distance.max(y_distance).powi(8) * 20.0;
            map[index] = if value < -0.25 {
                Tile::InfiniteFall
            } else {
                Tile::Cloud
            };
        }
    }
    for _ in 0..width * height / 50 {
        let x = random.next_int(width as i32 - 2) + 1;
        let y = random.next_int(height as i32 - 2) + 1;
        if area_is(&map, width, x, y, Tile::Cloud) {
            map[x as usize + y as usize * width] = Tile::CloudOre;
        }
    }
    place_stairs(&mut map, width, height, width / 64, 1, Tile::Cloud, random);
    map
}

fn dungeon(width: usize, height: usize, seed: i64) -> Vec<Tile> {
    let mut random = JavaRandom::new(seed);
    for _ in 0..100 {
        let map = create_dungeon(width, height, &mut random);
        if count(&map, Tile::ObsidianWall) >= 100
            && count(&map, Tile::ObsidianFloor) + count(&map, Tile::Dirt) >= 100
        {
            return map;
        }
    }
    create_dungeon(width, height, &mut random)
}

fn create_dungeon(width: usize, height: usize, random: &mut JavaRandom) -> Vec<Tile> {
    let noise_1 = Noise::new(width, height, 10, random);
    let noise_2 = Noise::new(width, height, 10, random);
    let mut map = vec![Tile::ObsidianWall; width * height];
    for y in 0..height {
        for x in 0..width {
            let index = x + y * width;
            let mut value = -(noise_1.values[index] - noise_2.values[index]).abs() * 3.0;
            let x_distance = (x as f64 / (width as f64 - 1.1) * 2.0 - 1.0).abs();
            let y_distance = (y as f64 / (height as f64 - 1.1) * 2.0 - 1.0).abs();
            value -= 0.2;
            value += 1.0 - x_distance.max(y_distance).powi(8) * 2.0;
            map[index] = if value < -0.05 {
                Tile::ObsidianWall
            } else if value < -0.03 {
                Tile::Lava
            } else if random.next_int(2) == 1 {
                if random.next_int(2) == 1 {
                    Tile::ObsidianFloor
                } else {
                    Tile::RawObsidian
                }
            } else {
                Tile::Dirt
            };
        }
    }
    map
}

fn place_stairs(
    map: &mut [Tile],
    width: usize,
    height: usize,
    target: usize,
    margin: i32,
    floor: Tile,
    random: &mut JavaRandom,
) {
    let mut placed = 0;
    for _ in 0..width * height {
        let x = random.next_int(width as i32 - margin * 2) + margin;
        let y = random.next_int(height as i32 - margin * 2) + margin;
        if area_is(map, width, x, y, floor)
            && !nearby(map, width, height, x, y, STAIR_RADIUS, Tile::StairsDown)
        {
            map[x as usize + y as usize * width] = Tile::StairsDown;
            placed += 1;
            if placed >= target {
                break;
            }
        }
    }
}

fn area_is(map: &[Tile], width: usize, x: i32, y: i32, tile: Tile) -> bool {
    (y - 1..=y + 1)
        .all(|yy| (x - 1..=x + 1).all(|xx| map[xx as usize + yy as usize * width] == tile))
}

fn create_surface(
    width: usize,
    height: usize,
    spec: WorldSpec,
    random: &mut JavaRandom,
) -> Vec<Tile> {
    let moisture_1 = Noise::new(width, height, 16, random);
    let moisture_2 = Noise::new(width, height, 16, random);
    let moisture_3 = Noise::new(width, height, 16, random);
    let noise_1 = Noise::new(width, height, 32, random);
    let noise_2 = Noise::new(width, height, 32, random);
    let mut map = vec![Tile::Grass; width * height];

    for y in 0..height {
        for x in 0..width {
            let index = x + y * width;
            let mut value = (noise_1.values[index] - noise_2.values[index]).abs() * 3.0 - 2.0;
            let moisture = ((moisture_1.values[index] - moisture_2.values[index]).abs()
                - moisture_3.values[index])
                .abs()
                * 3.0
                - 2.0;
            let x_distance = (x as f64 / (width - 1) as f64 * 2.0 - 1.0).abs();
            let y_distance = (y as f64 / (height - 1) as f64 * 2.0 - 1.0).abs();
            value += 1.0 - x_distance.max(y_distance).powi(8) * 20.0;
            let liquid = if spec.theme == Theme::Hell {
                Tile::Lava
            } else {
                Tile::Water
            };
            map[index] = match spec.terrain {
                TerrainType::Island => {
                    if value < -0.5 {
                        liquid
                    } else if value > 0.5 && moisture < -1.5 {
                        Tile::Rock
                    } else {
                        Tile::Grass
                    }
                }
                TerrainType::Box => {
                    if value < -1.5 {
                        liquid
                    } else if value > 0.5 && moisture < -1.5 {
                        Tile::Rock
                    } else {
                        Tile::Grass
                    }
                }
                TerrainType::Mountain => {
                    if value < -0.4 {
                        Tile::Grass
                    } else if value > 0.5 && moisture < -1.5 {
                        liquid
                    } else {
                        Tile::Rock
                    }
                }
                TerrainType::Irregular => {
                    if value < -0.5 && moisture < -0.5 {
                        liquid
                    } else if value > 0.5 && moisture < -1.5 {
                        Tile::Rock
                    } else {
                        Tile::Grass
                    }
                }
            };
        }
    }

    let sand_divisor = if spec.theme == Theme::Desert {
        200
    } else {
        2800
    };
    for _ in 0..width * height / sand_divisor {
        let origin_x = random.next_int(width as i32);
        let origin_y = random.next_int(height as i32);
        for _ in 0..10 {
            let x = origin_x + random.next_int(21) - 10;
            let y = origin_y + random.next_int(21) - 10;
            for _ in 0..100 {
                let xx = x + random.next_int(5) - random.next_int(5);
                let yy = y + random.next_int(5) - random.next_int(5);
                for adjacent_y in yy - 1..=yy + 1 {
                    for adjacent_x in xx - 1..=xx + 1 {
                        replace_if(
                            &mut map,
                            width,
                            height,
                            adjacent_x,
                            adjacent_y,
                            Tile::Grass,
                            Tile::Sand,
                        );
                    }
                }
            }
        }
    }

    if spec.theme == Theme::Forest {
        add_tree_clusters(&mut map, width, height, width * height / 200, random);
    }
    if spec.theme != Theme::Forest && spec.theme != Theme::Plain {
        add_tree_clusters(&mut map, width, height, width * height / 1200, random);
    }
    if spec.theme == Theme::Plain {
        add_tree_clusters(&mut map, width, height, width * height / 2800, random);
    }
    if spec.theme != Theme::Plain {
        add_tree_clusters(&mut map, width, height, width * height / 400, random);
    }

    for _ in 0..width * height / 400 {
        let x = random.next_int(width as i32);
        let y = random.next_int(height as i32);
        let _flower_variant = random.next_int(4);
        for _ in 0..30 {
            let xx = x + random.next_int(5) - random.next_int(5);
            let yy = y + random.next_int(5) - random.next_int(5);
            if replace_if(&mut map, width, height, xx, yy, Tile::Grass, Tile::Flower) {
                let _rotation = random.next_int(4);
            }
        }
    }

    for _ in 0..width * height / 100 {
        let x = random.next_int(width as i32);
        let y = random.next_int(height as i32);
        replace_if(&mut map, width, height, x, y, Tile::Sand, Tile::Cactus);
    }

    let mut stair_count = 0;
    for _ in 0..width * height / 100 {
        let x = random.next_int(width as i32 - 2) + 1;
        let y = random.next_int(height as i32 - 2) + 1;
        let surrounded = (y - 1..=y + 1).all(|yy| {
            (x - 1..=x + 1).all(|xx| map[xx as usize + yy as usize * width] == Tile::Rock)
        });
        if !surrounded || nearby(&map, width, height, x, y, STAIR_RADIUS, Tile::StairsDown) {
            continue;
        }
        map[x as usize + y as usize * width] = Tile::StairsDown;
        stair_count += 1;
        if stair_count >= width / 21 {
            break;
        }
    }
    map
}

fn add_tree_clusters(
    map: &mut [Tile],
    width: usize,
    height: usize,
    clusters: usize,
    random: &mut JavaRandom,
) {
    for _ in 0..clusters {
        let x = random.next_int(width as i32);
        let y = random.next_int(height as i32);
        for _ in 0..200 {
            let xx = x + random.next_int(15) - random.next_int(15);
            let yy = y + random.next_int(15) - random.next_int(15);
            replace_if(map, width, height, xx, yy, Tile::Grass, Tile::Tree);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn replace_if(
    map: &mut [Tile],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    expected: Tile,
    replacement: Tile,
) -> bool {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return false;
    }
    let tile = &mut map[x as usize + y as usize * width];
    if *tile == expected {
        *tile = replacement;
        true
    } else {
        false
    }
}

fn nearby(
    map: &[Tile],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    radius: i32,
    target: Tile,
) -> bool {
    for yy in (y - radius).max(0)..=(y + radius).min(height as i32 - 1) {
        for xx in (x - radius).max(0)..=(x + radius).min(width as i32 - 1) {
            if map[xx as usize + yy as usize * width] == target {
                return true;
            }
        }
    }
    false
}

fn count(map: &[Tile], target: Tile) -> usize {
    map.iter().filter(|tile| **tile == target).count()
}

#[cfg(test)]
mod tests {
    use super::{level, surface};
    use crate::world::{Theme, Tile, WorldSpec};

    #[test]
    fn surface_contains_required_biomes_and_stairs() {
        let map = surface(128, 128, 0x100);
        for tile in [
            Tile::Water,
            Tile::Grass,
            Tile::Rock,
            Tile::Sand,
            Tile::Tree,
            Tile::StairsDown,
        ] {
            assert!(map.contains(&tile), "generated map lacks {tile:?}");
        }
    }

    #[test]
    fn generation_is_seed_deterministic() {
        assert_eq!(surface(128, 128, 42), surface(128, 128, 42));
    }

    #[test]
    fn all_six_depths_generate_their_signature_tiles() {
        let seed = 0x100;
        let spec = WorldSpec::default();
        let sky = level(128, 128, 1, seed, spec);
        assert!(sky.contains(&Tile::Cloud));
        assert!(sky.contains(&Tile::InfiniteFall));
        assert!(sky.contains(&Tile::StairsDown));

        for (depth, ore) in [(-1, Tile::IronOre), (-2, Tile::GoldOre), (-3, Tile::GemOre)] {
            let cave = level(128, 128, depth, seed, spec);
            assert!(cave.contains(&Tile::Rock));
            assert!(cave.contains(&Tile::Dirt));
            assert!(cave.contains(&ore));
            assert!(cave.contains(&Tile::StairsDown));
        }

        let dungeon = level(128, 128, -4, seed, spec);
        assert!(dungeon.contains(&Tile::ObsidianWall));
        assert!(dungeon.contains(&Tile::ObsidianFloor));
    }

    #[test]
    fn themes_and_terrain_presets_affect_surface_generation() {
        let normal = super::surface_with_spec(128, 128, 99, WorldSpec::default());
        let desert = super::surface_with_spec(
            128,
            128,
            99,
            WorldSpec {
                theme: Theme::Desert,
                ..WorldSpec::default()
            },
        );
        let hell = super::surface_with_spec(
            128,
            128,
            99,
            WorldSpec {
                theme: Theme::Hell,
                ..WorldSpec::default()
            },
        );
        assert_ne!(normal, desert);
        assert!(
            desert.iter().filter(|tile| **tile == Tile::Sand).count()
                > normal.iter().filter(|tile| **tile == Tile::Sand).count()
        );
        assert!(hell.contains(&Tile::Lava));
        assert!(!hell.contains(&Tile::Water));
    }
}
