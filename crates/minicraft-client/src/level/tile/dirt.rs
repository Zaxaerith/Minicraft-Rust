//! Java `DirtTile`.
use super::Tile;

pub const MAY_SPAWN: bool = true;

/// Dirt has no custom connection checker in Java.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

/// Java `DirtTile.render`: depth-tinted dirt sprite, no extra underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [None, None]
}

/// Depth index for dirt variants (0 surface, 1 caves, 2 dungeons).
pub fn depth_sprite_index(depth: i8) -> usize {
    match depth {
        0 => 0,
        -4 => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirt_never_auto_tile_connects() {
        assert!(!connects(Tile::Dirt));
        assert!(!connects(Tile::Grass));
    }

    #[test]
    fn dirt_depth_sprite_index_matches_java_didx() {
        assert_eq!(depth_sprite_index(0), 0);
        assert_eq!(depth_sprite_index(-4), 2);
        assert_eq!(depth_sprite_index(-1), 1);
        assert_eq!(depth_sprite_index(1), 1);
        const {
            assert!(MAY_SPAWN);
        }
    }
}
