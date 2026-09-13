//! Java Mob constructor health values and MobAi.die drop tables.
use crate::item::{ItemId, ItemStack};
use crate::level::noise::JavaRandom;

use super::mob::NaturalMob;

/// Java per-species max health (Mob / EnemyMob / AirWizard / ObsidianKnight constructors).
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
