//! Java `SignTile`.
use super::Tile;

/// Sign appearance is the base floor stored in tile data.
pub fn underlay(data: u16) -> [Option<Tile>; 2] {
    let base = u8::try_from(data).ok().and_then(Tile::from_id);
    if matches!(base, Some(Tile::Grass | Tile::Sand)) {
        [Some(Tile::Dirt), base]
    } else {
        [base, None]
    }
}

/// Broken sign restores its base floor and removes the tile entity.
pub fn broken_tile(data: u16) -> Tile {
    Tile::from_id(data as u8).unwrap_or(Tile::Dirt)
}

/// Java `SignTile` accepts an empty hand (open editor) or an axe (break).
pub fn accepts_axe(is_axe: bool) -> bool {
    is_axe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_underlay_and_break() {
        assert_eq!(
            underlay(Tile::Sand.id() as u16),
            [Some(Tile::Dirt), Some(Tile::Sand)]
        );
        assert_eq!(broken_tile(Tile::WoodFloor.id() as u16), Tile::WoodFloor);
        assert!(accepts_axe(true));
        assert!(!accepts_axe(false));
    }
}
