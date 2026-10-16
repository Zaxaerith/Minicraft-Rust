//! Java `FurnitureItem` — item ↔ furniture kind mapping and placement rules.
use super::ItemId;
use crate::entity::furniture::{FurnitureKind, bed};
use crate::item::dye::DyeColor;

/// Java `FurnitureItem.getAllInstances` item identities.
pub const ALL: &[ItemId] = &[
    ItemId::CowSpawner,
    ItemId::PigSpawner,
    ItemId::SheepSpawner,
    ItemId::SlimeSpawner,
    ItemId::ZombieSpawner,
    ItemId::CreeperSpawner,
    ItemId::SkeletonSpawner,
    ItemId::SnakeSpawner,
    ItemId::KnightSpawner,
    ItemId::Chest,
    ItemId::DungeonChest,
    ItemId::Workbench,
    ItemId::Oven,
    ItemId::Furnace,
    ItemId::Anvil,
    ItemId::Enchanter,
    ItemId::Loom,
    ItemId::Lantern,
    ItemId::IronLantern,
    ItemId::GoldLantern,
    ItemId::Bed,
    ItemId::Tnt,
    ItemId::Composter,
    ItemId::DyeVat,
    ItemId::BlackBed,
    ItemId::RedBed,
    ItemId::GreenBed,
    ItemId::BrownBed,
    ItemId::BlueBed,
    ItemId::PurpleBed,
    ItemId::CyanBed,
    ItemId::LightGrayBed,
    ItemId::GrayBed,
    ItemId::PinkBed,
    ItemId::LimeBed,
    ItemId::YellowBed,
    ItemId::LightBlueBed,
    ItemId::MagentaBed,
    ItemId::OrangeBed,
    ItemId::WhiteBed,
];

/// Java `FurnitureItem` constructor: wraps a furniture entity for placement.
pub fn furniture_for_item(item: ItemId) -> Option<FurnitureKind> {
    if DyeColor::from_bed_item(item).is_some() {
        return Some(FurnitureKind::Bed);
    }
    Some(match item {
        ItemId::DyeVat => FurnitureKind::DyeVat,
        ItemId::Workbench => FurnitureKind::Workbench,
        ItemId::Oven => FurnitureKind::Oven,
        ItemId::Furnace => FurnitureKind::Furnace,
        ItemId::Anvil => FurnitureKind::Anvil,
        ItemId::Enchanter => FurnitureKind::Enchanter,
        ItemId::Loom => FurnitureKind::Loom,
        ItemId::Chest => FurnitureKind::Chest,
        ItemId::DungeonChest => FurnitureKind::DungeonChest,
        ItemId::Lantern => FurnitureKind::Lantern,
        ItemId::IronLantern => FurnitureKind::IronLantern,
        ItemId::GoldLantern => FurnitureKind::GoldLantern,
        ItemId::Tnt => FurnitureKind::Tnt,
        ItemId::Bed => FurnitureKind::Bed,
        ItemId::Composter => FurnitureKind::Composter,
        ItemId::CowSpawner => FurnitureKind::CowSpawner,
        ItemId::PigSpawner => FurnitureKind::PigSpawner,
        ItemId::SheepSpawner => FurnitureKind::SheepSpawner,
        ItemId::SlimeSpawner => FurnitureKind::SlimeSpawner,
        ItemId::ZombieSpawner => FurnitureKind::ZombieSpawner,
        ItemId::CreeperSpawner => FurnitureKind::CreeperSpawner,
        ItemId::SkeletonSpawner => FurnitureKind::SkeletonSpawner,
        ItemId::SnakeSpawner => FurnitureKind::SnakeSpawner,
        ItemId::KnightSpawner => FurnitureKind::KnightSpawner,
        _ => return None,
    })
}

/// Inverse mapping used when picking furniture back up (Java Power Glove).
/// KnightStatue has no placeable item form.
pub fn item_for_furniture(kind: FurnitureKind) -> Option<ItemId> {
    Some(match kind {
        FurnitureKind::DyeVat => ItemId::DyeVat,
        FurnitureKind::Workbench => ItemId::Workbench,
        FurnitureKind::Oven => ItemId::Oven,
        FurnitureKind::Furnace => ItemId::Furnace,
        FurnitureKind::Anvil => ItemId::Anvil,
        FurnitureKind::Enchanter => ItemId::Enchanter,
        FurnitureKind::Loom => ItemId::Loom,
        FurnitureKind::Chest => ItemId::Chest,
        FurnitureKind::DungeonChest => ItemId::DungeonChest,
        FurnitureKind::Lantern => ItemId::Lantern,
        FurnitureKind::IronLantern => ItemId::IronLantern,
        FurnitureKind::GoldLantern => ItemId::GoldLantern,
        FurnitureKind::Tnt => ItemId::Tnt,
        FurnitureKind::Bed => ItemId::Bed,
        FurnitureKind::Composter => ItemId::Composter,
        FurnitureKind::CowSpawner => ItemId::CowSpawner,
        FurnitureKind::PigSpawner => ItemId::PigSpawner,
        FurnitureKind::SheepSpawner => ItemId::SheepSpawner,
        FurnitureKind::SlimeSpawner => ItemId::SlimeSpawner,
        FurnitureKind::ZombieSpawner => ItemId::ZombieSpawner,
        FurnitureKind::CreeperSpawner => ItemId::CreeperSpawner,
        FurnitureKind::SkeletonSpawner => ItemId::SkeletonSpawner,
        FurnitureKind::SnakeSpawner => ItemId::SnakeSpawner,
        FurnitureKind::KnightSpawner => ItemId::KnightSpawner,
        FurnitureKind::KnightStatue => return None,
    })
}

/// Color belongs to the furniture instance, not the shared Bed kind.
pub fn state_for_item(item: ItemId) -> u16 {
    DyeColor::from_bed_item(item).map_or(0, bed::state_for_color)
}

/// Native Bed state zero preserves the existing plain Bed item identity.
pub fn item_for_furniture_state(kind: FurnitureKind, state: u16) -> Option<ItemId> {
    if kind == FurnitureKind::Bed && state != 0 {
        return Some(bed::color_from_state(state).bed_item());
    }
    item_for_furniture(kind)
}

/// Java `FurnitureItem.canAttack` is false.
pub const fn can_attack(_item: ItemId) -> bool {
    false
}

/// Java `FurnitureItem.isDepleted` tracks the `placed` flag.
pub const fn is_depleted(placed: bool) -> bool {
    placed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn furniture_item_round_trips() {
        for &item in ALL {
            let kind = furniture_for_item(item).expect("furniture item maps");
            assert_eq!(
                item_for_furniture_state(kind, state_for_item(item)),
                Some(item)
            );
        }
    }

    #[test]
    fn knight_statue_has_no_item_form() {
        assert_eq!(item_for_furniture(FurnitureKind::KnightStatue), None);
        assert_eq!(furniture_for_item(ItemId::ObsidianPoppet), None);
    }
}
