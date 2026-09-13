//! Java `farming.PotatoTile`.
use super::crop::MAX_AGE;
use crate::level::tile::Tile;

/// Potato has no seed drop (`seed == null`).
pub const SEED: Option<&str> = None;
pub const STAGE_SPRITES: usize = 6;

pub fn sprite_stage(age: u16) -> usize {
    ((age as f32 / MAX_AGE as f32) * 5.0) as usize
}

pub fn tile() -> Tile {
    Tile::Potato
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potato_stage_mapping() {
        assert_eq!(sprite_stage(0), 0);
        assert_eq!(sprite_stage(MAX_AGE), 5);
        assert!(SEED.is_none());
        assert_eq!(tile(), Tile::Potato);
    }
}
