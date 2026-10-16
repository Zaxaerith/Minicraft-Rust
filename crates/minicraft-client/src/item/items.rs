//! Java `Items` — name lookup and creative-inventory helpers on top of `ItemId`.
use super::ItemId;

/// Java `Items.get(name)` case-insensitive display/asset lookup.
/// Accepts either the display name ("Wood Sword" is tool-only) or asset stem.
pub fn get(name: &str) -> Option<ItemId> {
    let needle = name.trim().to_ascii_lowercase();
    if needle == "white wool" || needle == "white_wool" {
        return Some(ItemId::Wool);
    }
    ItemId::ALL.iter().copied().find(|item| {
        item.display_name().to_ascii_lowercase() == needle
            || item.asset_name().to_ascii_lowercase() == needle
    })
}

/// Java `Items.getCount` — stackable items report their stack size; others 1.
pub const fn get_count(item: Option<ItemId>, stack_count: u16) -> u16 {
    match item {
        None => 0,
        Some(_) if stack_count > 0 => stack_count,
        Some(_) => 1,
    }
}

/// Java `Items.arrowItem`.
pub const ARROW: ItemId = ItemId::Arrow;

/// Java `Items.CreativeModeInventory` lists every item except the glove.
pub fn creative_entries() -> impl Iterator<Item = ItemId> {
    ItemId::ALL
        .iter()
        .copied()
        .filter(|item| *item != ItemId::PowerGlove)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_lookup_is_case_insensitive() {
        assert_eq!(get("wood"), Some(ItemId::Wood));
        assert_eq!(get("WOOD"), Some(ItemId::Wood));
        // Java display name for the enchanted apple is "Gold Apple".
        assert_eq!(get("Gold Apple"), Some(ItemId::GoldenApple));
        assert_eq!(get("golden_apple"), Some(ItemId::GoldenApple));
        assert_eq!(get("not an item"), None);
    }

    #[test]
    fn creative_inventory_skips_power_glove() {
        let entries: Vec<_> = creative_entries().collect();
        assert_eq!(entries.len(), ItemId::ALL.len() - 1);
        assert!(!entries.contains(&ItemId::PowerGlove));
    }
}
