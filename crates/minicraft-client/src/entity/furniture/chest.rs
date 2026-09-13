//! Java Chest / DeathChest / DungeonChest storage and expiry.
use crate::item::{Inventory, ItemId};

use super::FurnitureKind;

/// Java DeathChest lifetime: 300 seconds (18000 ticks at 60 tps).
pub const DEATH_CHEST_LIFETIME: u32 = 300 * 60;

/// Death chest marker state (dungeon chest uses 0/1 for lock state).
pub const DEATH_CHEST_STATE: u16 = 2;

/// Java DungeonChest starting loot.
pub fn dungeon_chest_loot() -> Inventory {
    let mut storage = Inventory::new(27);
    storage.add(ItemId::GoldIngot, 3);
    storage.add(ItemId::Gem, 2);
    storage.add(ItemId::Key, 1);
    storage
}

/// Whether this furniture kind holds a container inventory.
pub const fn is_container(kind: FurnitureKind) -> bool {
    matches!(kind, FurnitureKind::Chest | FurnitureKind::DungeonChest)
}

/// Java DeathChest.tick: expire after lifetime or when emptied.
pub fn death_chest_expired(age: u32, used_slots: usize) -> bool {
    age >= DEATH_CHEST_LIFETIME || used_slots == 0
}
