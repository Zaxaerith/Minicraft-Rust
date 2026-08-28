use super::Tile;

const MOB_SPAWN_FACTOR: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaturalMob {
    Slime,
    Zombie,
    Creeper,
    Skeleton,
    Snake,
    Knight,
    Cow,
    Pig,
    Sheep,
    AirWizard,
    ObsidianKnight,
}

impl NaturalMob {
    pub const ALL: [Self; 11] = [
        Self::Slime,
        Self::Zombie,
        Self::Creeper,
        Self::Skeleton,
        Self::Snake,
        Self::Knight,
        Self::Cow,
        Self::Pig,
        Self::Sheep,
        Self::AirWizard,
        Self::ObsidianKnight,
    ];

    pub fn id(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnIntent {
    pub kind: NaturalMob,
    pub x: i32,
    pub y: i32,
}

pub fn max_mob_count(depth: i8, difficulty: usize) -> usize {
    let mut maximum = 150 + 150 * difficulty.min(2);
    if depth == 1 {
        maximum /= 2;
    }
    if matches!(depth, 0 | -4 | -5) {
        maximum = maximum * 2 / 3;
    }
    maximum
}

pub fn spawn_skip_chance(mob_count: usize, max_mob_count: usize) -> usize {
    if max_mob_count == 0 {
        return usize::MAX;
    }
    MOB_SPAWN_FACTOR.saturating_mul(mob_count.saturating_mul(mob_count))
        / max_mob_count.saturating_mul(max_mob_count)
}

pub fn hostile_allowed(depth: i8, day_tick: u32, days: u32, tile: Tile, lit: bool) -> bool {
    let nighttime_after_day_one = day_tick >= 48_600 && days > 1;
    let valid_time = depth != 0 || nighttime_after_day_one;
    valid_time && !lit && hostile_tile_allowed(depth, tile)
}

pub fn passive_allowed(depth: i8, tile: Tile) -> bool {
    depth == 0 && matches!(tile, Tile::Grass | Tile::Flower)
}

pub fn choose_hostile(depth: i8, roll: i32) -> NaturalMob {
    if depth == -4 {
        if roll <= 40 || roll >= 85 {
            NaturalMob::Snake
        } else {
            NaturalMob::Knight
        }
    } else if roll <= 40 {
        NaturalMob::Slime
    } else if roll <= 75 {
        NaturalMob::Zombie
    } else if roll >= 85 {
        NaturalMob::Skeleton
    } else {
        NaturalMob::Creeper
    }
}

pub fn choose_passive(night: bool, roll: i32) -> NaturalMob {
    if roll <= if night { 22 } else { 33 } {
        NaturalMob::Cow
    } else if roll >= 68 {
        NaturalMob::Pig
    } else {
        NaturalMob::Sheep
    }
}

fn hostile_tile_allowed(depth: i8, tile: Tile) -> bool {
    if depth == -4 {
        tile == Tile::ObsidianFloor
    } else {
        matches!(
            tile,
            Tile::Grass
                | Tile::Dirt
                | Tile::Flower
                | Tile::TreeSapling
                | Tile::Sand
                | Tile::CactusSapling
                | Tile::WoodFloor
                | Tile::StoneFloor
                | Tile::ObsidianFloor
                | Tile::Path
                | Tile::RawStone
                | Tile::RawObsidian
                | Tile::OrnateStone
                | Tile::OrnateObsidian
                | Tile::BossFloor
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_and_depth_match_java_mob_caps() {
        assert_eq!(max_mob_count(-1, 0), 150);
        assert_eq!(max_mob_count(-1, 1), 300);
        assert_eq!(max_mob_count(-1, 2), 450);
        assert_eq!(max_mob_count(1, 1), 150);
        assert_eq!(max_mob_count(0, 1), 200);
        assert_eq!(max_mob_count(-4, 2), 300);
        assert_eq!(spawn_skip_chance(300, 300), 100);
    }

    #[test]
    fn spawn_time_light_tiles_and_species_follow_2_2_4_rules() {
        assert!(!hostile_allowed(0, 50_000, 1, Tile::Grass, false));
        assert!(hostile_allowed(0, 50_000, 2, Tile::Grass, false));
        assert!(!hostile_allowed(0, 50_000, 2, Tile::Grass, true));
        assert!(!hostile_allowed(-1, 0, 1, Tile::Farmland, false));
        assert!(hostile_allowed(-4, 0, 1, Tile::ObsidianFloor, false));
        assert_eq!(choose_hostile(-1, 80), NaturalMob::Creeper);
        assert_eq!(choose_hostile(-4, 80), NaturalMob::Knight);
        assert_eq!(choose_passive(false, 20), NaturalMob::Cow);
        assert_eq!(choose_passive(false, 70), NaturalMob::Pig);
        assert!(passive_allowed(0, Tile::Flower));
    }
}
