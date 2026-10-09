//! Java `SandTile`.
use super::Tile;

pub const MAY_SPAWN: bool = true;

/// Java `SandTile.connectsToSand`.
pub fn connects(neighbor: Tile) -> bool {
    matches!(
        neighbor,
        Tile::Sand | Tile::Cactus | Tile::CactusSapling | Tile::Hole | Tile::Lava | Tile::Exploded
    )
}

/// Java `SandTile.render`: dirt underlay.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), None]
}

/// Java `SandTile.steppedOn` — sand particles every 8 walk distance.
pub fn should_emit_step_particle(walk_dist: i32) -> bool {
    walk_dist.rem_euclid(8) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_connects_to_sand_family() {
        assert!(connects(Tile::Sand));
        assert!(connects(Tile::Cactus));
        assert!(connects(Tile::CactusSapling));
        assert!(connects(Tile::Hole));
        assert!(connects(Tile::Lava));
        assert!(!connects(Tile::Water));
    }

    #[test]
    fn sand_step_particles_every_eight() {
        assert!(should_emit_step_particle(0));
        assert!(should_emit_step_particle(8));
        assert!(!should_emit_step_particle(7));
        const {
            assert!(MAY_SPAWN);
        }
    }
}
