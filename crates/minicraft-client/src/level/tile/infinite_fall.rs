//! Java `InfiniteFallTile`.
use super::Tile;

/// Java `InfiniteFallTile.mayPass`: AirWizard, arrows, and creative players only.
pub fn may_pass(is_air_wizard: bool, is_arrow: bool, is_creative_player: bool) -> bool {
    is_air_wizard || is_arrow || is_creative_player
}

/// Java `InfiniteFallTile` renders nothing and has no connection checker.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

/// Creative break / stand path treats infinite fall as solid except creative.
pub fn solid_for_player(is_creative: bool) -> bool {
    !is_creative
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinite_fall_pass_rules() {
        assert!(may_pass(true, false, false));
        assert!(may_pass(false, true, false));
        assert!(may_pass(false, false, true));
        assert!(!may_pass(false, false, false));
        assert!(solid_for_player(false));
        assert!(!solid_for_player(true));
        assert!(!connects(Tile::Cloud));
    }
}
