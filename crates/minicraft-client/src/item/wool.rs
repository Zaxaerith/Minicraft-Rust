//! Java `WoolItem` — wool tiles placed from colored wool items.
use super::ItemId;
use crate::level::tile::Tile;

/// Java `WoolItem` valid placement targets: hole / water.
pub const VALID_TILES: [Tile; 2] = [Tile::Hole, Tile::Water];

/// Colored wool ItemId → placed tile (Java `TileModel(name)`).
pub const fn wool_tile(item: ItemId) -> Option<Tile> {
    let color = match super::dye::DyeColor::from_wool_item(item) {
        Some(color) => color,
        None => return None,
    };
    Some(match color {
        super::dye::DyeColor::Black => Tile::BlackWool,
        super::dye::DyeColor::Red => Tile::RedWool,
        super::dye::DyeColor::Green => Tile::GreenWool,
        super::dye::DyeColor::Brown => Tile::BrownWool,
        super::dye::DyeColor::Blue => Tile::BlueWool,
        super::dye::DyeColor::Purple => Tile::PurpleWool,
        super::dye::DyeColor::Cyan => Tile::CyanWool,
        super::dye::DyeColor::LightGray => Tile::LightGrayWool,
        super::dye::DyeColor::Gray => Tile::GrayWool,
        super::dye::DyeColor::Pink => Tile::PinkWool,
        super::dye::DyeColor::Lime => Tile::LimeWool,
        super::dye::DyeColor::Yellow => Tile::YellowWool,
        super::dye::DyeColor::LightBlue => Tile::LightBlueWool,
        super::dye::DyeColor::Magenta => Tile::MagentaWool,
        super::dye::DyeColor::Orange => Tile::OrangeWool,
        super::dye::DyeColor::White => Tile::WhiteWool,
    })
}

/// Java `WoolItem.getAllInstances` in `DyeColor` ordinal order.
pub const ALL: [ItemId; 16] = [
    ItemId::BlackWool,
    ItemId::RedWool,
    ItemId::GreenWool,
    ItemId::BrownWool,
    ItemId::BlueWool,
    ItemId::PurpleWool,
    ItemId::CyanWool,
    ItemId::LightGrayWool,
    ItemId::GrayWool,
    ItemId::PinkWool,
    ItemId::LimeWool,
    ItemId::YellowWool,
    ItemId::LightBlueWool,
    ItemId::MagentaWool,
    ItemId::OrangeWool,
    ItemId::Wool,
];

/// Recover the exact colored wool item when shearing its tile.
pub const fn item_for_tile(tile: Tile) -> Option<ItemId> {
    match tile {
        Tile::BlackWool => Some(ItemId::BlackWool),
        Tile::RedWool => Some(ItemId::RedWool),
        Tile::GreenWool => Some(ItemId::GreenWool),
        Tile::BrownWool => Some(ItemId::BrownWool),
        Tile::BlueWool => Some(ItemId::BlueWool),
        Tile::PurpleWool => Some(ItemId::PurpleWool),
        Tile::CyanWool => Some(ItemId::CyanWool),
        Tile::LightGrayWool => Some(ItemId::LightGrayWool),
        Tile::GrayWool => Some(ItemId::GrayWool),
        Tile::PinkWool => Some(ItemId::PinkWool),
        Tile::LimeWool => Some(ItemId::LimeWool),
        Tile::YellowWool => Some(ItemId::YellowWool),
        Tile::LightBlueWool => Some(ItemId::LightBlueWool),
        Tile::MagentaWool => Some(ItemId::MagentaWool),
        Tile::OrangeWool => Some(ItemId::OrangeWool),
        Tile::WhiteWool => Some(ItemId::Wool),
        _ => None,
    }
}

/// Placement result when `item` is used on `target`.
pub fn placement(item: ItemId, target: Tile) -> Option<(Tile, u16)> {
    let tile = wool_tile(item)?;
    VALID_TILES.contains(&target).then_some((tile, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java230_all_wool_colors_place_and_recover_exact_item() {
        let mut tiles = std::collections::HashSet::new();
        for color in crate::item::dye::DyeColor::ALL {
            let item = color.wool_item();
            let (tile, data) = placement(item, Tile::Water).unwrap();
            assert_eq!(data, 0);
            assert_eq!(item_for_tile(tile), Some(item));
            assert!(tiles.insert(tile.id()));
            assert_eq!(Tile::from_id(tile.id()), Some(tile));
            assert!(crate::level::tile::wool::is_wool(tile));
            assert_eq!(
                super::super::tile_placement(ItemId::Torch, tile),
                Some((Tile::Torch, tile.id() as u16))
            );
        }
        assert_eq!(tiles.len(), 16);
        assert_eq!(Tile::WhiteWool.id(), 35);
        assert_eq!(Tile::BlackWool.id(), 41);
        assert_eq!(ItemId::Wool.asset_name(), "wool");
    }

    #[test]
    fn wool_places_on_hole_and_water_only() {
        assert_eq!(
            placement(ItemId::RedWool, Tile::Hole),
            Some((Tile::RedWool, 0))
        );
        assert_eq!(
            placement(ItemId::Wool, Tile::Water),
            Some((Tile::WhiteWool, 0))
        );
        assert_eq!(placement(ItemId::RedWool, Tile::Grass), None);
    }
}
