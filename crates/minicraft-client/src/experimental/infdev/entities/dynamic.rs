//! Animal state uses the shared Mob/Inventory/Tool models; no parallel item IDs.
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
pub(crate) struct Animal {
    pub mob: Mob,
    pub age: u32,
    pub px: u8,
    pub py: u8,
    #[serde(default, skip_serializing_if = "zero_i8")]
    pub knock_x: i8,
    #[serde(default, skip_serializing_if = "zero_i8")]
    pub knock_y: i8,
    #[serde(default, skip_serializing_if = "zero_u32")]
    pub idle: u32,
}
// Historical PigV1 keeps its serialized Kind::Pig and zero-valued added fields.
pub(crate) type Pig = Animal;
impl Animal {
    pub fn new() -> Self {
        Self::of(NaturalMob::Pig)
    }
    pub fn of(species: NaturalMob) -> Self {
        let mut arena = EntityArena::default();
        arena
            .spawn_mob_level(species, 1, 1, false, 0, 0)
            .expect("valid passive species");
        let EntityKind::Mob(mob) = arena.entities()[0].kind.clone() else {
            unreachable!()
        };
        Self {
            mob,
            age: 0,
            px: 8,
            py: 8,
            knock_x: 0,
            knock_y: 0,
            idle: 0,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let m = &self.mob;
        if !matches!(
            m.species,
            NaturalMob::Pig | NaturalMob::Cow | NaturalMob::Sheep
        ) || m.level != 1
            || m.difficulty != 1
            || m.max_health
                != crate::entity::stats::mob_health_at_level(m.species, 1, 1).unwrap_or(0)
            || !(1..=m.max_health).contains(&m.health)
            || m.hurt_time > 10
            || m.walk_time > 45
            || !(-1..=1).contains(&m.x_move)
            || !(-1..=1).contains(&m.y_move)
            || self.px > 15
            || self.py > 15
            || m.score_mode
            || m.sheep_color > 15
            || (m.species != NaturalMob::Sheep && (m.sheared || m.sheep_color != 15))
            || !(-6..=6).contains(&self.knock_x)
            || !(-6..=6).contains(&self.knock_y)
            || m.attack_delay != 0
            || m.attack_time != 0
            || m.phase != 0
        {
            return Err("invalid persistent passive animal state".into());
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

fn zero_i8(v: &i8) -> bool {
    *v == 0
}
fn zero_u32(v: &u32) -> bool {
    *v == 0
}
pub(crate) fn animal_drops(
    animal: &Animal,
    random: &mut JavaRandom,
) -> Vec<crate::item::ItemStack> {
    use crate::item::{ItemId, ItemStack};
    match animal.mob.species {
        NaturalMob::Pig => vec![pig_drop(random)],
        NaturalMob::Cow => {
            let count = (1 + random.next_int(2)) as u16;
            vec![
                ItemStack::new(ItemId::Leather, count),
                ItemStack::new(ItemId::RawBeef, count),
            ]
        }
        NaturalMob::Sheep => {
            crate::entity::stats::sheep_drops(animal.mob.sheep_color, animal.mob.sheared, 1, random)
        }
        _ => unreachable!("validated passive species"),
    }
}
