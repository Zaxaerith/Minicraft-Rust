//! Java `GrassTile`.
use super::Tile;

pub const MAY_SPAWN: bool = true;

/// Java `GrassTile.connectsToGrass` — sprite checks `neighbor.connectsToGrass`.
pub fn connects(neighbor: Tile) -> bool {
    matches!(
        neighbor,
        Tile::Grass | Tile::Flower | Tile::Tree | Tile::TreeSapling | Tile::Path
    )
}

/// Java `GrassTile.render`: dirt underlay + grass sprite.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), None]
}

/// Chance denominator for grass spreading onto dirt (Java `random.nextInt(40)`).
pub const SPREAD_CHANCE: i32 = 40;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grass_connects_to_dirt_surface_family() {
        assert!(connects(Tile::Grass));
        assert!(connects(Tile::Flower));
        assert!(connects(Tile::Tree));
        assert!(connects(Tile::TreeSapling));
        assert!(connects(Tile::Path));
        assert!(!connects(Tile::Dirt));
        assert!(!connects(Tile::Water));
    }

    #[test]
    fn grass_underlay_is_dirt() {
        assert_eq!(underlay(), [Some(Tile::Dirt), None]);
        assert!(MAY_SPAWN);
    }
}
