//! Java `OreTile` (iron, gold, gem, lapis, cloud).
use super::Tile;

/// Java `OreType` variants registered by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OreType {
    Iron,
    Gold,
    Gem,
    Lapis,
    Cloud,
}

impl OreType {
    pub fn tile(self) -> Tile {
        match self {
            Self::Iron => Tile::IronOre,
            Self::Gold => Tile::GoldOre,
            Self::Gem => Tile::GemOre,
            Self::Lapis => Tile::LapisOre,
            Self::Cloud => Tile::CloudOre,
        }
    }

    /// Java drop replacement when the ore is fully mined.
    pub fn broken_tile(self) -> Tile {
        match self {
            Self::Cloud => Tile::Cloud,
            _ => Tile::Dirt,
        }
    }
}

/// Java `OreTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Java `OreTile.render`: dirt underlay (cloud ore uses cloud).
pub fn underlay(ore: OreType) -> [Option<Tile>; 2] {
    match ore {
        OreType::Cloud => [Some(Tile::Cloud), None],
        _ => [Some(Tile::Dirt), None],
    }
}

/// Java ore health: `random.nextInt(10) * 4 + 20` → 20..=56.
pub fn max_health(rand_0_to_9: u16) -> u16 {
    rand_0_to_9.min(9) * 4 + 20
}

/// Ore drop count: `random.nextInt(2)` plus 2 when the ore breaks.
pub fn drop_count(rand_0_or_1: u16, broke: bool) -> u16 {
    (rand_0_or_1 & 1) + if broke { 2 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ore_types_map_to_tiles_and_break_targets() {
        assert_eq!(OreType::Iron.tile(), Tile::IronOre);
        assert_eq!(OreType::Cloud.tile(), Tile::CloudOre);
        assert_eq!(OreType::Iron.broken_tile(), Tile::Dirt);
        assert_eq!(OreType::Cloud.broken_tile(), Tile::Cloud);
    }

    #[test]
    fn ore_health_and_drops_match_java() {
        assert_eq!(max_health(0), 20);
        assert_eq!(max_health(9), 56);
        assert_eq!(drop_count(0, false), 0);
        assert_eq!(drop_count(1, true), 3);
        assert!(!may_pass());
        assert_eq!(underlay(OreType::Cloud), [Some(Tile::Cloud), None]);
    }
}
