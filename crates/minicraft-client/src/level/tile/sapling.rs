//! Java `SaplingTile` (tree sapling on grass, cactus sapling on sand).
use super::Tile;

pub const MAY_SPAWN: bool = true;

/// Java tree sapling: `onType = Grass` → `connectsToGrass` true-family.
/// Sapling sprites have no connection checker, so center-tile joins are false.
pub fn tree_connects_to_grass() -> bool {
    true
}

pub fn tree_connects(_neighbor: Tile) -> bool {
    false
}

/// Java cactus sapling: `onType = Sand` → `connectsToSand`.
pub fn cactus_connects_to_sand() -> bool {
    true
}

pub fn cactus_connects(_neighbor: Tile) -> bool {
    false
}

/// Tree sapling underlay: grass surface.
pub fn tree_underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), Some(Tile::Grass)]
}

/// Cactus sapling underlay: sand surface.
pub fn cactus_underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), Some(Tile::Sand)]
}

/// Java `SaplingTile.tick` — age > 110 grows (entity occupancy checked by caller).
pub const GROW_AGE: u16 = 110;

/// Next age after one tick, or the grown tile when age exceeds the threshold.
pub fn tick_age(age: u16) -> SaplingTick {
    let next = age.saturating_add(1);
    if next > GROW_AGE {
        SaplingTick::ReadyToGrow
    } else {
        SaplingTick::Age(next)
    }
}

pub enum SaplingTick {
    Age(u16),
    ReadyToGrow,
}

/// Tree sapling grows into `Tree`; cactus sapling into `Cactus`.
pub fn grown_tile(is_tree_sapling: bool) -> Tile {
    if is_tree_sapling {
        Tile::Tree
    } else {
        Tile::Cactus
    }
}

/// Java `SaplingTile.hurt` — broken sapling reverts to its `onType`.
pub fn on_type(is_tree_sapling: bool) -> Tile {
    if is_tree_sapling {
        Tile::Grass
    } else {
        Tile::Sand
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sapling_grows_at_age_threshold() {
        assert!(matches!(tick_age(GROW_AGE), SaplingTick::ReadyToGrow));
        assert!(matches!(tick_age(10), SaplingTick::Age(11)));
        assert_eq!(grown_tile(true), Tile::Tree);
        assert_eq!(grown_tile(false), Tile::Cactus);
    }

    #[test]
    fn sapling_on_types_and_underlays() {
        assert_eq!(on_type(true), Tile::Grass);
        assert_eq!(on_type(false), Tile::Sand);
        assert_eq!(tree_underlay(), [Some(Tile::Dirt), Some(Tile::Grass)]);
        assert_eq!(cactus_underlay(), [Some(Tile::Dirt), Some(Tile::Sand)]);
        assert!(tree_connects_to_grass());
        assert!(cactus_connects_to_sand());
        assert!(!tree_connects(Tile::Path));
        assert!(!cactus_connects(Tile::Lava));
        const {
            assert!(MAY_SPAWN);
        }
    }
}
