//! Legacy path kept for M4 consumers; Java `TileItem` lives in `crate::item`.
//!
//! Prefer `crate::item::{is_tile_item, tile_placement}` in new code.
pub use crate::item::tile_item::{is_tile_item, tile_placement};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemId;
    use crate::level::tile::Tile;

    #[test]
    fn dirt_fills_fluid_and_hole() {
        assert_eq!(
            tile_placement(ItemId::Dirt, Tile::Water),
            Some((Tile::Dirt, 0))
        );
        assert_eq!(tile_placement(ItemId::Dirt, Tile::Grass), None);
    }

    #[test]
    fn torch_stores_base_floor_in_data() {
        assert_eq!(
            tile_placement(ItemId::Torch, Tile::WoodFloor),
            Some((Tile::Torch, Tile::WoodFloor.id() as u16))
        );
        assert!(is_tile_item(ItemId::Torch));
    }

    #[test]
    fn crops_require_farmland() {
        assert_eq!(
            tile_placement(ItemId::WheatSeeds, Tile::Farmland),
            Some((Tile::Wheat, 0))
        );
        assert_eq!(tile_placement(ItemId::WheatSeeds, Tile::Dirt), None);
    }
}
