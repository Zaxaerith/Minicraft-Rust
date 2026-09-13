//! Java `farming.CropTile` — shared stage/fertilizer bit layout for crops.
use super::farm::MOISTURE_MASK;
use crate::level::tile::Tile;

/// Stage occupies bits 3..=5 (3 bits, max stage 7).
pub const MAX_AGE: u16 = 0b111;
pub const STAGE_SHIFT: u32 = 3;
/// Fertilizer starts at bit 7 (after moisture + stage).
pub const FERTILIZATION_SHIFT: u32 = 7;
pub const MAX_FERTILIZATION: u16 = 511;

pub fn stage(data: u16) -> u16 {
    (data >> STAGE_SHIFT) & MAX_AGE
}

pub fn fertilization(data: u16) -> u16 {
    data >> FERTILIZATION_SHIFT
}

/// Encode moisture/stage/fertilizer back into tile data.
pub fn encode(moisture: u16, stage: u16, fertilization: u16) -> u16 {
    (moisture & MOISTURE_MASK)
        | ((stage & MAX_AGE) << STAGE_SHIFT)
        | ((fertilization & MAX_FERTILIZATION) << FERTILIZATION_SHIFT)
}

/// Java fertilize amounts by current fertilization band.
pub fn fertilizer_amount(current: u16, arcane: bool) -> u16 {
    if arcane {
        300
    } else if current < 100 {
        40
    } else if current < 200 {
        30
    } else if current < 300 {
        25
    } else if current < 400 {
        20
    } else {
        10
    }
}

/// Apply fertilizer with the Java 511 clamp. `amount` is the band amount
/// (see `fertilizer_amount`); `arcane` selects the flat +300 path.
pub fn apply_fertilizer(data: u16, amount: u16, arcane: bool) -> u16 {
    let current = fertilization(data);
    let add = if arcane { 300 } else { amount };
    let next = current.saturating_add(add).min(MAX_FERTILIZATION);
    encode(data & MOISTURE_MASK, stage(data), next)
}

/// Base growth points: 4 when moist, 2 when dry (before neighbour modifiers).
pub fn base_growth_points(moisture: u16) -> f64 {
    if moisture > 0 {
        4.0
    } else {
        2.0
    }
}

/// Harvest drops the crop when mature (Java `age == maxAge`).
/// Returns the minimum produce count; callers add RNG (`nextInt(3)+2`).
pub fn mature_produce_min() -> u16 {
    2
}

/// Whether harvesting at `age` yields the crop item.
pub fn is_mature(age: u16) -> bool {
    age == MAX_AGE
}

/// Broken crop returns to farmland, preserving moisture.
pub fn broken_tile() -> Tile {
    Tile::Farmland
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_bit_layout_matches_java() {
        // moisture=2, stage=5, fertilization=3
        let data = encode(2, 5, 3);
        assert_eq!(data & 7, 2);
        assert_eq!(stage(data), 5);
        assert_eq!(fertilization(data), 3);
        assert_eq!(MAX_AGE, 7);
    }

    #[test]
    fn fertilizer_bands_match_java() {
        assert_eq!(fertilizer_amount(0, false), 40);
        assert_eq!(fertilizer_amount(100, false), 30);
        assert_eq!(fertilizer_amount(200, false), 25);
        assert_eq!(fertilizer_amount(300, false), 20);
        assert_eq!(fertilizer_amount(400, false), 10);
        assert_eq!(fertilizer_amount(0, true), 300);
        let filled = apply_fertilizer(encode(0, 0, 500), 40, false);
        assert_eq!(fertilization(filled), MAX_FERTILIZATION);
        assert_eq!(base_growth_points(1), 4.0);
        assert_eq!(base_growth_points(0), 2.0);
        assert_eq!(broken_tile(), Tile::Farmland);
    }
}
