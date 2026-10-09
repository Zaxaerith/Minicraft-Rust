//! Java `PathTile`.
use super::Tile;

pub const MAY_SPAWN: bool = true;

/// Java `PathTile.connectsToGrass` — true, but PathTile has no sprite checker.
pub fn connects_to_grass() -> bool {
    true
}

/// Dispatcher entry: no sprite connection checker on PathTile.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

/// Path sits on dirt (no grass underlay in current renderer).
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), Some(Tile::Grass)]
}

/// Path broken with a shovel becomes hole and drops dirt.
pub fn broken_tile() -> Tile {
    Tile::Hole
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_reports_grass_family_but_has_no_center_checker() {
        assert!(connects_to_grass());
        assert!(!connects(Tile::Grass));
        assert!(!connects(Tile::Sand));
        const {
            assert!(MAY_SPAWN);
        }
        assert_eq!(broken_tile(), Tile::Hole);
    }
}
