//! Java `RockTile`.
use super::Tile;

pub const MAX_HEALTH: u16 = 50;

/// Java sprite connection: only other rock tiles.
pub fn connects(neighbor: Tile) -> bool {
    neighbor == Tile::Rock
}

/// Java `RockTile.render`: dirt underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), None]
}

/// Java `RockTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Java `RockTile.tick` — decay stored damage by 1; returns new data if damaged.
pub fn tick_decay(data: u16) -> Option<u16> {
    (data > 0).then(|| data - 1)
}

/// Stone drop count when mined with a pickaxe (Java: 1 + random 0..=2).
pub fn pickaxe_stone_count(rand_0_to_3: u16) -> u16 {
    1 + rand_0_to_3.min(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rock_only_connects_to_rock() {
        assert!(connects(Tile::Rock));
        assert!(!connects(Tile::HardRock));
        assert!(!connects(Tile::Dirt));
    }

    #[test]
    fn rock_tick_decays_damage() {
        assert_eq!(tick_decay(3), Some(2));
        assert_eq!(tick_decay(0), None);
        assert_eq!(MAX_HEALTH, 50);
        assert!(!may_pass());
    }
}
