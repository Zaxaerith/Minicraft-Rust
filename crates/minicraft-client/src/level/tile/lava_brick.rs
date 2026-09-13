//! Java `LavaBrickTile`.
use super::Tile;

/// Java `LavaBrickTile.mayPass`: wool-walkable entities only.
pub fn may_pass(can_wool: bool) -> bool {
    can_wool
}

/// Java `LavaBrickTile.bumpedInto` damage.
pub const BUMP_DAMAGE: u8 = 3;

/// Java `LavaBrickTile` has no connection checker.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_brick_may_pass_and_bump() {
        assert!(may_pass(true));
        assert!(!may_pass(false));
        assert_eq!(BUMP_DAMAGE, 3);
        assert!(!connects(Tile::Lava));
    }
}
