//! Java `BossFloorTile` — Obsidian floor locked until Obsidian Knight is beaten.
use super::Tile;

pub fn may_pass() -> bool {
    true
}

/// Locked while the knight fight is active / undefeated (survival only).
pub fn locked(knight_beaten: bool, knight_active: bool, creative: bool) -> bool {
    !creative && (!knight_beaten || knight_active)
}

/// Broken boss floor follows floor rules (sky → cloud).
pub fn broken_tile(depth: i8) -> Tile {
    if depth == 1 { Tile::Cloud } else { Tile::Hole }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_floor_lock_gate() {
        assert!(locked(false, false, false));
        assert!(!locked(true, false, false));
        assert!(!locked(false, false, true));
        assert!(may_pass());
        assert_eq!(broken_tile(1), Tile::Cloud);
    }
}
