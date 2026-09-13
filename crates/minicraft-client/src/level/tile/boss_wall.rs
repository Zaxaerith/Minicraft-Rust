//! Java `BossWallTile` — Obsidian wall locked until Obsidian Knight is beaten.
use super::Tile;

/// Boss walls auto-tile with other walls (Java extends `WallTile`).
pub fn connects(neighbor: Tile) -> bool {
    matches!(
        neighbor,
        Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall | Tile::BossWall
    )
}

/// Java `BossWallTile.mayPass` (inherited).
pub fn may_pass() -> bool {
    false
}

/// Locked while the knight fight is active / undefeated (survival only).
pub fn locked(knight_beaten: bool, knight_active: bool, creative: bool) -> bool {
    !creative && (!knight_beaten || knight_active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_wall_lock_gate() {
        assert!(locked(false, false, false));
        assert!(locked(true, true, false));
        assert!(!locked(true, false, false));
        assert!(!locked(false, true, true));
        assert!(!may_pass());
        assert!(connects(Tile::StoneWall));
    }
}
