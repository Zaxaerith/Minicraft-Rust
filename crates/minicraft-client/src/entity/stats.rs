//! Java Mob constructor health values and MobAi.die drop tables.
use crate::item::{ItemId, ItemStack};
use crate::level::noise::JavaRandom;

use super::mob::NaturalMob;

/// Historical Rust base health. New Java instances apply EnemyMob scaling via
/// mob_health_at_level; saved historical max_health is retained on load.
pub fn mob_health(species: NaturalMob) -> u16 {
    match species {
        NaturalMob::Slime => 1,
        NaturalMob::Zombie | NaturalMob::Cow => 5,
        NaturalMob::Skeleton => 6,
        NaturalMob::Snake => 7,
        NaturalMob::Knight => 9,
        NaturalMob::Creeper => 10,
        NaturalMob::Pig | NaturalMob::Sheep => 8,
        NaturalMob::AirWizard => 2_000,
        NaturalMob::ObsidianKnight => 5_000,
    }
}

/// Java MobAi.die / species dropItem tables.
pub fn mob_drops(species: NaturalMob, random: &mut JavaRandom) -> Vec<ItemStack> {
    match species {
        NaturalMob::AirWizard => vec![ItemStack::new(
            ItemId::CloudOre,
            (random.next_int(6) + 5) as u16,
        )],
        NaturalMob::ObsidianKnight => vec![
            ItemStack::new(ItemId::Shard, (random.next_int(11) + 15) as u16),
            ItemStack::new(ItemId::ObsidianHeart, 1),
        ],
        _ => {
            let (item, maximum) = match species {
                NaturalMob::Slime => (ItemId::Slime, 4),
                NaturalMob::Zombie => (ItemId::Cloth, 3),
                NaturalMob::Creeper => (ItemId::Gunpowder, 3),
                NaturalMob::Skeleton => (ItemId::Bone, 2),
                NaturalMob::Snake => (ItemId::Scale, 2),
                NaturalMob::Knight => (ItemId::Shard, 3),
                NaturalMob::Cow => (ItemId::RawBeef, 3),
                NaturalMob::Pig => (ItemId::RawPork, 3),
                NaturalMob::Sheep => (ItemId::Wool, 3),
                NaturalMob::AirWizard | NaturalMob::ObsidianKnight => unreachable!(),
            };
            vec![ItemStack::new(item, (random.next_int(maximum) + 1) as u16)]
        }
    }
}

/// Java EnemyMob / PassiveMob constructor health; bosses have fixed maxima.
pub fn mob_health_at_level(species: NaturalMob, level: u8, difficulty: u8) -> Option<u16> {
    if !(1..=mob_max_level(species)).contains(&level) || difficulty > 2 {
        return None;
    }
    // PassiveMob: 5 + healthFactor * difficulty (Cow factor 5; default 3).
    // This is distinct from historical Rust max_health stored in old worlds.
    if matches!(
        species,
        NaturalMob::Cow | NaturalMob::Pig | NaturalMob::Sheep
    ) {
        let factor = if species == NaturalMob::Cow { 5 } else { 3 };
        return Some(5 + factor * u16::from(difficulty));
    }
    let base = if species == NaturalMob::Snake && level > 1 {
        8
    } else {
        mob_health(species)
    };
    Some(if ordinary_enemy(species) {
        base * u16::from(level).pow(2) * (1 << difficulty)
    } else {
        base
    })
}

pub const fn ordinary_enemy(species: NaturalMob) -> bool {
    matches!(
        species,
        NaturalMob::Slime
            | NaturalMob::Zombie
            | NaturalMob::Skeleton
            | NaturalMob::Creeper
            | NaturalMob::Snake
            | NaturalMob::Knight
    )
}

pub const fn mob_max_level(species: NaturalMob) -> u8 {
    if ordinary_enemy(species) { 4 } else { 1 }
}

/// Java overrides in Creeper/Snake, otherwise EnemyMob.touchedBy.
pub fn mob_contact_damage(species: NaturalMob, level: u8, difficulty: u8) -> u8 {
    if !species.hostile() {
        return 0;
    }
    match species {
        NaturalMob::Creeper => 1,
        NaturalMob::ObsidianKnight => 2,
        NaturalMob::Snake => level + difficulty.min(2),
        _ => level * if difficulty == 2 { 2 } else { 1 },
    }
}

/// Java normal-enemy drop tables; level affects only Snake/Knight key chance.
/// The random stream is supplied by the host; Java Save does not persist it.
pub fn mob_drops_at_level(
    species: NaturalMob,
    level: u8,
    difficulty: u8,
    score_mode: bool,
    random: &mut JavaRandom,
) -> Vec<ItemStack> {
    let difficulty = difficulty.min(2);
    let mut drops = Vec::new();
    let mut add = |item, count| {
        if count > 0 {
            drops.push(ItemStack::new(item, count));
        }
    };
    match species {
        NaturalMob::Slime | NaturalMob::Creeper => {
            let max = if score_mode && species == NaturalMob::Slime {
                2
            } else {
                4 - i32::from(difficulty)
            };
            add(
                if species == NaturalMob::Slime {
                    ItemId::Slime
                } else {
                    ItemId::Gunpowder
                },
                (random.next_int(max) + 1) as u16,
            );
        }
        NaturalMob::Zombie => {
            let (min, max) = match difficulty {
                0 => (2, 4),
                1 => (1, 3),
                _ => (1, 2),
            };
            add(ItemId::Cloth, (min + random.next_int(max - min + 1)) as u16);
            if random.next_int(60) == 2 {
                add(ItemId::IronIngot, 1);
            }
            if random.next_int(40) == 19 {
                add(
                    [
                        ItemId::GreenClothes,
                        ItemId::RedClothes,
                        ItemId::BlueClothes,
                    ][random.next_int(3) as usize],
                    1,
                );
            }
            if random.next_int(100) < 4 {
                add(ItemId::Potato, 1);
            }
        }
        NaturalMob::Skeleton => {
            let count = (random.next_int(3 - i32::from(difficulty)) + 1) as u16;
            let book_count = random.next_int(1) + 1;
            let roll = random.next_int(if difficulty == 2 { 30 } else { 20 });
            if roll <= [13, 18, 28][difficulty as usize] {
                add(ItemId::Bone, count);
                add(ItemId::Arrow, count);
            } else if difficulty == 0 && roll >= 19 {
                add(ItemId::Arrow, 10);
            } else {
                add(ItemId::AntidiousBook, book_count as u16);
                add(ItemId::Arrow, book_count as u16);
            }
        }
        NaturalMob::Snake | NaturalMob::Knight => {
            let (item, min, max) = if species == NaturalMob::Snake {
                (
                    ItemId::Scale,
                    i32::from(difficulty == 2),
                    i32::from(difficulty == 2) + 1,
                )
            } else if difficulty == 0 {
                (ItemId::Shard, 1, 3)
            } else {
                (ItemId::Shard, 0, 2)
            };
            add(item, (random.next_int(max - min + 1) + min) as u16);
            let bound = 24 / i32::from(level.clamp(1, 4)) / (i32::from(difficulty) + 1);
            if random.next_int(bound) == 0 {
                add(ItemId::Key, (random.next_int(1) + 1) as u16);
            }
        }
        _ => return mob_drops(species, random),
    }
    drops
}
