//! Java `FishingRodItem` — rod levels, loot chances, and break formula.
use super::ItemId;

/// Java `FishingRodItem.LEVEL_NAMES`.
pub const LEVEL_NAMES: [&str; 4] = ["Wood", "Iron", "Gold", "Gem"];

/// Java `FishingRodItem.LEVEL_CHANCES` — cumulative thresholds for
/// fish / junk / tools / rare (first bucket subtracts from 100).
pub const LEVEL_CHANCES: [[u8; 4]; 4] = [
    [44, 14, 9, 4],
    [24, 14, 9, 4],
    [59, 49, 9, 4],
    [79, 69, 59, 4],
];

/// Java `FishingRodItem.getChance`.
pub const fn chance(category: usize, level: u8) -> u8 {
    LEVEL_CHANCES[level as usize][category]
}

/// Java rod instances: Wood / Iron / Gold / Gem.
pub const fn fishing_level(item: ItemId) -> Option<u8> {
    match item {
        ItemId::WoodFishingRod => Some(0),
        ItemId::IronFishingRod => Some(1),
        ItemId::GoldFishingRod => Some(2),
        ItemId::GemFishingRod => Some(3),
        _ => None,
    }
}

/// Java `FishingRodItem.isDepleted` break roll:
/// `random.nextInt(100) > 120 - uses + level * 6`.
pub fn breaks_after_use(roll: u8, uses: u16, level: u8) -> bool {
    i32::from(roll) > 120i32 - i32::from(uses) + i32::from(level) * 6
}

/// Java `FishingRodItem.canAttack` is false.
pub fn can_attack(item: ItemId) -> bool {
    fishing_level(item).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rod_levels_match_java() {
        assert_eq!(fishing_level(ItemId::WoodFishingRod), Some(0));
        assert_eq!(fishing_level(ItemId::IronFishingRod), Some(1));
        assert_eq!(fishing_level(ItemId::GoldFishingRod), Some(2));
        assert_eq!(fishing_level(ItemId::GemFishingRod), Some(3));
        assert_eq!(fishing_level(ItemId::Wood), None);
    }

    #[test]
    fn chance_table_matches_java_shape() {
        assert_eq!(chance(0, 0), 44);
        assert_eq!(chance(0, 1), 24);
        assert_eq!(chance(0, 2), 59);
        assert_eq!(chance(0, 3), 79);
        assert_eq!(chance(3, 3), 4);
    }
}
