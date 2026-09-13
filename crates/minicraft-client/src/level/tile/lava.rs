//! Java `LavaTile`.
use super::Tile;

pub const LIGHT_RADIUS: i32 = 6;

/// Java `LavaTile.connectsToSand`.
pub fn connects_to_sand() -> bool {
    true
}

/// Java `LavaTile.connectsToFluid`.
pub fn connects_to_fluid() -> bool {
    true
}

/// Appearance joins fluid-connecting neighbors (sprite checker is fluid-only).
pub fn connects(neighbor: Tile) -> bool {
    matches!(
        neighbor,
        Tile::Water | Tile::Lava | Tile::Hole | Tile::Exploded
    )
}

/// Java `LavaTile.render`: dirt underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), None]
}

/// Java `LavaTile.mayPass`: only swimmers.
pub fn may_pass(can_swim: bool) -> bool {
    can_swim
}

/// Java `LavaTile.tick` — lava fills adjacent holes.
pub fn tick_neighbor(neighbor: Tile) -> Option<Tile> {
    (neighbor == Tile::Hole).then_some(Tile::Lava)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_light_and_fluid_connects() {
        assert_eq!(LIGHT_RADIUS, 6);
        assert!(connects(Tile::Water));
        assert!(connects_to_sand());
        assert!(connects_to_fluid());
        assert!(!connects(Tile::Sand));
    }

    #[test]
    fn lava_fills_holes() {
        assert_eq!(tick_neighbor(Tile::Hole), Some(Tile::Lava));
        assert_eq!(tick_neighbor(Tile::Water), None);
    }
}
