//! Java `HeartItem` — Obsidian Heart permanent health increase.
use super::ItemId;

/// Java `HeartItem` health gain.
pub const HEALTH_GAIN: u8 = 5;

/// Java `HeartItem.staminaCost`.
pub const STAMINA_COST: u8 = 7;

/// Java `HeartItem` instance.
pub const ALL: [ItemId; 1] = [ItemId::ObsidianHeart];

pub fn is_heart(item: ItemId) -> bool {
    item == ItemId::ObsidianHeart
}

/// Java `HeartItem.interactsWithWorld` is false.
pub fn interacts_with_world(item: ItemId) -> bool {
    !is_heart(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heart_constants_match_java() {
        assert_eq!(HEALTH_GAIN, 5);
        assert_eq!(STAMINA_COST, 7);
        assert!(is_heart(ItemId::ObsidianHeart));
        assert!(!is_heart(ItemId::GoldenApple));
    }
}
