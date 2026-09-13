//! Java `farming.WheatTile`.
use super::crop::MAX_AGE;
use crate::level::tile::Tile;

pub const SEED: &str = "wheat seeds";
/// Wheat renders 6 stage sprites mapped from age 0..=7.
pub const STAGE_SPRITES: usize = 6;

/// Java `WheatTile.render` stage index: `(age / maxAge) * 5`.
pub fn sprite_stage(age: u16) -> usize {
    ((age as f32 / MAX_AGE as f32) * 5.0) as usize
}

pub fn tile() -> Tile {
    Tile::Wheat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheat_stage_mapping() {
        assert_eq!(sprite_stage(0), 0);
        assert_eq!(sprite_stage(MAX_AGE), 5);
        assert_eq!(STAGE_SPRITES, 6);
        assert_eq!(tile(), Tile::Wheat);
        assert_eq!(SEED, "wheat seeds");
    }
}
