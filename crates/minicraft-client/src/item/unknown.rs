//! Java `UnknownItem` — placeholder for invalid/unresolved item names.
use super::ItemId;

/// Java `Items.get` returns UnknownItem when a name is missing.
pub const UNKNOWN_ASSET: &str = "missing_item";

/// Marker used by save import when a legacy name cannot be resolved.
pub const fn is_unknown_marker(item: ItemId) -> bool {
    // Placeholder assets identify items that are not drawn as inventory textures.
    matches!(
        item,
        ItemId::PowerGlove
            | ItemId::FarmlandItem
            | ItemId::HoleItem
            | ItemId::LavaItem
            | ItemId::PathItem
            | ItemId::WaterItem
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_assets_are_known() {
        assert!(is_unknown_marker(ItemId::FarmlandItem));
        assert!(!is_unknown_marker(ItemId::Wood));
        assert_eq!(UNKNOWN_ASSET, "missing_item");
    }
}
