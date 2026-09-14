//! Java `Item` — shared identity helpers used across item kinds.
use super::ItemId;

/// Java `Item.canAttack` is false for non-tools.
pub const fn default_can_attack(_item: ItemId) -> bool {
    false
}

/// Java `Item.isDepleted` default.
pub const fn default_is_depleted() -> bool {
    false
}

/// Java `Item.interactsWithWorld` default is true; inventory-only items
/// (food, armor, potions, books, clothing, hearts, summons) return false.
pub fn interacts_with_world(item: ItemId) -> bool {
    !matches!(
        item,
        // FoodItem
        ItemId::BakedPotato
            | ItemId::Apple
            | ItemId::RawPork
            | ItemId::RawFish
            | ItemId::RawBeef
            | ItemId::Bread
            | ItemId::CookedFish
            | ItemId::CookedPork
            | ItemId::Steak
            | ItemId::GoldenApple
            // ArmorItem
            | ItemId::LeatherArmor
            | ItemId::SnakeArmor
            | ItemId::IronArmor
            | ItemId::GoldArmor
            | ItemId::GemArmor
            // PotionItem
            | ItemId::AwkwardPotion
            | ItemId::SpeedPotion
            | ItemId::LightPotion
            | ItemId::SwimPotion
            | ItemId::HastePotion
            | ItemId::LavaPotion
            | ItemId::EnergyPotion
            | ItemId::RegenPotion
            | ItemId::HealthPotion
            | ItemId::EscapePotion
            | ItemId::ShieldPotion
            | ItemId::TimePotion
            // BookItem
            | ItemId::Book
            | ItemId::AntidiousBook
            // ClothingItem
            | ItemId::RedClothes
            | ItemId::BlueClothes
            | ItemId::GreenClothes
            | ItemId::YellowClothes
            | ItemId::BlackClothes
            | ItemId::OrangeClothes
            | ItemId::PurpleClothes
            | ItemId::CyanClothes
            | ItemId::RegularClothes
            // HeartItem
            | ItemId::ObsidianHeart
            // SummonItem
            | ItemId::AirTotem
            | ItemId::ObsidianPoppet
    )
}

/// Java `Item.getDisplayName` prepends a space (HUD alignment).
pub fn display_name_padded(item: ItemId) -> String {
    format!(" {}", item.display_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_only_items_do_not_interact_with_world() {
        assert!(!interacts_with_world(ItemId::Apple));
        assert!(!interacts_with_world(ItemId::HealthPotion));
        assert!(!interacts_with_world(ItemId::Book));
        assert!(interacts_with_world(ItemId::Dirt));
        assert!(interacts_with_world(ItemId::Torch));
        assert_eq!(display_name_padded(ItemId::Wood), " Wood");
    }
}
