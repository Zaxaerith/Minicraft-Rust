//! Java Skeleton: arrow volley when player within 100px.
use crate::entity::projectile::ProjectileKind;
use crate::entity::types::Mob;

/// Java Skeleton.tick: artime counts down; fire Arrow at player within detect range.
/// `arrowtime = 500 / (lvl + 5)`; lvl=1 → 83.
pub fn tick_attack(
    mob: &mut Mob,
    _x: i32,
    _y: i32,
    _player_x: i32,
    _player_y: i32,
    distance: i32,
    queued_projectiles: &mut Vec<(ProjectileKind, i32, i32, i32, i32, u8)>,
) {
    mob.attack_delay = mob.attack_delay.saturating_sub(1);
    if distance < 100 * 100 && mob.attack_delay == 0 {
        queued_projectiles.push((ProjectileKind::Arrow, _x, _y, _player_x, _player_y, 1));
        mob.attack_delay = 83;
    }
}
