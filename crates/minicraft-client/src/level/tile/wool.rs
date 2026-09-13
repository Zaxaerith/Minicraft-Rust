//! Java `WoolTile`.
use super::Tile;

/// Java `WoolTile.mayPass`: wool-walkable entities only.
pub fn may_pass(can_wool: bool) -> bool {
    can_wool
}

/// Wool tiles share the wool auto-tiling group only with identical colors.
pub fn connects(center: Tile, neighbor: Tile) -> bool {
    center == neighbor && is_wool(center)
}

pub fn is_wool(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::WhiteWool
            | Tile::RedWool
            | Tile::BlueWool
            | Tile::GreenWool
            | Tile::YellowWool
            | Tile::BlackWool
    )
}

/// Stamina cost to shear wool (Java `payStamina(3 - tool.level)` base 3).
pub const SHEAR_STAMINA_BASE: i32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wool_pass_and_connects() {
        assert!(may_pass(true));
        assert!(!may_pass(false));
        assert!(is_wool(Tile::RedWool));
        assert!(!is_wool(Tile::Path));
        // Same-color wool does not use the shared connects() dispatcher in mod.rs
        // (default false); this helper documents the per-tile identity rule.
        assert!(connects(Tile::RedWool, Tile::RedWool));
        assert!(!connects(Tile::RedWool, Tile::BlueWool));
    }
}
