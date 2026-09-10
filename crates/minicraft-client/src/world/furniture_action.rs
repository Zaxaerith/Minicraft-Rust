//! Java Furniture place/use helpers (Chest/Bed/TNT/spawner placement).
use super::{FurnitureKind, GameMode, ItemId, TILE_SIZE, Tile, World};

pub(super) fn furniture_for_item(item: ItemId) -> Option<FurnitureKind> {
    Some(match item {
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

impl World {
    pub(super) fn place_furniture(
        &mut self,
        kind: FurnitureKind,
        item: ItemId,
        offset_x: i32,
        offset_y: i32,
    ) -> bool {
        let tile_x = (self.player.x + offset_x) / TILE_SIZE;
        let tile_y = (self.player.y + offset_y) / TILE_SIZE;
        if tile_x < 0 || tile_y < 0 || tile_x >= self.width as i32 || tile_y >= self.height as i32 {
            return false;
        }
        let index = tile_x as usize + tile_y as usize * self.width;
        let tile = self.levels[self.current_level].tiles[index];
        if tile.solid(self.levels[self.current_level].data[index])
            || matches!(
                tile,
                Tile::Water
                    | Tile::Lava
                    | Tile::Hole
                    | Tile::InfiniteFall
                    | Tile::StairsUp
                    | Tile::StairsDown
            )
        {
            return false;
        }
        let x = tile_x * TILE_SIZE + 8;
        let y = tile_y * TILE_SIZE + 8;
        if self.levels[self.current_level]
            .entities
            .furniture_near(x, y, 8)
            .is_some()
        {
            return false;
        }
        if self.mode != GameMode::Creative && !self.player.inventory.remove(item, 1) {
            self.player.active_item = None;
            return false;
        }
        self.levels[self.current_level]
            .entities
            .spawn_furniture(kind, x, y);
        if self.mode != GameMode::Creative && self.player.inventory.count(item) == 0 {
            self.player.active_item = None;
        }
        true
    }
}
