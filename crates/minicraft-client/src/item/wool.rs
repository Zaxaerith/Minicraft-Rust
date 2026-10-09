//! Java `WoolItem` — wool tiles placed from colored wool items.
use super::ItemId;
use crate::level::tile::Tile;

/// Java `WoolItem` valid placement targets: hole / water.
pub const VALID_TILES: [Tile; 2] = [Tile::Hole, Tile::Water];

/// Colored wool ItemId → placed tile (Java `TileModel(name)`).
pub const fn wool_tile(item: ItemId) -> Option<Tile> {
    match item {
        ItemId::Wool => Some(Tile::WhiteWool),
        ItemId::RedWool => Some(Tile::RedWool),
        ItemId::BlueWool => Some(Tile::BlueWool),
        ItemId::GreenWool => Some(Tile::GreenWool),
        ItemId::YellowWool => Some(Tile::YellowWool),
        ItemId::BlackWool => Some(Tile::BlackWool),
        _ => None,
    }
}

/// Java `WoolItem.getAllInstances` (white plus dyed variants present in the port).
pub const ALL: [ItemId; 6] = [
    ItemId::Wool,
    ItemId::RedWool,
    ItemId::BlueWool,
    ItemId::GreenWool,
    ItemId::YellowWool,
    ItemId::BlackWool,
];

/// Placement result when `item` is used on `target`.
pub fn placement(item: ItemId, target: Tile) -> Option<(Tile, u16)> {
    let tile = wool_tile(item)?;
    VALID_TILES.contains(&target).then_some((tile, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

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
