//! Java `WateringCanItem` — capacity and fill/water helpers.
use super::ItemId;

/// Java `WateringCanItem.CAPACITY`.
pub const CAPACITY: u16 = 1_800;

/// Java `WateringCanItem` asset keys.
pub const SPRITE_EMPTY: &str = "watering_can";
pub const SPRITE_FILLED: &str = "watering_can_filled";

/// Filled sprite is used while content > 0 (Java `updateSprite`).
pub const fn sprite_for(content: u16) -> &'static str {
    if content > 0 {
        SPRITE_FILLED
    } else {
        SPRITE_EMPTY
    }
}

/// Java watering can instance name.
pub fn is_watering_can(item: ItemId) -> bool {
    item == ItemId::WateringCan
}

/// Java `WateringCanItem` crop fertilization cap (`fertilization < 150`).
pub const MAX_FERTILIZATION: u16 = 150;

/// Java `CropTile.getFertilization` bits live in `data >> 7`. One watering
/// tick adds 1, capped at `MAX_FERTILIZATION`, and preserves the low 7
/// growth bits (same layout as `world/tile_attack.rs`).
pub fn water_crop_data(data: u16) -> u16 {
    let fertilization = (data >> 7).min(MAX_FERTILIZATION);
    (data & 0x7f) | ((fertilization + 1).min(MAX_FERTILIZATION) << 7)
}

/// Java `WateringCanItem.getData` appends content for save/load.
pub fn save_suffix(content: u16) -> String {
    format!("_{content}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_and_sprite_match_java() {
        assert_eq!(CAPACITY, 1_800);
        assert_eq!(sprite_for(0), SPRITE_EMPTY);
        assert_eq!(sprite_for(1), SPRITE_FILLED);
        assert!(is_watering_can(ItemId::WateringCan));
    }

    #[test]
    fn crop_fertilization_is_capped() {
        assert_eq!(water_crop_data(0), 1 << 7);
        assert_eq!(water_crop_data(150 << 7), 150 << 7);
    }
}
