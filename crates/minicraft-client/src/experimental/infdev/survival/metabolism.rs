//! Normal difficulty, no potions or sleeping. Same persistent Player counters
//! as the fixed world and Infdev Player.java (42ba9192, lines 398-450).
use crate::world::Player;

pub(super) fn reset(p: &mut Player) {
    p.hunger = 10;
    p.hunger_ticks = 400;
    p.hunger_stamina_count = 7;
    p.step_count = 0;
    p.hunger_charge_delay = 0;
    p.hunger_starve_delay = 0;
}

pub(super) fn tick(p: &mut Player, ticks: u64) {
    if p.stamina < 10 {
        p.hunger_ticks = p.hunger_ticks.saturating_sub(1);
        if p.stamina == 0 {
            p.hunger_ticks = p.hunger_ticks.saturating_sub(1);
        }
    }
    if ticks.is_multiple_of(30) {
        p.hunger_ticks = p.hunger_ticks.saturating_sub(1);
    }
    if p.step_count >= 3 {
        p.hunger_ticks = p.hunger_ticks.saturating_sub(1);
        p.step_count = 0;
    }
    if p.hunger_charge_delay > 0 {
        p.hunger_ticks = p.hunger_ticks.saturating_sub(3);
        if p.hunger == 0 {
            p.hunger_ticks = p.hunger_ticks.saturating_sub(1);
        }
    }
    while p.hunger_ticks <= 0 {
        p.hunger_ticks += 400;
        p.hunger_stamina_count = p.hunger_stamina_count.saturating_sub(1);
    }
    while p.hunger_stamina_count <= 0 {
        p.hunger = p.hunger.saturating_sub(1);
        p.hunger_stamina_count += 7;
    }
    if p.health < p.max_health && p.hunger > 5 {
        p.hunger_charge_delay = p.hunger_charge_delay.saturating_add(1);
        let distance = u16::from(12 - p.hunger);
        if p.hunger_charge_delay > 20 * distance * distance {
            p.health = (p.health + 1).min(p.max_health);
            p.hunger_charge_delay = 0;
        }
    } else {
        p.hunger_charge_delay = 0;
    }
    if p.hunger_starve_delay == 0 {
        p.hunger_starve_delay = 120;
    }
    if p.hunger == 0 && p.health > 3 {
        p.hunger_starve_delay -= 1;
        if p.hunger_starve_delay == 0 {
            p.health -= 1;
        }
    }
}
