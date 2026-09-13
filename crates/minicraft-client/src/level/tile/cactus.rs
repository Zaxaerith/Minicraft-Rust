//! Java `CactusTile`.
use super::Tile;

pub const MAX_HEALTH: u16 = 10;

/// Java `CactusTile.connectsToSand` — true, but CactusTile has no sprite checker.
pub fn connects_to_sand() -> bool {
    true
}

/// Dispatcher entry: no sprite connection checker on CactusTile.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

/// Java `CactusTile.render`: sand underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), Some(Tile::Sand)]
}

/// Java `CactusTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Java `CactusTile.tick` — decay stored damage.
pub fn tick_decay(data: u16) -> Option<u16> {
    (data > 0).then(|| data - 1)
}

/// Java `CactusTile.bumpedInto` damage by difficulty (easy/normal 1, hard 2).
pub fn bump_damage(difficulty: u8) -> u8 {
    if difficulty.min(2) == 2 {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cactus_reports_sand_family_but_has_no_center_checker() {
        assert!(connects_to_sand());
        assert!(!connects(Tile::Sand));
        assert!(!connects(Tile::Grass));
    }

    #[test]
    fn cactus_bump_damage_and_decay() {
        assert_eq!(bump_damage(0), 1);
        assert_eq!(bump_damage(1), 1);
        assert_eq!(bump_damage(2), 2);
        assert_eq!(tick_decay(1), Some(0));
        assert_eq!(tick_decay(0), None);
        assert_eq!(MAX_HEALTH, 10);
        assert!(!may_pass());
    }
}
