//! Java `MaterialTile` — raw stone and raw obsidian walkable materials.
use super::{Material, Tile};

pub const MAY_SPAWN: bool = true;

/// Java `MaterialTile.mayPass`.
pub fn may_pass() -> bool {
    true
}

/// Broken raw material becomes cloud on sky depth, otherwise hole.
pub fn broken_tile(depth: i8) -> Tile {
    if depth == 1 { Tile::Cloud } else { Tile::Hole }
}

pub fn tile_for(material: Material) -> Tile {
    match material {
        Material::Stone => Tile::RawStone,
        Material::Obsidian => Tile::RawObsidian,
        Material::Wood => Tile::WoodFloor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_tile_pass_and_break() {
        assert!(may_pass());
        assert_eq!(broken_tile(1), Tile::Cloud);
        assert_eq!(broken_tile(0), Tile::Hole);
        assert_eq!(tile_for(Material::Stone), Tile::RawStone);
        assert_eq!(tile_for(Material::Obsidian), Tile::RawObsidian);
        const {
            assert!(MAY_SPAWN);
        }
    }
}
