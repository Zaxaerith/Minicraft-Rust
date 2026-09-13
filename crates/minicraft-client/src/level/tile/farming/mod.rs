//! Java `farming/*` — farmland, crops, and berry plants.
pub mod carrot;
pub mod crop;
pub mod farm;
pub mod heavenly_berries;
pub mod hellish_berries;
pub mod potato;
pub mod tomato;
pub mod wheat;

use super::Tile;

/// Shared crop identity helper (Java `CropTile` subclasses).
pub fn is_crop(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::Wheat
            | Tile::Potato
            | Tile::Tomato
            | Tile::Carrot
            | Tile::HeavenlyBerries
            | Tile::HellishBerries
    )
}

/// Java `FarmTile` family membership used by crop growth adjacency.
pub fn is_farm(tile: Tile) -> bool {
    tile == Tile::Farmland || is_crop(tile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn farm_family_membership() {
        assert!(is_farm(Tile::Farmland));
        assert!(is_farm(Tile::Wheat));
        assert!(is_crop(Tile::HellishBerries));
        assert!(!is_crop(Tile::Farmland));
        assert!(!is_farm(Tile::Grass));
    }
}
