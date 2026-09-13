//! Java `ExplodedTile`.
use super::Tile;

/// Java sprite connection: only other exploded tiles.
pub fn connects(neighbor: Tile) -> bool {
    neighbor == Tile::Exploded
}

/// Java `ExplodedTile.mayPass`.
pub fn may_pass() -> bool {
    true
}

/// Java `ExplodedTile.connectsToFluid` / `connectsToSand`.
pub fn connects_to_fluid() -> bool {
    true
}

pub fn connects_to_sand() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exploded_connects_only_to_exploded() {
        assert!(connects(Tile::Exploded));
        assert!(!connects(Tile::Water));
        assert!(may_pass());
        assert!(connects_to_fluid());
        assert!(connects_to_sand());
    }
}
