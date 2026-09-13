//! Java `WaterTile`.
use super::Tile;

/// Java `WaterTile.connectsToFluid`.
pub fn connects_to_fluid() -> bool {
    true
}

/// Appearance joins every fluid-connecting neighbor.
pub fn connects(neighbor: Tile) -> bool {
    matches!(
        neighbor,
        Tile::Water | Tile::Lava | Tile::Hole | Tile::Exploded
    )
}

/// Java `WaterTile.render`: dirt underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), None]
}

/// Java `WaterTile.mayPass`: only swimmers.
pub fn may_pass(can_swim: bool) -> bool {
    can_swim
}

/// Java `WaterTile.tick` — water fills adjacent holes and solidifies lava to raw obsidian.
/// Returns the replacement tile for a neighbor at `dx,dy` if any.
pub fn tick_neighbor(neighbor: Tile, dx: i32, dy: i32) -> Option<Tile> {
    if neighbor == Tile::Hole {
        return Some(Tile::Water);
    }
    // Cardinal adjacency only (Java loops x then y on the same axis).
    if (dx == 0 || dy == 0) && neighbor == Tile::Lava {
        return Some(Tile::RawObsidian);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_connects_to_fluid_family() {
        assert!(connects(Tile::Lava));
        assert!(connects(Tile::Hole));
        assert!(connects(Tile::Exploded));
        assert!(!connects(Tile::Sand));
    }

    #[test]
    fn water_tick_fills_hole_and_solidifies_lava() {
        assert_eq!(tick_neighbor(Tile::Hole, 1, 0), Some(Tile::Water));
        assert_eq!(tick_neighbor(Tile::Lava, 1, 0), Some(Tile::RawObsidian));
        assert_eq!(tick_neighbor(Tile::Lava, 0, -1), Some(Tile::RawObsidian));
        // Diagonals are not solidified by the Java loop structure.
        assert_eq!(tick_neighbor(Tile::Lava, 1, 1), None);
        assert_eq!(tick_neighbor(Tile::Grass, 1, 0), None);
    }
}
