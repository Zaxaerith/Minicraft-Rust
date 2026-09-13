//! Java `HoleTile`.
use super::Tile;

/// Java `HoleTile.connectsToSand`.
pub fn connects_to_sand() -> bool {
    true
}

/// Java `HoleTile.connectsToFluid`.
pub fn connects_to_fluid() -> bool {
    true
}

/// Appearance join: hole participates in the fluid/sand groups.
pub fn connects(neighbor: Tile) -> bool {
    matches!(
        neighbor,
        Tile::Water | Tile::Lava | Tile::Hole | Tile::Exploded
    )
}

/// Java `HoleTile.render`: dirt underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), None]
}

/// Java `HoleTile.mayPass`: only swimmers.
pub fn may_pass(can_swim: bool) -> bool {
    can_swim
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hole_connects_to_fluid_group() {
        assert!(connects(Tile::Water));
        assert!(connects(Tile::Lava));
        assert!(connects(Tile::Hole));
        assert!(!connects(Tile::Dirt));
    }

    #[test]
    fn hole_is_swimmable() {
        assert!(may_pass(true));
        assert!(!may_pass(false));
        assert!(connects_to_sand());
        assert!(connects_to_fluid());
    }
}
