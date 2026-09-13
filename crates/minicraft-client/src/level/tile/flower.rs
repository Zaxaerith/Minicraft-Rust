//! Java `FlowerTile`.
use super::Tile;

pub const MAY_SPAWN: bool = true;

/// Java `FlowerTile.connectsToGrass` — true, but FlowerTile's sprite has no
/// connection checker, so flowers do not auto-tile as the center tile.
pub fn connects_to_grass() -> bool {
    true
}

/// Dispatcher entry: no sprite connection checker on FlowerTile.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

/// Java `FlowerTile.render`: grass underlay + flower sprite.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), Some(Tile::Grass)]
}

/// Chance denominator for flower spreading grass onto dirt (Java `nextInt(30)`).
pub const SPREAD_CHANCE: i32 = 30;

/// Number of `FlowerVariant` values registered in Java.
pub const VARIANT_COUNT: usize = 19;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flower_reports_grass_family_but_has_no_center_checker() {
        assert!(connects_to_grass());
        assert!(!connects(Tile::Grass));
        assert!(!connects(Tile::Path));
    }

    #[test]
    fn flower_renders_over_grass() {
        assert_eq!(underlay(), [Some(Tile::Dirt), Some(Tile::Grass)]);
        assert_eq!(VARIANT_COUNT, 19);
    }
}
