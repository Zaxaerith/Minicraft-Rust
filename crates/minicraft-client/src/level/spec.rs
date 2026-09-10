//! World creation parameters and game modes shared by saves and generation.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Normal,
    Forest,
    Desert,
    Plain,
    Hell,
}

impl Theme {
    pub fn from_index(index: usize) -> Self {
        [
            Self::Normal,
            Self::Forest,
            Self::Desert,
            Self::Plain,
            Self::Hell,
        ]
        .get(index)
        .copied()
        .unwrap_or(Self::Normal)
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainType {
    Island,
    Box,
    Mountain,
    Irregular,
}

impl TerrainType {
    pub fn from_index(index: usize) -> Self {
        [Self::Island, Self::Box, Self::Mountain, Self::Irregular]
            .get(index)
            .copied()
            .unwrap_or(Self::Island)
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSpec {
    pub size: usize,
    pub theme: Theme,
    pub terrain: TerrainType,
}

impl Default for WorldSpec {
    fn default() -> Self {
        Self {
            size: 128,
            theme: Theme::Normal,
            terrain: TerrainType::Island,
        }
    }
}

impl WorldSpec {
    pub fn new(size: usize, theme: usize, terrain: usize) -> Self {
        Self {
            size: match size {
                128 | 256 | 512 => size,
                _ => 128,
            },
            theme: Theme::from_index(theme),
            terrain: TerrainType::from_index(terrain),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Hardcore,
    Score,
}

impl GameMode {
    pub const fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Creative,
            2 => Self::Hardcore,
            3 => Self::Score,
            _ => Self::Survival,
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}
