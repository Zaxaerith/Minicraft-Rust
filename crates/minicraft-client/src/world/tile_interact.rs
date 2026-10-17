//! Tile collision helpers and creative break; attack damage lives in World::attack.
use super::{GameMode, TILE_SIZE, Tile, World};

impl World {
    pub(super) fn can_stand(&self, x: i32, y: i32) -> bool {
        // Java Entity moveX/Y: tile solidity plus full furniture hitbox overlap.
        [(-4, -3), (4, -3), (-4, 3), (4, 3), (0, 0)]
            .into_iter()
            .all(|(offset_x, offset_y)| {
                let (tile, data) = self.tile_and_data_at_pixel(x + offset_x, y + offset_y);
                self.mode == GameMode::Creative && tile == Tile::InfiniteFall || !tile.solid(data)
            })
            && !self.levels[self.current_level]
                .entities
                .furniture_blocks_player(x, y)
    }

    pub(super) fn tile_at_pixel(&self, x: i32, y: i32) -> Tile {
        self.tile_and_data_at_pixel(x, y).0
    }

    /// Java Mob.isSwimming: water or lava under the entity origin.
    pub(super) fn player_is_swimming(&self) -> bool {
        matches!(
            self.tile_at_pixel(self.player.x, self.player.y),
            Tile::Water | Tile::Lava
        )
    }

    pub(super) fn tile_and_data_at_pixel(&self, x: i32, y: i32) -> (Tile, u16) {
        let tile_x = (x / TILE_SIZE).clamp(0, self.width as i32 - 1) as usize;
        let tile_y = (y / TILE_SIZE).clamp(0, self.height as i32 - 1) as usize;
        let index = tile_x + tile_y * self.width;
        (
            self.levels[self.current_level].tiles[index],
            self.levels[self.current_level].data[index],
        )
    }

    pub(super) fn apply_bumped_tile_damage(&mut self, x: i32, y: i32) {
        let damage = [(-4, -3), (4, -3), (-4, 4), (4, 4)]
            .into_iter()
            .map(|(offset_x, offset_y)| self.tile_at_pixel(x + offset_x, y + offset_y))
            .map(|tile| match tile {
                Tile::Cactus => crate::level::tile::cactus::bump_damage(
                    u8::try_from(self.difficulty.min(2)).unwrap_or(0),
                ),
                Tile::LavaBrick => crate::level::tile::lava_brick::BUMP_DAMAGE,
                _ => 0,
            })
            .max()
            .unwrap_or(0);
        if damage > 0 {
            self.hurt_player(damage, false);
        }
    }

    pub(super) fn creative_break_tile(&mut self, index: usize) -> bool {
        let tile = self.levels[self.current_level].tiles[index];
        let data = self.levels[self.current_level].data[index];
        let replacement = match tile {
            Tile::Tree | Tile::TreeSapling | Tile::Flower | Tile::Flower230 => Tile::Grass,
            wool if crate::level::tile::wool::is_wool(wool) => Tile::Hole,
            Tile::Cactus | Tile::CactusSapling => Tile::Sand,
            Tile::Rock
            | Tile::HardRock
            | Tile::IronOre
            | Tile::GoldOre
            | Tile::GemOre
            | Tile::LapisOre => Tile::Dirt,
            Tile::CloudOre => Tile::Cloud,
            Tile::OrnateWood => {
                if self.levels[self.current_level].depth == 1 {
                    Tile::Cloud
                } else {
                    Tile::Hole
                }
            }
            Tile::Wheat
            | Tile::Potato
            | Tile::Tomato
            | Tile::Carrot
            | Tile::HeavenlyBerries
            | Tile::HellishBerries => Tile::Farmland,
            Tile::WoodWall => Tile::WoodFloor,
            Tile::StoneWall => Tile::StoneFloor,
            Tile::ObsidianWall => Tile::ObsidianFloor,
            Tile::WoodDoor => Tile::WoodFloor,
            Tile::StoneDoor => Tile::StoneFloor,
            Tile::ObsidianDoor => Tile::ObsidianFloor,
            Tile::Torch | Tile::Sign | Tile::WoodFence | Tile::StoneFence | Tile::ObsidianFence => {
                Tile::from_id(data as u8).unwrap_or(Tile::Dirt)
            }
            _ => return false,
        };
        self.levels[self.current_level].tiles[index] = replacement;
        self.levels[self.current_level].data[index] = 0;
        if tile == Tile::Sign {
            self.signs[self.current_level].remove(&index);
        }
        true
    }
}
