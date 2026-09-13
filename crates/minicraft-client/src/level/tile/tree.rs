//! Java `TreeTile`.
use super::Tile;

pub const MAX_HEALTH: u16 = 20;

/// Java `TreeTile.connectsToGrass` — true, but TreeTile draws custom canopy
/// pieces and has no sprite connection checker.
pub fn connects_to_grass() -> bool {
    true
}

/// Dispatcher entry: no sprite connection checker on TreeTile.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

/// Java `TreeTile.render`: grass underlay + canopy pieces.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), Some(Tile::Grass)]
}

/// Java `TreeTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Java `TreeTile.tick` — decay stored damage by 1.
pub fn tick_decay(data: u16) -> Option<u16> {
    (data > 0).then(|| data - 1)
}

/// Chance denominator for an apple drop while hurting a tree (Java `nextInt(100)`).
pub const APPLE_DROP_CHANCE: i32 = 100;

/// Java `TreeType` count.
pub const TREE_TYPE_COUNT: usize = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_reports_grass_family_but_has_no_center_checker() {
        assert!(connects_to_grass());
        assert!(!connects(Tile::Grass));
        assert!(!connects(Tile::Sand));
    }

    #[test]
    fn tree_is_solid_and_decays() {
        assert!(!may_pass());
        assert_eq!(tick_decay(5), Some(4));
        assert_eq!(tick_decay(0), None);
        assert_eq!(MAX_HEALTH, 20);
        assert_eq!(TREE_TYPE_COUNT, 7);
    }
}
