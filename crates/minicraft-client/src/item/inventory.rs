//! Java Inventory 27-slot bag and ItemStack stacking.
use serde::{Deserialize, Serialize};

use super::{ItemId, MAX_STACK, ToolItem, ToolKind, ToolTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u16,
}

impl ItemStack {
    pub const fn new(item: ItemId, count: u16) -> Self {
        Self { item, count }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    slots: Vec<ItemStack>,
    pub(crate) tools: Vec<ToolItem>,
    capacity: usize,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: Vec::new(),
            tools: Vec::new(),
            capacity,
        }
    }

    pub fn slots(&self) -> &[ItemStack] {
        &self.slots
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
            self.slots.split_off(stack_capacity)
        } else {
            Vec::new()
        }
    }

    pub fn take_all(&mut self) -> Vec<ItemStack> {
        std::mem::take(&mut self.slots)
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
        self.tools.remove(index);
        true
    }

    pub fn count(&self, item: ItemId) -> u16 {
        self.slots
            .iter()
            .filter(|stack| stack.item == item)
            .map(|stack| stack.count)
            .sum()
    }

    /// Adds as much as possible and returns the uninserted remainder.
    pub fn add(&mut self, item: ItemId, mut count: u16) -> u16 {
        for stack in self.slots.iter_mut().filter(|stack| stack.item == item) {
            let room = MAX_STACK - stack.count;
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
        self.slots.retain(|stack| stack.count > 0);
        true
    }
}
