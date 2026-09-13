//! Java `CloudTile`.
use super::Tile;

/// Java sprite connection: anything except infinite fall.
pub fn connects(neighbor: Tile) -> bool {
    neighbor != Tile::InfiniteFall
}

/// Java `CloudTile.mayPass`.
pub fn may_pass() -> bool {
    true
}

/// Java `CloudTile` has no underlay (sky surface).
pub fn underlay() -> [Option<Tile>; 2] {
    [None, None]
}

/// Stamina cost to shovel cloud (Java `payStamina(5)`).
pub const SHOVEL_STAMINA: i32 = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_connects_to_everything_but_infinite_fall() {
        assert!(connects(Tile::Cloud));
        assert!(connects(Tile::Dirt));
        assert!(!connects(Tile::InfiniteFall));
        assert!(may_pass());
        assert_eq!(SHOVEL_STAMINA, 5);
    }
}
