//! Java `Inventory` — 27-slot bag and `StackableItem` / `ItemStack` stacking.
use serde::{Deserialize, Serialize};

use super::{ItemId, MAX_STACK, ToolItem, ToolKind, ToolTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u16,
    /// `None` on a legacy watering can means its individual contents were not
    /// recorded. It must not be silently interpreted as an empty can.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watering_content: Option<u16>,
}

impl ItemStack {
    pub const fn new(item: ItemId, count: u16) -> Self {
        Self {
            item,
            count,
            watering_content: if matches!(item, ItemId::WateringCan) {
                Some(0)
            } else {
                None
            },
        }
    }

    pub const fn with_water(content: u16) -> Self {
        Self {
            item: ItemId::WateringCan,
            count: 1,
            watering_content: Some(content),
        }
    }

    pub(crate) fn validate_instance(self) -> Result<(), String> {
        if self.item == ItemId::WateringCan {
            if self.count != 1 {
                return Err("watering can is not one instance".to_owned());
            }
            if self.watering_content.is_some_and(|content| content > 1_800) {
                return Err("watering can exceeds capacity 1800".to_owned());
            }
        } else if self.watering_content.is_some() {
            return Err("non-watering item has water contents".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    slots: Vec<ItemStack>,
    pub(crate) tools: Vec<ToolItem>,
    capacity: usize,
    /// Invalidates selected slot references whenever existing positions change.
    #[serde(default)]
    slot_revision: u64,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: Vec::new(),
            tools: Vec::new(),
            capacity,
            slot_revision: 0,
        }
    }

    pub fn slots(&self) -> &[ItemStack] {
        &self.slots
    }

    pub(crate) fn slot_mut(&mut self, index: usize) -> Option<&mut ItemStack> {
        self.slots.get_mut(index)
    }

    pub fn slot_revision(&self) -> u64 {
        self.slot_revision
    }

    fn invalidate_slot_references(&mut self) {
        self.slot_revision = self.slot_revision.wrapping_add(1);
    }

    pub fn tools(&self) -> &[ToolItem] {
        &self.tools
    }

    pub fn tools_mut(&mut self) -> &mut [ToolItem] {
        &mut self.tools
    }

    pub fn used_slots(&self) -> usize {
        self.slots.len() + self.tools.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Restores a fixed slot limit and returns stack slots that no longer fit. The
    /// world loader drops those stacks beside the player instead of deleting them.
    pub fn set_capacity(&mut self, capacity: usize) -> Vec<ItemStack> {
        self.capacity = capacity;
        let stack_capacity = capacity.saturating_sub(self.tools.len());
        if self.slots.len() > stack_capacity {
            self.invalidate_slot_references();
            self.slots.split_off(stack_capacity)
        } else {
            Vec::new()
        }
    }

    pub fn take_all(&mut self) -> Vec<ItemStack> {
        if !self.slots.is_empty() {
            self.invalidate_slot_references();
        }
        std::mem::take(&mut self.slots)
    }

    pub(crate) fn take_tools(&mut self) -> Vec<ToolItem> {
        std::mem::take(&mut self.tools)
    }

    pub(crate) fn take_tool(&mut self, index: usize) -> Option<ToolItem> {
        (index < self.tools.len()).then(|| self.tools.remove(index))
    }

    pub fn add_tool(&mut self, tool: ToolItem) -> Option<usize> {
        if self.used_slots() >= self.capacity {
            return None;
        }
        self.tools.push(tool);
        Some(self.tools.len() - 1)
    }

    pub(crate) fn remove_tool(&mut self, kind: ToolKind, tier: ToolTier) -> bool {
        let Some(index) = self
            .tools
            .iter()
            .position(|tool| tool.kind == kind && tool.tier == tier)
        else {
            return false;
        };
        self.take_tool(index).is_some()
    }

    pub fn count(&self, item: ItemId) -> u16 {
        self.slots
            .iter()
            .filter(|stack| stack.item == item)
            .map(|stack| stack.count)
            .fold(0, u16::saturating_add)
    }

    /// Adds as much as possible and returns the uninserted remainder.
    pub fn add(&mut self, item: ItemId, count: u16) -> u16 {
        if item == ItemId::WateringCan {
            // This API creates new items. Construct each empty can separately;
            // transferring an existing can must use add_stack with its state.
            let inserted = usize::from(count).min(self.capacity.saturating_sub(self.used_slots()));
            self.slots
                .extend((0..inserted).map(|_| ItemStack::with_water(0)));
            return count - inserted as u16;
        }
        self.add_stack(ItemStack::new(item, count))
    }

    /// Transfers the full instance state and returns the uninserted quantity.
    /// Callers retain the original state when constructing any remainder.
    pub fn add_stack(&mut self, stack: ItemStack) -> u16 {
        let ItemStack {
            item,
            mut count,
            watering_content,
        } = stack;
        if item == ItemId::WateringCan {
            if watering_content.is_some_and(|content| content > 1_800 || count != 1) {
                return count;
            }
            while count > 0 && self.used_slots() < self.capacity {
                self.slots.push(ItemStack {
                    item,
                    count: 1,
                    watering_content,
                });
                count -= 1;
            }
            return count;
        }
        if watering_content.is_some() {
            return count;
        }
        for stack in self.slots.iter_mut().filter(|stack| stack.item == item) {
            if stack.watering_content.is_some() {
                continue;
            }
            let room = MAX_STACK.saturating_sub(stack.count);
            let inserted = room.min(count);
            stack.count += inserted;
            count -= inserted;
            if count == 0 {
                return 0;
            }
        }
        while count > 0 && self.used_slots() < self.capacity {
            let inserted = count.min(MAX_STACK);
            self.slots.push(ItemStack::new(item, inserted));
            count -= inserted;
        }
        count
    }

    /// Removes from this exact slot, rather than a different equal item ID.
    pub fn remove_slot(&mut self, index: usize, count: u16) -> Option<ItemStack> {
        if count == 0 {
            return None;
        }
        let stack = self.slots.get_mut(index)?;
        let removed = count.min(stack.count);
        if removed == 0 {
            return None;
        }
        let result = ItemStack {
            count: removed,
            ..*stack
        };
        stack.count -= removed;
        if stack.count == 0 {
            self.slots.remove(index);
            self.invalidate_slot_references();
        }
        Some(result)
    }

    pub fn remove(&mut self, item: ItemId, mut count: u16) -> bool {
        if self.count(item) < count {
            return false;
        }
        for stack in self.slots.iter_mut().rev() {
            if stack.item != item {
                continue;
            }
            let removed = stack.count.min(count);
            stack.count -= removed;
            count -= removed;
            if count == 0 {
                break;
            }
        }
        let previous_len = self.slots.len();
        self.slots.retain(|stack| stack.count > 0);
        if previous_len != self.slots.len() {
            self.invalidate_slot_references();
        }
        true
    }

    pub fn validate_instances(&self) -> Result<(), String> {
        for (index, stack) in self.slots.iter().enumerate() {
            stack
                .validate_instance()
                .map_err(|error| format!("inventory slot {index}: {error}"))?;
        }
        Ok(())
    }

    /// Splits old shared-state can stacks without inventing individual water
    /// amounts. The world migration separately assigns a provably unique can.
    /// The cap prevents hostile legacy counts from expanding without a bound.
    pub(crate) fn normalize_legacy_water(&mut self) -> Result<(), String> {
        const MAX_LEGACY_SLOTS: usize = 65_536;
        let mut expanded_len = self.slots.len();
        let mut needs_expansion = false;
        for stack in &self.slots {
            if stack.item != ItemId::WateringCan {
                continue;
            }
            if stack.count == 0 {
                return Err("legacy watering can has zero instances".to_owned());
            }
            if stack.count > 1 && stack.watering_content.is_some() {
                return Err("legacy multi-can stack has ambiguous instance contents".to_owned());
            }
            if stack.count > 1 {
                needs_expansion = true;
                expanded_len = expanded_len
                    .checked_add(usize::from(stack.count) - 1)
                    .ok_or("legacy watering can instance count overflows")?;
            }
        }
        if !needs_expansion {
            return Ok(());
        }
        if expanded_len > MAX_LEGACY_SLOTS {
            return Err(format!(
                "legacy watering can expansion exceeds {MAX_LEGACY_SLOTS} slots"
            ));
        }
        let mut expanded = Vec::new();
        expanded
            .try_reserve_exact(expanded_len)
            .map_err(|error| format!("cannot reserve legacy watering can instances: {error}"))?;
        for stack in &self.slots {
            if stack.item == ItemId::WateringCan {
                expanded.extend((0..stack.count).map(|_| ItemStack { count: 1, ..*stack }));
            } else {
                expanded.push(*stack);
            }
        }
        self.slots = expanded;
        self.invalidate_slot_references();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watering_can_instances_never_merge_or_lose_contents_on_transfer() {
        let mut bag = Inventory::new(3);
        let first = ItemStack::with_water(731);
        let second = ItemStack::with_water(23);
        assert_eq!(bag.add_stack(first), 0);
        assert_eq!(bag.add_stack(second), 0);
        assert_eq!(bag.add(ItemId::WateringCan, 1), 0);
        assert_eq!(bag.slots(), &[first, second, ItemStack::with_water(0)]);
        assert_eq!(bag.add_stack(ItemStack::with_water(1_800)), 1);
        let mut chest = Inventory::new(1);
        let removed = bag.remove_slot(1, 1).unwrap();
        assert_eq!(removed, second);
        assert_eq!(chest.add_stack(removed), 0);
        assert_eq!(bag.slots(), &[first, ItemStack::with_water(0)]);
        assert_eq!(chest.slots(), &[second]);
        assert!(bag.validate_instances().is_ok());
        assert!(chest.validate_instances().is_ok());
    }

    #[test]
    fn capacity_overflow_and_split_remainders_preserve_unknown_instances() {
        let mut bag = Inventory::new(2);
        let unknown = ItemStack {
            item: ItemId::WateringCan,
            count: 3,
            watering_content: None,
        };
        assert_eq!(bag.add_stack(unknown), 1);
        assert_eq!(bag.count(ItemId::WateringCan), 2);
        assert!(
            bag.slots()
                .iter()
                .all(|can| can.count == 1 && can.watering_content.is_none())
        );
        let overflow = bag.set_capacity(1);
        assert_eq!(overflow.len(), 1);
        assert_eq!(overflow[0].watering_content, None);
        assert_eq!(bag.used_slots(), 1);
    }

    #[test]
    fn slot_revision_changes_only_when_existing_positions_can_be_invalidated() {
        let mut bag = Inventory::new(4);
        let revision = bag.slot_revision();
        bag.add(ItemId::Wood, 5);
        bag.add_stack(ItemStack::with_water(23));
        assert_eq!(bag.slot_revision(), revision);
        assert_eq!(bag.remove_slot(0, 1), Some(ItemStack::new(ItemId::Wood, 1)));
        assert_eq!(bag.slot_revision(), revision);
        bag.slot_mut(1).unwrap().watering_content = Some(22);
        assert_eq!(bag.slot_revision(), revision);
        assert!(bag.remove(ItemId::Wood, 4));
        assert_ne!(bag.slot_revision(), revision);
        let revision = bag.slot_revision();
        bag.take_all();
        assert_ne!(bag.slot_revision(), revision);
        let revision = bag.slot_revision();
        bag.take_all();
        assert_eq!(bag.slot_revision(), revision);
        bag.slot_revision = u64::MAX;
        bag.add(ItemId::Wood, 1);
        bag.remove_slot(0, 1);
        assert_eq!(bag.slot_revision(), 0);
    }

    #[test]
    fn legacy_missing_water_field_is_unknown_and_normalization_is_bounded() {
        let mut bag: Inventory = serde_json::from_value(serde_json::json!({
            "slots": [{"item":"Wood","count":2}, {"item":"WateringCan","count":2}],
            "tools": [], "capacity":27
        }))
        .unwrap();
        assert_eq!(bag.slot_revision(), 0);
        assert_eq!(bag.slots()[1].watering_content, None);
        assert!(bag.validate_instances().is_err());
        bag.normalize_legacy_water().unwrap();
        assert_eq!(bag.slots().len(), 3);
        assert_eq!(bag.slots()[0], ItemStack::new(ItemId::Wood, 2));
        assert!(
            bag.slots()[1..]
                .iter()
                .all(|can| can.count == 1 && can.watering_content.is_none())
        );
        bag.validate_instances().unwrap();
        let encoded = serde_json::to_string(&bag).unwrap();
        assert!(!encoded.contains("watering_content"));
        assert_eq!(serde_json::from_str::<Inventory>(&encoded).unwrap(), bag);
        let revision = bag.slot_revision();
        bag.normalize_legacy_water().unwrap();
        assert_eq!(bag.slot_revision(), revision);

        let unknown = ItemStack {
            item: ItemId::WateringCan,
            count: u16::MAX,
            watering_content: None,
        };
        bag.slots = vec![unknown, unknown];
        let before = bag.clone();
        assert!(bag.normalize_legacy_water().unwrap_err().contains("65536"));
        assert_eq!(bag, before);
    }

    #[test]
    fn invalid_water_states_are_rejected_without_discarding_existing_items() {
        let mut bag = Inventory::new(4);
        bag.add_stack(ItemStack::with_water(23));
        let valid = bag.clone();
        assert_eq!(bag.add_stack(ItemStack::with_water(1_801)), 1);
        assert_eq!(
            bag.add_stack(ItemStack {
                count: 2,
                ..ItemStack::with_water(731)
            }),
            2
        );
        let invalid_wood = ItemStack {
            item: ItemId::Wood,
            count: 2,
            watering_content: Some(1),
        };
        assert_eq!(bag.add_stack(invalid_wood), 2);
        assert_eq!(bag, valid);
        bag.slots.push(invalid_wood);
        assert!(
            bag.validate_instances()
                .unwrap_err()
                .contains("non-watering")
        );
        bag.slots.pop();
        bag.slots[0].watering_content = Some(1_801);
        assert!(bag.validate_instances().unwrap_err().contains("capacity"));
        bag.slots[0] = ItemStack::new(ItemId::WateringCan, 2);
        let before = bag.clone();
        assert!(
            bag.normalize_legacy_water()
                .unwrap_err()
                .contains("ambiguous")
        );
        assert_eq!(bag, before);
    }

    #[test]
    fn new_multiple_cans_are_empty_instances_and_full_bags_keep_the_remainder() {
        let mut bag = Inventory::new(3);
        bag.add_stack(ItemStack::with_water(731));
        assert_eq!(bag.add(ItemId::WateringCan, 3), 1);
        assert_eq!(
            bag.slots(),
            &[
                ItemStack::with_water(731),
                ItemStack::with_water(0),
                ItemStack::with_water(0)
            ]
        );
        assert_eq!(bag.add(ItemId::WateringCan, 1), 1);
        bag.validate_instances().unwrap();
    }

    #[test]
    fn exact_tool_removal_preserves_durability_and_counts_toward_capacity() {
        let mut bag = Inventory::new(2);
        let mut first = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
        first.durability = 17;
        let mut second = first;
        second.durability = 11;
        assert_eq!(bag.add_tool(first), Some(0));
        assert_eq!(bag.add_tool(second), Some(1));
        assert_eq!(bag.add_stack(ItemStack::with_water(731)), 1);
        assert_eq!(bag.take_tool(1), Some(second));
        assert_eq!(bag.tools(), &[first]);
        assert_eq!(bag.take_tool(9), None);
        assert_eq!(bag.add_stack(ItemStack::with_water(731)), 0);
    }

    #[test]
    fn oversized_saved_stacks_do_not_overflow_or_delete_items() {
        let mut bag = Inventory::new(3);
        bag.slots = vec![
            ItemStack::new(ItemId::Wood, u16::MAX),
            ItemStack::new(ItemId::Wood, u16::MAX),
        ];
        assert_eq!(bag.count(ItemId::Wood), u16::MAX);
        assert_eq!(bag.add(ItemId::Wood, 1), 0);
        assert_eq!(bag.slots[0].count, u16::MAX);
        assert_eq!(bag.slots[1].count, u16::MAX);
        assert_eq!(bag.slots[2].count, 1);
    }
}
