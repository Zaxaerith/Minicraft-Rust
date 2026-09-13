//! Java AirWizard: phased spark attack, attackDelay/attackTime timers.
use crate::entity::projectile::ProjectileKind;
use crate::entity::types::Mob;

/// Java AirWizard attack cadence: attackDelay=120 then attackTime=120.
pub const ATTACK_DELAY: u16 = 120;
pub const ATTACK_TIME: u16 = 120;
/// Java: start attack when player within 50 entity-pixels.
pub const ATTACK_RANGE: i32 = 50;

/// Ticks AirWizard / ObsidianKnight shared spark-volley attack.
/// Phase 0 until health ≤ max/2; spark direction rotates every 5 ticks.
pub fn tick_boss_attack(
    mob: &mut Mob,
    x: i32,
    y: i32,
    distance: i32,
    queued_projectiles: &mut Vec<(ProjectileKind, i32, i32, i32, i32, u8)>,
) {
    mob.phase = u8::from(mob.health <= mob.max_health / 2);
    mob.attack_delay = mob.attack_delay.saturating_sub(1);
    if mob.attack_time > 0 {
        mob.attack_time -= 1;
        mob.x_move = 0;
        mob.y_move = 0;
        if mob.attack_time.is_multiple_of(5) {
            let directions = [
                (-2, 0),
                (-1, -1),
                (0, -2),
                (1, -1),
                (2, 0),
                (1, 1),
                (0, 2),
                (-1, 1),
            ];
            let direction = directions[(mob.attack_time as usize / 5 + mob.phase as usize) % 8];
            queued_projectiles.push((
                if mob.species == crate::entity::mob::NaturalMob::ObsidianKnight {
                    ProjectileKind::FireSpark
                } else {
                    ProjectileKind::Spark
                },
                x,
                y,
                x + direction.0,
                y + direction.1,
                1 + mob.phase,
            ));
        }
    } else if mob.attack_delay == 0 && distance < ATTACK_RANGE * ATTACK_RANGE {
        mob.attack_delay = ATTACK_DELAY;
        mob.attack_time = ATTACK_TIME;
    }
}
