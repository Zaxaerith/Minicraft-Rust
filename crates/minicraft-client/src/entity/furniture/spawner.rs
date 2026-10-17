//! Java dev2 Spawner countdown, population pressure, light/day and player gates.
use crate::entity::helpers::squared_distance;
use crate::entity::mob::NaturalMob;
use crate::entity::types::SpawnerState;
use crate::level::noise::JavaRandom;
use crate::level::tile::Tile;

pub const INTERVAL_BASE: u32 = 200;
pub const INTERVAL_SPAN: u32 = 301;
pub const PLAYER_RANGE: i32 = 128;

pub fn new_state(level: u8, random: &mut JavaRandom) -> SpawnerState {
    SpawnerState {
        level,
        spawn_ticks: (200 + random.next_int(301)) as u16,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn tick_spawn(
    species: NaturalMob,
    state: &mut SpawnerState,
    x: i32,
    y: i32,
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    player: Option<(i32, i32)>,
    depth: i8,
    day_tick: u32,
    mob_count: usize,
    max_mob_count: usize,
    lit: bool,
    random: &mut JavaRandom,
) -> Option<(NaturalMob, u8, i32, i32)> {
    state.spawn_ticks = state.spawn_ticks.saturating_sub(1);
    if state.spawn_ticks != 0 {
        return None;
    }
    // Java resets the 200..500 timer on every attempt, including failed gates.
    state.spawn_ticks = (200 + random.next_int(301)) as u16;
    if max_mob_count == 0 || mob_count >= max_mob_count {
        return None;
    }
    let chance = 10usize.saturating_mul(mob_count.saturating_mul(mob_count))
        / max_mob_count.saturating_mul(max_mob_count);
    if chance > 0 && random.next_int(chance.min(i32::MAX as usize) as i32) != 0 {
        return None;
    }
    if species.hostile() && ((depth >= 0 && day_tick > 8100 && day_tick < 40500) || lit) {
        return None;
    }
    let (px, py) = player?;
    if squared_distance(x, y, px, py) > PLAYER_RANGE * PLAYER_RANGE {
        return None;
    }
    // getAreaTilePositions: y-major inclusive square, one uniformly chosen tile.
    let (tx, ty) = (x.div_euclid(16), y.div_euclid(16));
    let mut valid = Vec::with_capacity(9);
    for cy in ty - 1..=ty + 1 {
        for cx in tx - 1..=tx + 1 {
            if cx < 0 || cy < 0 || cx >= width as i32 || cy >= height as i32 {
                continue;
            }
            let index = cx as usize + cy as usize * width;
            let tile = tiles[index];
            if tile.solid(data[index])
                || matches!(
                    tile,
                    Tile::Water | Tile::Lava | Tile::Hole | Tile::InfiniteFall
                )
            {
                continue;
            }
            if species.hostile() && tile.light_radius() > 0 {
                continue;
            }
            valid.push((cx * 16, cy * 16));
        }
    }
    if valid.is_empty() {
        return None;
    }
    let (sx, sy) = valid[random.next_int(valid.len() as i32) as usize];
    Some((species, state.level, sx, sy))
}
