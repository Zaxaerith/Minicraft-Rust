//! Player.tick's early-return boundary, before Mob.tick and HUD controls.
//! Java Updater.tick still invokes Game.player.tick() while currentDisplay
//! != null, but Player.tick returns immediately when a Display is already
//! requested — no hurt/attack decrement, potion/fishing, hunger_ticks or
//! stamina_recharge. World menu early-returns in tick_after_prelude match
//! that no-op (Java 2.2.4).
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
