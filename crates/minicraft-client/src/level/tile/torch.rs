//! Java `TorchTile`.
use super::Tile;

pub const LIGHT_RADIUS: i32 = 5;

/// Torch appearance is the base floor stored in tile data.
pub fn underlay(data: u16) -> [Option<Tile>; 2] {
    let base = u8::try_from(data).ok().and_then(Tile::from_id);
    if matches!(base, Some(Tile::Grass | Tile::Sand)) {
        [Some(Tile::Dirt), base]
    } else {
        [base, None]
    }
}

/// Java `TorchTile` forwards connects to the base tile in data.
pub fn connects_with_data(data: u16, neighbor: Tile) -> bool {
    let base = Tile::from_id(data as u8).unwrap_or(Tile::Dirt);
    super::connects(base, neighbor)
}

/// Broken torch restores its base floor.
pub fn broken_tile(data: u16) -> Tile {
    Tile::from_id(data as u8).unwrap_or(Tile::Dirt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_light_and_base_underlay() {
        assert_eq!(LIGHT_RADIUS, 5);
        assert_eq!(
            underlay(Tile::Grass.id() as u16),
            [Some(Tile::Dirt), Some(Tile::Grass)]
        );
        assert_eq!(
            underlay(Tile::WoodFloor.id() as u16),
            [Some(Tile::WoodFloor), None]
        );
        assert_eq!(broken_tile(Tile::StoneFloor.id() as u16), Tile::StoneFloor);
        // Torch on grass inherits grass connection to path.
        assert!(connects_with_data(Tile::Grass.id() as u16, Tile::Path));
    }
}
