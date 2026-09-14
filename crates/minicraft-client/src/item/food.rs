//! Java `FoodItem` — feed values and eat stamina cost.
use super::ItemId;

/// Java `FoodItem.staminaCost`.
pub const FOOD_STAMINA_COST: u8 = 2;

/// Java `FoodItem.getAllInstances` feed amounts.
pub const fn food_value(item: ItemId) -> Option<u8> {
    match item {
        ItemId::BakedPotato
        | ItemId::Apple
        | ItemId::RawPork
        | ItemId::RawFish
        | ItemId::RawBeef => Some(1),
        ItemId::Bread => Some(2),
        ItemId::CookedFish | ItemId::CookedPork | ItemId::Steak => Some(3),
        ItemId::GoldenApple => Some(10),
        _ => None,
    }
}

/// Java `FoodItem` instances.
pub const ALL: &[ItemId] = &[
    ItemId::BakedPotato,
    ItemId::Apple,
    ItemId::RawPork,
    ItemId::RawFish,
    ItemId::RawBeef,
    ItemId::Bread,
    ItemId::CookedFish,
    ItemId::CookedPork,
    ItemId::Steak,
    ItemId::GoldenApple,
];

/// Java `FoodItem.interactsWithWorld` is false.
pub const fn interacts_with_world(item: ItemId) -> bool {
    food_value(item).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn food_values_match_the_java_registry() {
        assert_eq!(food_value(ItemId::RawPork), Some(1));
        assert_eq!(food_value(ItemId::Bread), Some(2));
        assert_eq!(food_value(ItemId::CookedPork), Some(3));
        assert_eq!(food_value(ItemId::GoldenApple), Some(10));
        assert_eq!(food_value(ItemId::Coal), None);
    }

    #[test]
    fn food_instances_are_all_registered() {
        assert_eq!(ALL.len(), 10);
        for item in ALL {
            assert!(food_value(*item).is_some());
        }
    }
}
