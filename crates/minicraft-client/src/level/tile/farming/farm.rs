//! Java `farming.FarmTile` — moisture farmland.
#[cfg(test)]
use crate::level::tile::Tile;

/// Moisture nibble mask (3 bits).
pub const MOISTURE_MASK: u16 = 0b111;
pub const MAX_MOISTURE: u16 = 7;

/// Java `FarmTile.tick` hydration radius.
pub const WATER_RADIUS: usize = 4;

/// Moisture after a hydrating tick (Java stores the pre-increment value).
pub fn hydrate(data: u16) -> u16 {
    let moisture = data & MOISTURE_MASK;
    if moisture < MAX_MOISTURE {
        (data & !MOISTURE_MASK) | moisture
    } else {
        data
    }
}

/// Moisture after a drying tick (Java stores the pre-decrement value).
pub fn dry(data: u16) -> u16 {
    let moisture = data & MOISTURE_MASK;
    if moisture > 0 {
        (data & !MOISTURE_MASK) | moisture
    } else {
        data
    }
}

/// Dry farmland with zero moisture reverts to dirt.
pub fn reverts_to_dirt(data: u16) -> bool {
    (data & MOISTURE_MASK) == 0
}

/// Moist farmland sprite is selected when moisture > 0.
pub fn is_moist(data: u16) -> bool {
    (data & MOISTURE_MASK) > 0
}

/// Watering can on farmland maxes moisture.
pub fn water_fill() -> u16 {
    MAX_MOISTURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn farm_moisture_bit_layout() {
        assert_eq!(hydrate(0), 0);
        assert_eq!(hydrate(3), 3);
        assert_eq!(dry(3), 3);
        assert_eq!(dry(0), 0);
        assert!(reverts_to_dirt(0));
        assert!(!reverts_to_dirt(1));
        assert!(is_moist(7));
        assert!(!is_moist(0));
        assert_eq!(water_fill(), 7);
        assert_eq!(MOISTURE_MASK, 0b111);
        let _ = Tile::Farmland;
    }
}
