//! Java `TileItem` — placement rules for terrain items onto target tiles.
use super::Tile;
use crate::item::ItemId;

pub fn tile_placement(item: ItemId, target: Tile) -> Option<(Tile, u16)> {
    let fluid_or_hole = matches!(target, Tile::Hole | Tile::Water | Tile::Lava);
    let floor_target = matches!(
        target,
        Tile::Dirt
            | Tile::Grass
            | Tile::Sand
            | Tile::Path
            | Tile::WoodFloor
            | Tile::StoneFloor
            | Tile::ObsidianFloor
            | Tile::RawStone
            | Tile::RawObsidian
            | Tile::OrnateStone
            | Tile::OrnateObsidian
            | Tile::WhiteWool
            | Tile::RedWool
            | Tile::BlueWool
            | Tile::GreenWool
            | Tile::YellowWool
            | Tile::BlackWool
    );
    Some(match item {
        ItemId::Flower if target == Tile::Grass => (Tile::Flower, 0),
        ItemId::Acorn if target == Tile::Grass => (Tile::TreeSapling, 0),
        ItemId::Dirt if fluid_or_hole => (Tile::Dirt, 0),
        ItemId::NaturalRock
            if fluid_or_hole
                || matches!(target, Tile::Dirt | Tile::Grass | Tile::Sand | Tile::Path) =>
        {
            (Tile::Rock, 0)
        }
        ItemId::Plank if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud) => {
            (Tile::WoodFloor, 0)
        }
        ItemId::PlankWall if target == Tile::WoodFloor => (Tile::WoodWall, 0),
        ItemId::WoodDoor if target == Tile::WoodFloor => (Tile::WoodDoor, 0),
        ItemId::WoodFence if floor_target => (Tile::WoodFence, target.id() as u16),
        ItemId::Stone if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) => {
            (Tile::RawStone, 0)
        }
        ItemId::StoneBrick
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::StoneFloor, 0)
        }
        ItemId::OrnateStone
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::OrnateStone, 0)
        }
        ItemId::StoneWall if target == Tile::StoneFloor => (Tile::StoneWall, 0),
        ItemId::StoneDoor if target == Tile::StoneFloor => (Tile::StoneDoor, 0),
        ItemId::StoneFence if floor_target => (Tile::StoneFence, target.id() as u16),
        ItemId::RawObsidian
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::RawObsidian, 0)
        }
        ItemId::ObsidianBrick
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::ObsidianFloor, 0)
        }
        ItemId::OrnateObsidian
            if matches!(target, Tile::Hole | Tile::Water | Tile::Cloud | Tile::Lava) =>
        {
            (Tile::OrnateObsidian, 0)
        }
        ItemId::ObsidianWall if target == Tile::ObsidianFloor => (Tile::ObsidianWall, 0),
        ItemId::ObsidianDoor if target == Tile::ObsidianFloor => (Tile::ObsidianDoor, 0),
        ItemId::ObsidianFence if floor_target => (Tile::ObsidianFence, target.id() as u16),
        ItemId::Wool if matches!(target, Tile::Hole | Tile::Water) => (Tile::WhiteWool, 0),
        ItemId::RedWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::RedWool, 0),
        ItemId::BlueWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::BlueWool, 0),
        ItemId::GreenWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::GreenWool, 0),
        ItemId::YellowWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::YellowWool, 0),
        ItemId::BlackWool if matches!(target, Tile::Hole | Tile::Water) => (Tile::BlackWool, 0),
        ItemId::Sand if fluid_or_hole => (Tile::Sand, 0),
        ItemId::Cactus if target == Tile::Sand => (Tile::CactusSapling, 0),
        ItemId::Cloud if target == Tile::InfiniteFall => (Tile::Cloud, 0),
        ItemId::WheatSeeds if target == Tile::Farmland => (Tile::Wheat, 0),
        ItemId::Potato if target == Tile::Farmland => (Tile::Potato, 0),
        ItemId::Carrot if target == Tile::Farmland => (Tile::Carrot, 0),
        ItemId::TomatoSeeds if target == Tile::Farmland => (Tile::Tomato, 0),
        ItemId::HeavenlyBerries if target == Tile::Farmland => (Tile::HeavenlyBerries, 0),
        ItemId::HellishBerries if target == Tile::Farmland => (Tile::HellishBerries, 0),
        ItemId::GrassSeeds if target == Tile::Dirt => (Tile::Grass, 0),
        ItemId::Torch if floor_target => (Tile::Torch, target.id() as u16),
        ItemId::Sign if floor_target => (Tile::Sign, target.id() as u16),
        ItemId::FarmlandItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Farmland, 0)
        }
        ItemId::HoleItem if matches!(target, Tile::Dirt | Tile::Grass) => (Tile::Hole, 0),
        ItemId::LavaItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Lava, 0)
        }
        ItemId::PathItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Path, 0)
        }
        ItemId::WaterItem if matches!(target, Tile::Dirt | Tile::Grass | Tile::Hole) => {
            (Tile::Water, 0)
        }
        _ => return None,
    })
}

/// Java TileItem instances: shows the placement frame and places via tile_placement.
pub fn is_tile_item(item: ItemId) -> bool {
    [
        Tile::Hole,
        Tile::Water,
        Tile::Lava,
        Tile::Grass,
        Tile::Sand,
        Tile::Dirt,
        Tile::Path,
        Tile::WoodFloor,
        Tile::StoneFloor,
        Tile::Farmland,
        Tile::Cloud,
        Tile::ObsidianFloor,
        Tile::WhiteWool,
        Tile::WoodWall,
        Tile::StoneWall,
    ]
    .iter()
    .any(|target| tile_placement(item, *target).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirt_fills_fluid_and_hole() {
        assert_eq!(
            tile_placement(ItemId::Dirt, Tile::Water),
            Some((Tile::Dirt, 0))
        );
        assert_eq!(
            tile_placement(ItemId::Dirt, Tile::Grass),
            None
        );
    }

    #[test]
    fn torch_stores_base_floor_in_data() {
        assert_eq!(
            tile_placement(ItemId::Torch, Tile::WoodFloor),
            Some((Tile::Torch, Tile::WoodFloor.id() as u16))
        );
        assert!(is_tile_item(ItemId::Torch));
        assert!(!is_tile_item(ItemId::Dirt) || tile_placement(ItemId::Dirt, Tile::Water).is_some());
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
