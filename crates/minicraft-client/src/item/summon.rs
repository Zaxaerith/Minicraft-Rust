//! Java `SummonItem` — boss summon totems (Air Wizard / Obsidian Knight).
use super::ItemId;

/// Java `SummonItem.mob` targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummonBoss {
    /// Totem of Air — Java level depth 1 (sky).
    AirWizard,
    /// Obsidian Poppet — Java level depth -4 (dungeon center).
    ObsidianKnight,
}

impl SummonBoss {
    /// Java `SummonItem` stamina cost.
    pub const STAMINA_COST: u8 = 2;

    /// Java level depth where the summon is allowed.
    pub const fn required_depth(self) -> i32 {
        match self {
            Self::AirWizard => 1,
            Self::ObsidianKnight => -4,
        }
    }

    pub const fn item(self) -> ItemId {
        match self {
            Self::AirWizard => ItemId::AirTotem,
            Self::ObsidianKnight => ItemId::ObsidianPoppet,
        }
    }
}

/// Java `SummonItem` instances.
pub const ALL: [ItemId; 2] = [ItemId::AirTotem, ItemId::ObsidianPoppet];

pub const fn summon_for_item(item: ItemId) -> Option<SummonBoss> {
    match item {
        ItemId::AirTotem => Some(SummonBoss::AirWizard),
        ItemId::ObsidianPoppet => Some(SummonBoss::ObsidianKnight),
        _ => None,
    }
}

/// Java SummonItem Obsidian Knight center check: 7×7 tiles around level center.
pub fn near_boss_center(player_tx: i32, player_ty: i32, width: i32, height: i32) -> bool {
    let center_x = width / 2;
    let center_y = height / 2;
    (player_tx - center_x).abs() <= 3 && (player_ty - center_y).abs() <= 3
}

/// Java `SummonItem.interactsWithWorld` is false.
pub const fn interacts_with_world(item: ItemId) -> bool {
    summon_for_item(item).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summon_depths_match_java() {
        assert_eq!(SummonBoss::AirWizard.required_depth(), 1);
        assert_eq!(SummonBoss::ObsidianKnight.required_depth(), -4);
        assert_eq!(summon_for_item(ItemId::AirTotem), Some(SummonBoss::AirWizard));
        assert_eq!(
            summon_for_item(ItemId::ObsidianPoppet),
            Some(SummonBoss::ObsidianKnight)
        );
    }

    #[test]
    fn boss_center_window_is_seven_tiles() {
        assert!(near_boss_center(32, 32, 64, 64));
        assert!(near_boss_center(29, 35, 64, 64));
        assert!(!near_boss_center(20, 32, 64, 64));
    }
}
