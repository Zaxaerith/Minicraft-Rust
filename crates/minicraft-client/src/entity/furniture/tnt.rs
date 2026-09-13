//! Java Tnt: fuse countdown then explode.
use crate::entity::helpers::squared_distance;

/// Java Tnt fuse length (ticks from lit to explode).
pub const FUSE_TICKS: u16 = 120;
/// Java Tnt blast radius (tiles).
pub const BLAST_RADIUS: i32 = 2;
/// Java Tnt player damage when within 32px at detonation.
pub const PLAYER_DAMAGE: u8 = 4;
pub const PLAYER_DAMAGE_RANGE: i32 = 32;

/// Java Tnt.use / interact: light fuse if idle.
pub fn ignite(state: &mut u16) -> bool {
    if *state == 0 {
        *state = FUSE_TICKS;
        true
    } else {
        false
    }
}

/// Java Tnt.tick: countdown fuse. Returns (x, y, radius, player_tnt) when exploding.
pub fn tick_fuse(
    state: &mut u16,
    x: i32,
    y: i32,
    player: Option<(i32, i32)>,
) -> Option<(i32, i32, i32, bool, u8)> {
    if *state == 0 {
        return None;
    }
    *state -= 1;
    if *state != 0 {
        return None;
    }
    let player_damage = if player.is_some_and(|(px, py)| {
        squared_distance(x, y, px, py) <= PLAYER_DAMAGE_RANGE * PLAYER_DAMAGE_RANGE
    }) {
        PLAYER_DAMAGE
    } else {
        0
    };
    Some((x, y, BLAST_RADIUS, true, player_damage))
}
