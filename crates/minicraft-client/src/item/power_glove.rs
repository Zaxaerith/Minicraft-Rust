//! Java `PowerGloveItem` — pickup glove; never stored in the inventory.
//!
//! Java `Items` registers `new PowerGloveItem()` first; `Inventory.add`
//! rejects it and `Items.CreativeModeInventory` skips it.
use super::ItemId;

pub const ITEM: ItemId = ItemId::PowerGlove;

/// Java `Inventory.add` refuses Power Glove stacks.
pub fn is_power_glove(item: ItemId) -> bool {
    item == ITEM
}

/// Java `Items.CreativeModeInventory` skips the glove.
pub fn include_in_creative_inventory(item: ItemId) -> bool {
    !is_power_glove(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glove_is_excluded_from_creative_inventory() {
        assert!(is_power_glove(ItemId::PowerGlove));
        assert!(!include_in_creative_inventory(ItemId::PowerGlove));
        assert!(include_in_creative_inventory(ItemId::Wood));
    }
}
