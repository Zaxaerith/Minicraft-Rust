//! Independent finite-memory, sparse-disk Infdev experiment. Coordinates follow
//! upstream 64x64 floorDiv/floorMod; terrain and storage are explicitly versioned
//! Rust prototypes. No fixed World/Level/GenerationVersion interpretation changes.
mod generation;
pub mod paged;
mod store;
pub mod terrain;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, path::PathBuf};

pub use generation::height;
pub const CHUNK_SIZE: usize = 64;
pub const NAMESPACE: &str = "infdev4a-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    pub fn at(x: i32, y: i32) -> Self {
        Self {
            x: x.div_euclid(CHUNK_SIZE as i32),
            y: y.div_euclid(CHUNK_SIZE as i32),
        }
    }
    pub fn local_index(x: i32, y: i32) -> usize {
        x.rem_euclid(CHUNK_SIZE as i32) as usize
            + y.rem_euclid(CHUNK_SIZE as i32) as usize * CHUNK_SIZE
    }
    fn valid(self) -> bool {
        let bound = i32::MAX / CHUNK_SIZE as i32;
        (-bound - 1..=bound).contains(&self.x) && (-bound - 1..=bound).contains(&self.y)
    }
}

/// Experimental IDs, unrelated to stable game Tile IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ground {
    Grass,
    Water,
    Sand,
    Rock,
    Tree,
    Flower,
    Cactus,
    StairsDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ground: Ground,
    pub data: u16,
}

struct Resident {
    cells: Vec<Cell>,
    dirty: bool,
    touched: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChunkObject {
    hash: u64,
    slot: u16,
}

pub struct InfdevWorld {
    directory: PathBuf,
    // Exclusive OS sharing lock; a crash releases it without a stale lock lease.
    _lock: File,
    seed: i64,
    position: (i32, i32),
    committed: BTreeMap<ChunkCoord, ChunkObject>,
    resident: BTreeMap<ChunkCoord, Resident>,
    capacity: usize,
    clock: u64,
    recovered: bool,
}

impl InfdevWorld {
    pub fn directory(&self) -> &std::path::Path {
        &self.directory
    }
    pub fn seed(&self) -> i64 {
        self.seed
    }
    pub fn position(&self) -> (i32, i32) {
        self.position
    }
    pub fn resident_count(&self) -> usize {
        self.resident.len()
    }
    pub fn saved_chunk_count(&self) -> usize {
        self.committed.len()
    }
    /// A validated complete previous checkpoint was selected, never regenerated.
    pub fn recovered_backup(&self) -> bool {
        self.recovered
    }

    fn ensure(&mut self, coord: ChunkCoord) -> Result<(), String> {
        if !coord.valid() {
            return Err("chunk outside supported signed tile coordinates".into());
        }
        if !self.resident.contains_key(&coord) {
            let cells = match self.committed.get(&coord) {
                Some(object) => self.read_chunk(coord, *object)?,
                None => generation::generate(self.seed, coord),
            };
            if self.resident.len() == self.capacity {
                let oldest = *self
                    .resident
                    .iter()
                    .min_by_key(|(_, chunk)| chunk.touched)
                    .expect("capacity is nonzero")
                    .0;
                // Persist all dirty state before eviction. A failed write leaves
                // both the old resident chunk and the old player position intact.
                self.checkpoint()?;
                self.resident.remove(&oldest);
            }
            self.resident.insert(
                coord,
                Resident {
                    cells,
                    dirty: !self.committed.contains_key(&coord),
                    touched: 0,
                },
            );
        }
        self.clock = self.clock.wrapping_add(1);
        self.resident.get_mut(&coord).expect("loaded chunk").touched = self.clock;
        Ok(())
    }

    pub fn cell(&mut self, x: i32, y: i32) -> Result<Cell, String> {
        let coord = ChunkCoord::at(x, y);
        self.ensure(coord)?;
        Ok(self.resident[&coord].cells[ChunkCoord::local_index(x, y)])
    }

    pub fn set_cell(&mut self, x: i32, y: i32, cell: Cell) -> Result<(), String> {
        if cell.ground as u8 > 3 {
            return Err("legacy Phase4A supports only its original four ground IDs".into());
        }
        let coord = ChunkCoord::at(x, y);
        self.ensure(coord)?;
        let chunk = self.resident.get_mut(&coord).expect("loaded chunk");
        let old = &mut chunk.cells[ChunkCoord::local_index(x, y)];
        if *old != cell {
            *old = cell;
            chunk.dirty = true;
        }
        Ok(())
    }

    /// One exploration-cursor step, not Survival physics or collision. The
    /// destination chunk must load successfully before movement is committed.
    pub fn step(&mut self, dx: i32, dy: i32) -> Result<(), String> {
        if !matches!((dx, dy), (-1 | 1, 0) | (0, -1 | 1)) {
            return Err("one cardinal tile step required".into());
        }
        let x = self
            .position
            .0
            .checked_add(dx)
            .ok_or("x coordinate overflow")?;
        let y = self
            .position
            .1
            .checked_add(dy)
            .ok_or("y coordinate overflow")?;
        self.ensure(ChunkCoord::at(x, y))?;
        self.position = (x, y);
        Ok(())
    }

    pub fn unload_all(&mut self) -> Result<(), String> {
        self.checkpoint()?;
        self.resident.clear();
        Ok(())
    }
}
