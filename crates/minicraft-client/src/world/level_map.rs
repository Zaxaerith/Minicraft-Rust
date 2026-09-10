//! Per-level tile storage, independent of World orchestration.
use serde::{Deserialize, Serialize};

use super::Tile;
use super::entity::EntityArena;
use super::spawn::SpawnIntent;

#[derive(Serialize, Deserialize)]
pub struct Level {
    pub depth: i8,
    pub tiles: Vec<Tile>,
    pub data: Vec<u16>,
    pub max_mob_count: usize,
    pub pending_spawns: Vec<SpawnIntent>,
    pub entities: EntityArena,
}

#[allow(dead_code)] // Public Level API for World/render adapters during stage 8+.
impl Level {
    pub fn get_tile(&self, width: usize, x: usize, y: usize) -> Tile {
        self.tiles[x + y * width]
    }

    pub fn get_data(&self, width: usize, x: usize, y: usize) -> u16 {
        self.data[x + y * width]
    }

    pub fn set_tile(&mut self, width: usize, x: usize, y: usize, tile: Tile) {
        self.tiles[x + y * width] = tile;
    }

    pub fn set_data(&mut self, width: usize, x: usize, y: usize, data: u16) {
        self.data[x + y * width] = data;
    }

    pub fn tile_at_pixel(&self, width: usize, x: i32, y: i32) -> Tile {
        let tx = (x >> 4).max(0) as usize;
        let ty = (y >> 4).max(0) as usize;
        self.get_tile(width, tx.min(width.saturating_sub(1)), ty)
    }
}
