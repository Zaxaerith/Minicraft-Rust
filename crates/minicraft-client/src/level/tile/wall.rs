//! Java `WallTile` (wood / stone / obsidian) and boss wall shared predicates.
use super::{Material, Tile};

pub const MAX_HEALTH: u16 = 100;

/// Java wall sprite connection: any other `WallTile` (including boss wall).
pub fn connects(neighbor: Tile) -> bool {
    matches!(
        neighbor,
        Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall | Tile::BossWall
    )
}

/// Java `WallTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Java `WallTile.tick` — decay stored damage.
pub fn tick_decay(data: u16) -> Option<u16> {
    (data > 0).then_some(data - 1)
}

/// Java `WallTile.hurt` drop count: `3 - material.ordinal()`.
pub fn drop_count(material: Material) -> u16 {
    3 - material.ordinal() as u16
}

/// Floor left behind when a wall is destroyed.
pub fn broken_floor(material: Material) -> Tile {
    match material {
        Material::Wood => Tile::WoodFloor,
        Material::Stone => Tile::StoneFloor,
        Material::Obsidian => Tile::ObsidianFloor,
    }
}

/// Obsidian wall on dungeon depth (-3) is locked until Air Wizard is beaten.
pub fn obsidian_locked(depth: i8, material: Material, air_wizard_beaten: bool) -> bool {
    depth == -3 && material == Material::Obsidian && !air_wizard_beaten
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_connects_across_materials() {
        assert!(connects(Tile::WoodWall));
        assert!(connects(Tile::BossWall));
        assert!(!connects(Tile::WoodFloor));
    }

    #[test]
    fn wall_health_drops_and_lock() {
        assert_eq!(MAX_HEALTH, 100);
        assert_eq!(drop_count(Material::Wood), 3);
        assert_eq!(drop_count(Material::Stone), 2);
        assert_eq!(drop_count(Material::Obsidian), 1);
        assert_eq!(broken_floor(Material::Stone), Tile::StoneFloor);
        assert!(obsidian_locked(-3, Material::Obsidian, false));
        assert!(!obsidian_locked(-3, Material::Obsidian, true));
        assert!(!obsidian_locked(0, Material::Obsidian, false));
        assert!(!may_pass());
    }
}
