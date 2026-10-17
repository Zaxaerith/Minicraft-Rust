//! Java Creeper: fuse on player touch, 60-tick fuse, explode if player in radius.
use crate::entity::types::{Mob, TickOutcome};

/// Java Creeper.MAX_FUSE_TIME / TRIGGER_RADIUS / BLAST_DAMAGE.
pub const MAX_FUSE_TIME: u16 = 60;
pub const TRIGGER_RADIUS: i32 = 64;
pub const BLAST_DAMAGE: i32 = 50;

/// Ticks creeper fuse. Returns true when the creeper should be removed (exploded).
pub fn tick_fuse(
    mob: &mut Mob,
    x: i32,
    y: i32,
    distance: i32,
    player_difficulty: u8,
    outcome: &mut TickOutcome,
) -> bool {
    // Java Creeper: fuse starts on player touch (touchedBy), 60-tick fuse,
    // explode only if a player is still within TRIGGER_RADIUS (64px).
    if distance <= 8 * 8 && mob.attack_time == 0 {
        mob.attack_time = MAX_FUSE_TIME;
    }
    if mob.attack_time == 0 {
        return false;
    }
    mob.attack_time -= 1;
    mob.x_move = 0;
    mob.y_move = 0;
    if mob.attack_time != 0 {
        return false;
    }
    if distance < TRIGGER_RADIUS * TRIGGER_RADIUS {
        // Java: BLAST_DAMAGE * lvl * (1/(dist+1)) + difficulty index.
        let dist_diag = (distance as f64).sqrt();
        let damage = (f64::from(BLAST_DAMAGE) * f64::from(mob.level) * (1.0 / (dist_diag + 1.0))
            + f64::from(player_difficulty)) as u8;
        outcome.player_damage = outcome.player_damage.max(damage.max(1));
        outcome.explosions.push((x, y, i32::from(mob.level), false));
        return true;
    }
    // No player in blast range: defuse (fuseTime=0, fuseLit=false).
    false
}
