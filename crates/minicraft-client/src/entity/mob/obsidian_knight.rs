//! Java ObsidianKnight: phase 2 at half health doubles move speed.
use crate::entity::types::Mob;

/// Java ObsidianKnight: phase 0 armored (speed 1), phase 1 broken (speed 2).
pub fn phase_speed(mob: &Mob) -> u8 {
    if mob.species == crate::entity::mob::NaturalMob::ObsidianKnight && mob.phase == 1 {
        2
    } else {
        1
    }
}

/// Java: phase flips to 1 when health ≤ 2500 (half of MaxHealth 5000).
pub fn update_phase(mob: &mut Mob) {
    if mob.species == crate::entity::mob::NaturalMob::ObsidianKnight
        && mob.health <= mob.max_health / 2
        && mob.phase == 0
    {
        mob.phase = 1;
    }
}

/// Java ObsidianKnight phase-2 contact damage is 2 instead of 1.
pub fn contact_damage(mob: &Mob) -> u8 {
    if mob.species == crate::entity::mob::NaturalMob::ObsidianKnight {
        2
    } else {
        1
    }
}
