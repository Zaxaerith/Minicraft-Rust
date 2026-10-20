//! Pig state uses the shared Mob/Inventory/Tool models; no parallel item IDs.
use super::*;
use crate::{
    entity::{EntityArena, EntityKind, mob::NaturalMob, types::Mob},
    level::noise::JavaRandom,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Birth {
    pub key: Key,
    pub id: u64,
}
impl Birth {
    pub fn validate(&self) -> Result<(), String> {
        if !self.key.valid(true) || self.id == 0 {
            return Err("invalid entity birth identity".into());
        }
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Pig {
    pub mob: Mob,
    pub age: u32,
    pub px: u8,
    pub py: u8,
}
impl Pig {
    pub fn new() -> Self {
        let mut arena = EntityArena::default();
        arena
            .spawn_mob_level(NaturalMob::Pig, 1, 1, false, 0, 0)
            .expect("valid pig");
        let EntityKind::Mob(mob) = arena.entities()[0].kind.clone() else {
            unreachable!()
        };
        Self {
            mob,
            age: 0,
            px: 8,
            py: 8,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let m = &self.mob;
        if m.species != NaturalMob::Pig
            || m.level != 1
            || m.difficulty != 1
            || m.max_health != 8
            || !(1..=8).contains(&m.health)
            || m.hurt_time > 10
            || m.walk_time > 45
            || !(-1..=1).contains(&m.x_move)
            || !(-1..=1).contains(&m.y_move)
            || self.px > 15
            || self.py > 15
            || m.score_mode
            || m.sheared
            || m.sheep_color != 15
            || m.attack_delay != 0
            || m.attack_time != 0
            || m.phase != 0
        {
            return Err("invalid persistent pig state".into());
        }
        Ok(())
    }
    // Original MobAi/PassiveMob order: movement precedes direction reroll;
    // walk duration counts down, but Java does not clear xmov on reaching zero.
    pub fn reroll(&mut self, random: &mut JavaRandom) {
        if random.next_int(40) == 0 {
            if self.mob.x_move == 0 && self.mob.y_move == 0 {
                let _ = random.next_int(5);
            }
            self.mob.walk_time = 45;
            self.mob.x_move = (random.next_int(3) - 1) * random.next_int(2);
            self.mob.y_move = (random.next_int(3) - 1) * random.next_int(2);
        }
        self.mob.walk_time = self.mob.walk_time.saturating_sub(1);
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemDrop {
    pub loot: survival::Loot,
    pub delay: u16,
}

/// Original Pig.die Normal range1..2, not the historical generic Rust1..3.
pub(crate) fn pig_drop(random: &mut JavaRandom) -> crate::item::ItemStack {
    crate::item::ItemStack::new(
        crate::item::ItemId::RawPork,
        (1 + random.next_int(2)) as u16,
    )
}
pub(crate) fn take_drop(drop: &mut ItemDrop, player: &mut Inventory) -> bool {
    match &mut drop.loot {
        survival::Loot::Stack(s) => {
            s.count = player.add_stack(*s);
            s.count == 0
        }
        survival::Loot::Tool(t) => player.add_tool(*t).is_some(),
    }
}
