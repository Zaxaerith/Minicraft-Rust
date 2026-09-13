//! Java `HardRockTile`.
use super::Tile;

pub const MAX_HEALTH: u16 = 200;

/// Java sprite connection: only other hard rock tiles.
pub fn connects(neighbor: Tile) -> bool {
    neighbor == Tile::HardRock
}

/// Java `HardRockTile.render`: dirt underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), None]
}

/// Java `HardRockTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Java `HardRockTile.tick` — decay stored damage.
pub fn tick_decay(data: u16) -> Option<u16> {
    (data > 0).then(|| data - 1)
}

/// Only gem pickaxe (level 4) can mine hard rock.
pub const REQUIRED_TOOL_LEVEL: u8 = 4;

/// Stamina cost for a gem pickaxe hit (Java `payStamina(2)`).
pub const STAMINA_COST: i32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_rock_connects_only_to_hard_rock() {
        assert!(connects(Tile::HardRock));
        assert!(!connects(Tile::Rock));
    }

    #[test]
    fn hard_rock_health_and_gate() {
        assert_eq!(MAX_HEALTH, 200);
        assert_eq!(REQUIRED_TOOL_LEVEL, 4);
        assert!(!may_pass());
        assert_eq!(tick_decay(1), Some(0));
        assert_eq!(tick_decay(0), None);
    }
}
