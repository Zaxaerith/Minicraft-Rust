//! Java `PotionItem` — potion use helpers layered on `PotionKind`.
use super::{ItemId, PotionKind};

/// Java `PotionItem.applyPotion` identity table (ItemId ↔ PotionKind).
pub const fn potion_for_item(item: ItemId) -> Option<PotionKind> {
    match item {
        ItemId::AwkwardPotion => Some(PotionKind::Awkward),
        ItemId::SpeedPotion => Some(PotionKind::Speed),
        ItemId::LightPotion => Some(PotionKind::Light),
        ItemId::SwimPotion => Some(PotionKind::Swim),
        ItemId::HastePotion => Some(PotionKind::Haste),
        ItemId::LavaPotion => Some(PotionKind::Lava),
        ItemId::EnergyPotion => Some(PotionKind::Energy),
        ItemId::RegenPotion => Some(PotionKind::Regen),
        ItemId::HealthPotion => Some(PotionKind::Health),
        ItemId::EscapePotion => Some(PotionKind::Escape),
        ItemId::ShieldPotion => Some(PotionKind::Shield),
        ItemId::TimePotion => Some(PotionKind::Time),
        _ => None,
    }
}

/// Java `PotionItem.getAllInstances`.
pub const ALL: [ItemId; 12] = [
    ItemId::AwkwardPotion,
    ItemId::SpeedPotion,
    ItemId::LightPotion,
    ItemId::SwimPotion,
    ItemId::HastePotion,
    ItemId::LavaPotion,
    ItemId::EnergyPotion,
    ItemId::RegenPotion,
    ItemId::HealthPotion,
    ItemId::EscapePotion,
    ItemId::ShieldPotion,
    ItemId::TimePotion,
];

/// Java `PotionItem` returns an empty glass bottle after a successful drink.
pub const BOTTLE_RETURN: ItemId = ItemId::GlassBottle;

/// Java `PotionItem.interactsWithWorld` is false.
pub const fn interacts_with_world(item: ItemId) -> bool {
    potion_for_item(item).is_none()
}

/// Java `PotionType.toggleEffect` default: usable only when duration > 0,
/// except Health (instant heal) and Escape (level change) which override.
pub const fn can_apply(kind: PotionKind) -> bool {
    match kind {
        PotionKind::Awkward => false,
        PotionKind::Health | PotionKind::Escape => true,
        other => other.duration() > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potion_items_match_java_counts() {
        assert_eq!(ALL.len(), 12);
        assert_eq!(PotionKind::ALL.len(), 12);
        for item in ALL {
            assert!(potion_for_item(item).is_some());
        }
        assert_eq!(potion_for_item(ItemId::Wood), None);
        assert!(!can_apply(PotionKind::Awkward));
        assert!(can_apply(PotionKind::Health));
        assert!(can_apply(PotionKind::Speed));
    }
}
