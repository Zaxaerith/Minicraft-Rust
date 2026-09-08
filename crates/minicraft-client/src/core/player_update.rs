//! Player.tick's early-return boundary, before Mob.tick and HUD controls.
//! Updater may call Player before Display, but an already-requested Display
//! makes Player return without advancing gameplay state (Java 2.2.4).
pub fn can_tick(has_level: bool, removed: bool, has_requested_display: bool) -> bool {
    has_level && !removed && !has_requested_display
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn player_entry_rejects_missing_level_removed_and_requested_display() {
        for has_level in [false, true] {
            for removed in [false, true] {
                for display in [false, true] {
                    assert_eq!(
                        can_tick(has_level, removed, display),
                        has_level && !removed && !display
                    );
                }
            }
        }
    }
}
