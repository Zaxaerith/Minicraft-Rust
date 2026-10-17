//! Instance state and explicit migration of pre-instance native saves.
use super::{ActiveItem, EntityKind, ItemId, ItemStack, Player, World};

impl Player {
    pub(super) fn stack_for_active(&self, active: ActiveItem) -> Option<ItemStack> {
        match active {
            ActiveItem::Slot { index, revision } if revision == self.inventory.slot_revision() => {
                self.inventory.slots().get(index).copied()
            }
            ActiveItem::Stack(item) => self
                .inventory
                .slots()
                .iter()
                .find(|s| s.item == item)
                .copied(),
            _ => None,
        }
    }
    pub(super) fn active_slot(&self) -> Option<(usize, ItemStack)> {
        match self.active_item? {
            ActiveItem::Slot { index, revision } if revision == self.inventory.slot_revision() => {
                self.inventory
                    .slots()
                    .get(index)
                    .copied()
                    .map(|s| (index, s))
            }
            ActiveItem::Stack(ItemId::WateringCan)
                if self.inventory.count(ItemId::WateringCan) == 1 =>
            {
                self.inventory
                    .slots()
                    .iter()
                    .copied()
                    .enumerate()
                    .find(|(_, s)| s.item == ItemId::WateringCan)
            }
            _ => None,
        }
    }
}
impl World {
    pub(super) fn normalize_instance_selection(&mut self) {
        for active in [&mut self.player.active_item, &mut self.player.attack_item] {
            match *active {
                Some(ActiveItem::Slot { index, revision })
                    if revision != self.player.inventory.slot_revision()
                        || self
                            .player
                            .inventory
                            .slots()
                            .get(index)
                            .is_none_or(|s| s.item != ItemId::WateringCan) =>
                {
                    *active = None
                }
                Some(ActiveItem::Stack(ItemId::WateringCan)) => {
                    *active = if self.player.inventory.count(ItemId::WateringCan) == 1 {
                        self.player
                            .inventory
                            .slots()
                            .iter()
                            .position(|s| s.item == ItemId::WateringCan)
                            .map(|index| ActiveItem::Slot {
                                index,
                                revision: self.player.inventory.slot_revision(),
                            })
                    } else {
                        None
                    };
                }
                _ => {}
            }
        }
    }
    pub(super) fn upgrade_item_instances(&mut self) -> Result<(), String> {
        if self.player.item_instance_version > 1 {
            return Err("unsupported item instance version".into());
        }
        if self.player.item_instance_version == 1 {
            return Ok(());
        }
        if self.player.watering_content > crate::item::WATERING_CAN_CAPACITY {
            return Err("invalid legacy shared Watering Can content".into());
        }
        let ground_instances = self
            .levels
            .iter()
            .flat_map(|l| l.entities.entities())
            .filter_map(|e| match e.kind {
                EntityKind::Item(s) if s.item == ItemId::WateringCan => Some(u64::from(s.count)),
                _ => None,
            })
            .sum::<u64>();
        if ground_instances > 65_536 {
            return Err("too many legacy ground Watering Can instances".into());
        }
        let sole = self.player.inventory.count(ItemId::WateringCan) == 1;
        self.player.inventory.normalize_legacy_water()?;
        if sole
            && let Some(index) = self
                .player
                .inventory
                .slots()
                .iter()
                .position(|s| s.item == ItemId::WateringCan && s.watering_content.is_none())
        {
            self.player
                .inventory
                .slot_mut(index)
                .expect("located slot")
                .watering_content = Some(self.player.watering_content);
        }
        // Other old cans have no independent recorded quantity. Preserve None
        // (shown as unknown); only actual refilling establishes a known value.
        for level in &mut self.levels {
            let count = level
                .entities
                .entities()
                .iter()
                .filter_map(|e| match e.kind {
                    EntityKind::Item(s) if s.item == ItemId::WateringCan => {
                        Some(usize::from(s.count.saturating_sub(1)))
                    }
                    _ => None,
                })
                .sum::<usize>();
            let mut extra = Vec::new();
            extra
                .try_reserve(count)
                .map_err(|_| "cannot allocate legacy Watering Can instances")?;
            for entity in &mut level.entities.entities {
                if let Some(storage) = &mut entity.storage {
                    storage.normalize_legacy_water()?;
                }
                if let EntityKind::Item(stack) = &mut entity.kind
                    && stack.item == ItemId::WateringCan
                    && stack.count > 1
                {
                    if stack.watering_content.is_some() {
                        return Err("stacked stateful Watering Can in legacy ground item".into());
                    }
                    let count = stack.count;
                    stack.count = 1;
                    for _ in 1..count {
                        extra.push(entity.clone());
                    }
                }
            }
            for mut original in extra {
                let EntityKind::Item(stack) = original.kind else {
                    unreachable!()
                };
                let id = level.entities.spawn_item(stack, original.x, original.y);
                original.id = id;
                *level
                    .entities
                    .entities
                    .last_mut()
                    .expect("inserted legacy can") = original;
            }
        }
        self.player.item_instance_version = 1;
        Ok(())
    }
    pub(super) fn validate_item_instances(&self) -> Result<(), String> {
        if self.player.item_instance_version > 1 {
            return Err("unsupported item instance version".into());
        }
        self.player.inventory.validate_instances()?;
        for level in &self.levels {
            level.entities.validate_states()?;
            for entity in level.entities.entities() {
                if let Some(storage) = &entity.storage {
                    storage.validate_instances()?;
                }
                if let EntityKind::Item(stack) = entity.kind {
                    stack.validate_instance()?;
                }
            }
        }
        Ok(())
    }
}
