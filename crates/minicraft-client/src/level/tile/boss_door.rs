//! Java `BossDoorTile` — Obsidian door locked during the Obsidian Knight fight.
use super::Tile;

/// Locked while the knight is active (survival only).
pub fn hurt_locked(knight_active: bool, creative: bool, source_is_player: bool) -> bool {
    source_is_player && knight_active && !creative
}

/// Interact lock matches other boss tiles.
pub fn locked(knight_beaten: bool, knight_active: bool, creative: bool) -> bool {
    !creative && (!knight_beaten || knight_active)
}

/// Boss doors follow the same open/closed solid rule as normal doors.
pub fn is_closed(data: u16) -> bool {
    data == 0
}

pub fn may_pass(data: u16) -> bool {
    !is_closed(data)
}

pub fn floor_tile() -> Tile {
    Tile::BossFloor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_door_lock_and_open() {
        assert!(hurt_locked(true, false, true));
        assert!(!hurt_locked(true, false, false));
        assert!(!hurt_locked(true, true, true));
        assert!(locked(false, false, false));
        assert!(!locked(true, false, false));
        assert!(may_pass(1));
        assert!(!may_pass(0));
        assert_eq!(floor_tile(), Tile::BossFloor);
    }
}
