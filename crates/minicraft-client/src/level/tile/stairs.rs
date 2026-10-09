//! Java `StairsTile` (Stairs Up / Stairs Down).
use super::Tile;

/// Java `StairsTile` constructor: `maySpawn = false`.
pub const MAY_SPAWN: bool = false;

/// Java `StairsTile.render`: cloud underlay on sky depth, else dirt.
pub fn underlay(depth: i8) -> [Option<Tile>; 2] {
    [
        Some(if depth == 1 { Tile::Cloud } else { Tile::Dirt }),
        None,
    ]
}

/// Java `StairsTile.mayPass`: furniture cannot climb stairs.
pub fn may_pass(is_furniture: bool) -> bool {
    !is_furniture
}

/// Placement sites accepted by stair generation (Java stair search).
pub fn is_site(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::Rock
            | Tile::Dirt
            | Tile::Grass
            | Tile::Cloud
            | Tile::ObsidianFloor
            | Tile::RawObsidian
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stairs_underlay_switches_on_sky_depth() {
        assert_eq!(underlay(1), [Some(Tile::Cloud), None]);
        assert_eq!(underlay(0), [Some(Tile::Dirt), None]);
    }

    #[test]
    fn stairs_block_furniture_and_accept_rock_sites() {
        assert!(!may_pass(true));
        assert!(may_pass(false));
        assert!(is_site(Tile::Rock));
        assert!(is_site(Tile::Grass));
        assert!(!is_site(Tile::Water));
        const {
            assert!(!MAY_SPAWN);
        }
    }
}
