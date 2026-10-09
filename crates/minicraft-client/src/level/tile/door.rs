//! Java `DoorTile` (wood / stone / obsidian) plus boss door shared predicates.
use super::{Material, Tile};

/// Java `DoorTile.mayPass`: open doors (`data != 0`) are walkable.
pub fn is_closed(data: u16) -> bool {
    data == 0
}

pub fn may_pass(data: u16) -> bool {
    !is_closed(data)
}

/// Java `DoorTile.hurt` (player bump) toggles closed/open.
pub fn toggle_data(data: u16) -> u16 {
    if is_closed(data) { 1 } else { 0 }
}

/// Java `DoorTile.interact` breaks the door into the matching floor (`id + 3`).
pub fn floor_for(material: Material) -> Tile {
    match material {
        Material::Wood => Tile::WoodFloor,
        Material::Stone => Tile::StoneFloor,
        Material::Obsidian => Tile::ObsidianFloor,
    }
}

/// Open sprite asset suffix used by the renderer.
pub fn open_asset(material: Material) -> &'static str {
    match material {
        Material::Wood => "wood_door_opened",
        Material::Stone => "stone_door_opened",
        Material::Obsidian => "obsidian_door_opened",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn door_open_closed_and_floor_mapping() {
        assert!(is_closed(0));
        assert!(!is_closed(1));
        assert!(may_pass(1));
        assert!(!may_pass(0));
        assert_eq!(toggle_data(0), 1);
        assert_eq!(toggle_data(1), 0);
        assert_eq!(floor_for(Material::Wood), Tile::WoodFloor);
        assert_eq!(floor_for(Material::Stone), Tile::StoneFloor);
        assert_eq!(floor_for(Material::Obsidian), Tile::ObsidianFloor);
    }
}
