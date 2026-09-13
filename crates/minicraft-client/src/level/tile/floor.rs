//! Java `FloorTile` (wood planks / stone bricks / obsidian floor) and boss floor.
use super::{Material, Tile};

pub const MAY_SPAWN: bool = true;

/// Java `FloorTile.mayPass`.
pub fn may_pass() -> bool {
    true
}

/// Broken floor on sky depth becomes cloud, otherwise hole.
pub fn broken_tile(depth: i8) -> Tile {
    if depth == 1 {
        Tile::Cloud
    } else {
        Tile::Hole
    }
}

/// Java floor drop when mined with the required tool.
pub fn drop_for(material: Material) -> Tile {
    // Tile identity used only as a drop-name carrier by callers.
    match material {
        Material::Wood => Tile::WoodFloor,
        Material::Stone => Tile::StoneFloor,
        Material::Obsidian => Tile::ObsidianFloor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_pass_and_break_targets() {
        assert!(may_pass());
        assert_eq!(broken_tile(1), Tile::Cloud);
        assert_eq!(broken_tile(0), Tile::Hole);
        assert_eq!(drop_for(Material::Wood), Tile::WoodFloor);
        assert!(MAY_SPAWN);
    }
}
