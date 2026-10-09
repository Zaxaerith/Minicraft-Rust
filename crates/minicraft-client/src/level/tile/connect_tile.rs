//! Java `ConnectTile` — world-edge auto-tile filler, never a real gameplay tile.

/// Java `ConnectTile.mayPass`.
pub fn may_pass() -> bool {
    false
}

/// Java `ConnectTile.maySpawn`.
pub const MAY_SPAWN: bool = false;

/// ConnectTile is not part of the 2.2.4 playable registry (id 255 in Java).
pub fn is_debug_only() -> bool {
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn connect_tile_blocks_and_never_spawns() {
        assert!(!super::may_pass());
        const {
            assert!(!super::MAY_SPAWN);
        }
        assert!(super::is_debug_only());
    }
}
