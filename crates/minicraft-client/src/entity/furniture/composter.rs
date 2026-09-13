//! Java Composter: food scraps become Fertilizer at 3 points.
use crate::item::ItemId;

/// Points needed before fertilizer is produced.
pub const THRESHOLD: u16 = 3;

/// Java Composter: cooked food is worth 2 points, raw 1.
pub const fn compost_value(item: ItemId) -> u16 {
    match item {
        ItemId::BakedPotato | ItemId::Bread => 2,
        ItemId::Apple
        | ItemId::Potato
        | ItemId::Carrot
        | ItemId::Wheat
        | ItemId::RawPork
        | ItemId::RawBeef
        | ItemId::RawFish => 1,
        _ => 0,
    }
}
