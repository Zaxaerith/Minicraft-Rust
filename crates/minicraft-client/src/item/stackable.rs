//! Java `StackableItem` — stack metadata shared by inventory stacks.
use super::{ItemId, MAX_STACK};

/// Java `StackableItem.maxCount` (inventory stacks clamp at MAX_STACK here).
pub const DEFAULT_MAX_COUNT: u16 = MAX_STACK;

/// Java `StackableItem.getAllInstances` — plain materials that only stack.
pub const ALL: &[ItemId] = &[
    ItemId::Wood,
    ItemId::Leather,
    ItemId::Wheat,
    ItemId::Key,
    ItemId::Arrow,
    ItemId::String,
    ItemId::Coal,
    ItemId::IronOre,
    ItemId::Lapis,
    ItemId::GoldOre,
    ItemId::IronIngot,
    ItemId::GoldIngot,
    ItemId::Gunpowder,
    ItemId::Slime,
    ItemId::Glass,
    ItemId::Cloth,
    ItemId::Gem,
    ItemId::Scale,
    ItemId::Shard,
    ItemId::CloudOre,
    ItemId::GlassBottle,
    ItemId::Tomato,
    ItemId::Bone,
    ItemId::Fertilizer,
];

/// Java `StackableItem.stacksWith` — same item identity stacks.
pub fn stacks_with(a: ItemId, b: ItemId) -> bool {
    a == b
}

/// Java `StackableItem.getDisplayName` prefix amount (`min(count, 999)`).
pub fn display_count(count: u16) -> u16 {
    count.min(MAX_STACK)
}

/// Java `StackableItem.getData` save form `Name_count`.
pub fn save_data(item: ItemId, count: u16) -> String {
    format!("{}_{}", item.display_name(), count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stackables_match_java_instances() {
        assert_eq!(ALL.len(), 24);
        assert!(stacks_with(ItemId::Wood, ItemId::Wood));
        assert!(!stacks_with(ItemId::Wood, ItemId::Stone));
        assert_eq!(display_count(1_500), MAX_STACK);
        assert_eq!(save_data(ItemId::Wood, 3), "Wood_3");
    }
}
