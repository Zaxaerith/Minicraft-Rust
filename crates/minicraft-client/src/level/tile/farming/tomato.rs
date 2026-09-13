//! Java `farming.TomatoTile`.
use super::crop::MAX_AGE;
use crate::level::tile::Tile;

pub const SEED: &str = "tomato seeds";
/// Tomato renders 4 stage sprites.
pub const STAGE_SPRITES: usize = 4;

pub fn sprite_stage(age: u16) -> usize {
    ((age as f32 / MAX_AGE as f32) * 3.0) as usize
}

pub fn tile() -> Tile {
    Tile::Tomato
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tomato_stage_mapping() {
        assert_eq!(sprite_stage(0), 0);
        assert_eq!(sprite_stage(MAX_AGE), 3);
        assert_eq!(STAGE_SPRITES, 4);
        assert_eq!(tile(), Tile::Tomato);
    }
}
