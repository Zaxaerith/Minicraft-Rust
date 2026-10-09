//! Java Spawner: timed mob spawn near player with light/day/hostile skips.
use crate::entity::helpers::{can_stand, squared_distance};
use crate::entity::mob::NaturalMob;
use crate::level::tile::Tile;

/// Java Spawner random interval base (200–500 via age%301).
pub const INTERVAL_BASE: u32 = 200;
pub const INTERVAL_SPAN: u32 = 301;
/// Java: only spawn when player within 128 entity-pixels.
pub const PLAYER_RANGE: i32 = 128;

/// Candidate offsets tried around the spawner before falling back.
pub const SPAWN_OFFSETS: [(i32, i32); 6] =
    [(16, 0), (-16, 0), (0, 16), (0, -16), (16, 16), (-16, -16)];

/// Java Spawner.trySpawn: returns (species, x, y) when a spawn should occur.
#[allow(clippy::too_many_arguments)]
pub fn try_spawn(
    species: NaturalMob,
    x: i32,
    y: i32,
    age: u32,
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    player: Option<(i32, i32)>,
    level_depth: i8,
) -> Option<(NaturalMob, i32, i32)> {
    let interval = INTERVAL_BASE + (age % INTERVAL_SPAN);
    if !age.is_multiple_of(interval) {
        return None;
    }
    let hostile = species.hostile();
    let lit = {
        let tx = x.div_euclid(16);
        let ty = y.div_euclid(16);
        if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
            false
        } else {
            tiles[tx as usize + ty as usize * width].light_radius() > 0
        }
    };
    // Java Spawner.trySpawn: surface/day skip + light skip for hostiles.
    let surface_day = level_depth >= 0 && hostile && lit;
    let near_player = player
        .is_some_and(|(px, py)| squared_distance(x, y, px, py) <= PLAYER_RANGE * PLAYER_RANGE);
    if !near_player || (hostile && lit) || (hostile && surface_day) {
        return None;
    }
    let mut spawn_x = x + 16;
    let mut spawn_y = y;
    for (dx, dy) in SPAWN_OFFSETS {
        let candidate_x = x + dx;
        let candidate_y = y + dy;
        if can_stand(tiles, data, width, height, candidate_x, candidate_y) {
            spawn_x = candidate_x;
            spawn_y = candidate_y;
            break;
        }
    }
    Some((species, spawn_x, spawn_y))
}
