//! Java entity/mob/* — species IDs plus AI tick split by Java class.
use serde::{Deserialize, Serialize};

pub mod air_wizard;
pub mod ai;
pub mod creeper;
pub mod obsidian_knight;
pub mod skeleton;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NaturalMob {
    Slime,
    Zombie,
    Creeper,
    Skeleton,
    Snake,
    Knight,
    Cow,
    Pig,
    Sheep,
    AirWizard,
    ObsidianKnight,
}

impl NaturalMob {
    pub const ALL: [Self; 11] = [
        Self::Slime,
        Self::Zombie,
        Self::Creeper,
        Self::Skeleton,
        Self::Snake,
        Self::Knight,
        Self::Cow,
        Self::Pig,
        Self::Sheep,
        Self::AirWizard,
        Self::ObsidianKnight,
    ];

    pub const fn hostile(self) -> bool {
        !matches!(self, Self::Cow | Self::Pig | Self::Sheep)
    }

    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::Slime => "slime",
            Self::Zombie => "zombie",
            Self::Creeper => "creeper",
            Self::Skeleton => "skeleton",
            Self::Snake => "snake",
            Self::Knight => "knight",
            Self::Cow => "cow",
            Self::Pig => "pig",
            Self::Sheep => "sheep",
            Self::AirWizard => "air_wizard",
            Self::ObsidianKnight => "obsidian_knight",
        }
    }
}
