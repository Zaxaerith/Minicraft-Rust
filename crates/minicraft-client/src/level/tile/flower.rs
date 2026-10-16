//! Java `FlowerTile`.
use super::Tile;

pub const MAY_SPAWN: bool = true;

/// Java `FlowerTile.connectsToGrass` — true, but FlowerTile's sprite has no
/// connection checker, so flowers do not auto-tile as the center tile.
pub fn connects_to_grass() -> bool {
    true
}

/// Dispatcher entry: no sprite connection checker on FlowerTile.
pub fn connects(_neighbor: Tile) -> bool {
    false
}

/// Java `FlowerTile.render`: grass underlay + flower sprite.
pub fn underlay() -> [Option<Tile>; 2] {
    [Some(Tile::Dirt), Some(Tile::Grass)]
}

/// Chance denominator for flower spreading grass onto dirt (Java `nextInt(30)`).
pub const SPREAD_CHANCE: i32 = 30;

/// Number of `FlowerVariant` values registered in Java.
pub const VARIANT_COUNT: usize = 19;

/// Java 2.3.0-dev `FlowerVariant` ordinal values. Stored only on `Flower230`;
/// existing `Flower` data remains the original two-shape encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum FlowerVariant {
    OxeyeDaisy = 0,
    Rose = 1,
    Sunflower = 2,
    Allium = 3,
    BlueOrchid = 4,
    Cornflower = 5,
    Dandelion = 6,
    Hydrangea = 7,
    Iris = 8,
    OrangeTulip = 9,
    PinkTulip = 10,
    RedTulip = 11,
    WhiteTulip = 12,
    Peony = 13,
    Periwinkle = 14,
    PinkLily = 15,
    WhiteLily = 16,
    Poppy = 17,
    Violet = 18,
}

impl FlowerVariant {
    pub const ALL: [Self; VARIANT_COUNT] = [
        Self::OxeyeDaisy,
        Self::Rose,
        Self::Sunflower,
        Self::Allium,
        Self::BlueOrchid,
        Self::Cornflower,
        Self::Dandelion,
        Self::Hydrangea,
        Self::Iris,
        Self::OrangeTulip,
        Self::PinkTulip,
        Self::RedTulip,
        Self::WhiteTulip,
        Self::Peony,
        Self::Periwinkle,
        Self::PinkLily,
        Self::WhiteLily,
        Self::Poppy,
        Self::Violet,
    ];

    pub fn from_data(data: u16) -> Option<Self> {
        Self::ALL.get(usize::from(data)).copied()
    }

    pub const fn item(self) -> crate::item::ItemId {
        match self {
            Self::OxeyeDaisy => crate::item::ItemId::Flower,
            Self::Rose => crate::item::ItemId::Rose,
            Self::Sunflower => crate::item::ItemId::Sunflower,
            Self::Allium => crate::item::ItemId::Allium,
            Self::BlueOrchid => crate::item::ItemId::BlueOrchid,
            Self::Cornflower => crate::item::ItemId::Cornflower,
            Self::Dandelion => crate::item::ItemId::Dandelion,
            Self::Hydrangea => crate::item::ItemId::Hydrangea,
            Self::Iris => crate::item::ItemId::Iris,
            Self::OrangeTulip => crate::item::ItemId::OrangeTulip,
            Self::PinkTulip => crate::item::ItemId::PinkTulip,
            Self::RedTulip => crate::item::ItemId::RedTulip,
            Self::WhiteTulip => crate::item::ItemId::WhiteTulip,
            Self::Peony => crate::item::ItemId::Peony,
            Self::Periwinkle => crate::item::ItemId::Periwinkle,
            Self::PinkLily => crate::item::ItemId::PinkLily,
            Self::WhiteLily => crate::item::ItemId::WhiteLily,
            Self::Poppy => crate::item::ItemId::Poppy,
            Self::Violet => crate::item::ItemId::Violet,
        }
    }

    pub const fn from_item(item: crate::item::ItemId) -> Option<Self> {
        match item {
            crate::item::ItemId::Flower => Some(Self::OxeyeDaisy),
            crate::item::ItemId::Rose => Some(Self::Rose),
            crate::item::ItemId::Sunflower => Some(Self::Sunflower),
            crate::item::ItemId::Allium => Some(Self::Allium),
            crate::item::ItemId::BlueOrchid => Some(Self::BlueOrchid),
            crate::item::ItemId::Cornflower => Some(Self::Cornflower),
            crate::item::ItemId::Dandelion => Some(Self::Dandelion),
            crate::item::ItemId::Hydrangea => Some(Self::Hydrangea),
            crate::item::ItemId::Iris => Some(Self::Iris),
            crate::item::ItemId::OrangeTulip => Some(Self::OrangeTulip),
            crate::item::ItemId::PinkTulip => Some(Self::PinkTulip),
            crate::item::ItemId::RedTulip => Some(Self::RedTulip),
            crate::item::ItemId::WhiteTulip => Some(Self::WhiteTulip),
            crate::item::ItemId::Peony => Some(Self::Peony),
            crate::item::ItemId::Periwinkle => Some(Self::Periwinkle),
            crate::item::ItemId::PinkLily => Some(Self::PinkLily),
            crate::item::ItemId::WhiteLily => Some(Self::WhiteLily),
            crate::item::ItemId::Poppy => Some(Self::Poppy),
            crate::item::ItemId::Violet => Some(Self::Violet),
            _ => None,
        }
    }

    pub const fn asset_name(self) -> &'static str {
        self.item().asset_name()
    }
}

/// Resolve a 2.3 flower item before changing its tile data. Legacy Flower
/// shapes have no item identity; Java 2.2.4 drops daisies and roses independently.
pub fn harvested_item(tile: Tile, data: u16) -> Option<crate::item::ItemId> {
    if tile != Tile::Flower230 {
        return None;
    }
    FlowerVariant::from_data(data).map(FlowerVariant::item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java230_flowers_place_with_distinct_ordinals_and_recover_exact_items() {
        let mut names = std::collections::HashSet::new();
        for variant in FlowerVariant::ALL {
            let item = variant.item();
            assert!(names.insert(item.display_name()));
            assert_eq!(FlowerVariant::from_item(item), Some(variant));
            let placed = crate::item::tile_placement(item, Tile::Grass).unwrap();
            assert_eq!(placed, (Tile::Flower230, variant as u16));
            assert_eq!(harvested_item(placed.0, placed.1), Some(item));
            assert_eq!(crate::item::tile_placement(item, Tile::Dirt), None);
        }
        assert_eq!(FlowerVariant::from_data(19), None);
        assert_eq!(harvested_item(Tile::Flower230, 19), None);
        for data in [0, 1, 16, 17, 19] {
            assert_eq!(harvested_item(Tile::Flower, data), None);
        }
        assert_eq!(Tile::Flower.id(), 2);
    }

    #[test]
    fn flower_reports_grass_family_but_has_no_center_checker() {
        assert!(connects_to_grass());
        assert!(!connects(Tile::Grass));
        assert!(!connects(Tile::Path));
    }

    #[test]
    fn flower_renders_over_grass() {
        assert_eq!(underlay(), [Some(Tile::Dirt), Some(Tile::Grass)]);
        assert_eq!(VARIANT_COUNT, 19);
    }
}
