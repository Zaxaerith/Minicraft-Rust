//! Java `FenceTile` — wood / stone / obsidian fences on a base floor.
use super::{Material, Tile};

/// Java `FenceTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Fence appearance underlay comes from the base floor stored in tile data.
pub fn underlay(data: u16) -> [Option<Tile>; 2] {
    let base = u8::try_from(data).ok().and_then(Tile::from_id);
    if matches!(base, Some(Tile::Grass | Tile::Sand)) {
        [Some(Tile::Dirt), base]
    } else {
        [base, None]
    }
}

/// Java `FenceTile` connection sides: adjacent fences only.
pub fn connect_sides(neighbors: [Tile; 4]) -> [bool; 4] {
    let is_fence = |t: Tile| matches!(t, Tile::WoodFence | Tile::StoneFence | Tile::ObsidianFence);
    neighbors.map(is_fence)
}

/// Broken fence restores its base floor from data.
pub fn broken_tile(data: u16) -> Tile {
    Tile::from_id(data as u8).unwrap_or(Tile::Dirt)
}

pub fn material_for(tile: Tile) -> Option<Material> {
    match tile {
        Tile::WoodFence => Some(Material::Wood),
        Tile::StoneFence => Some(Material::Stone),
        Tile::ObsidianFence => Some(Material::Obsidian),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_underlay_and_broken_base() {
        assert_eq!(
            underlay(Tile::WoodFloor.id() as u16),
            [Some(Tile::WoodFloor), None]
        );
        assert_eq!(
            underlay(Tile::Grass.id() as u16),
            [Some(Tile::Dirt), Some(Tile::Grass)]
        );
        assert_eq!(broken_tile(Tile::StoneFloor.id() as u16), Tile::StoneFloor);
        assert!(!may_pass());
    }

    #[test]
    fn fence_sides_only_join_fences() {
        let sides = connect_sides([
            Tile::WoodFence,
            Tile::StoneFloor,
            Tile::StoneFence,
            Tile::Grass,
        ]);
        assert_eq!(sides, [true, false, true, false]);
        assert_eq!(material_for(Tile::ObsidianFence), Some(Material::Obsidian));
    }
}
