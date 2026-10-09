//! Java `DecorTile` — ornate stone / ornate obsidian decorations.
use super::{Material, Tile};

pub const MAY_SPAWN: bool = true;

/// Java `DecorTile.mayPass`.
pub fn may_pass() -> bool {
    true
}

/// Required mining tool material for each decor kind.
pub fn required_material(ornate_obsidian: bool) -> Material {
    if ornate_obsidian {
        Material::Obsidian
    } else {
        Material::Stone
    }
}

/// Broken decor becomes cloud on sky depth, otherwise hole.
pub fn broken_tile(depth: i8) -> Tile {
    if depth == 1 { Tile::Cloud } else { Tile::Hole }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decor_pass_and_tools() {
        assert!(may_pass());
        assert_eq!(required_material(false), Material::Stone);
        assert_eq!(required_material(true), Material::Obsidian);
        assert_eq!(broken_tile(1), Tile::Cloud);
        assert_eq!(broken_tile(-1), Tile::Hole);
        const {
            assert!(MAY_SPAWN);
        }
    }
}
