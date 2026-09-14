//! Java `ClothingItem` — shirt colors and clothing swap table.
use super::ItemId;

/// Java `ClothingItem.playerCol` values (Minicraft packed colors).
/// Each tuple is `(ItemId, Color.get(...))` from `ClothingItem.getAllInstances`.
pub const ALL: [(ItemId, u32); 9] = [
    (ItemId::RedClothes, 0x01_CC_00_00),
    (ItemId::BlueClothes, 0x01_00_00_CC),
    (ItemId::GreenClothes, 0x01_00_CC_00),
    (ItemId::YellowClothes, 0x01_CC_CC_00),
    (ItemId::BlackClothes, 0x01_33_33_33),
    (ItemId::OrangeClothes, 0x01_FF_66_00),
    (ItemId::PurpleClothes, 0x01_66_00_99),
    (ItemId::CyanClothes, 0x01_00_66_99),
    // Java comment: Dark Green.
    (ItemId::RegularClothes, 0x01_33_33_00),
];

pub const fn is_clothing(item: ItemId) -> bool {
    matches!(
        item,
        ItemId::RedClothes
            | ItemId::BlueClothes
            | ItemId::GreenClothes
            | ItemId::YellowClothes
            | ItemId::BlackClothes
            | ItemId::OrangeClothes
            | ItemId::PurpleClothes
            | ItemId::CyanClothes
            | ItemId::RegularClothes
    )
}

/// Java `ClothingItem` player shirt color for an item.
pub fn player_color(item: ItemId) -> Option<u32> {
    ALL.iter()
        .find(|(id, _)| *id == item)
        .map(|(_, color)| *color)
}

/// Java `ClothingItem` fallback when the previous shirt has no matching item.
pub const FALLBACK: ItemId = ItemId::RegularClothes;

/// Java `ClothingItem.interactsWithWorld` is false.
pub const fn interacts_with_world(item: ItemId) -> bool {
    !is_clothing(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clothing_table_matches_java_instances() {
        assert_eq!(ALL.len(), 9);
        assert_eq!(player_color(ItemId::RedClothes), Some(ALL[0].1));
        assert_eq!(player_color(ItemId::Wood), None);
        assert!(is_clothing(ItemId::RegularClothes));
        assert!(!is_clothing(ItemId::LeatherArmor));
    }
}
