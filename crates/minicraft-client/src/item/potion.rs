//! Java PotionType kinds and durations (durations live on World potion timers).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PotionKind {
    Awkward,
    Speed,
    Light,
    Swim,
    Energy,
    Regen,
    Health,
    Time,
    Lava,
    Shield,
    Haste,
    Escape,
}

impl PotionKind {
    pub const ALL: [Self; 12] = [
        Self::Awkward,
        Self::Speed,
        Self::Light,
        Self::Swim,
        Self::Energy,
        Self::Regen,
        Self::Health,
        Self::Time,
        Self::Lava,
        Self::Shield,
        Self::Haste,
        Self::Escape,
    ];

    pub const fn id(self) -> usize {
        self as usize
    }

    pub const fn duration(self) -> u16 {
        match self {
            Self::Awkward | Self::Health | Self::Escape => 0,
            Self::Speed => 4_200,
            Self::Light => 6_000,
            Self::Swim | Self::Haste => 4_800,
            Self::Energy => 8_400,
            Self::Regen | Self::Time => 1_800,
            Self::Lava => 7_200,
            Self::Shield => 5_400,
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Awkward => "Awkward",
            Self::Speed => "Speed",
            Self::Light => "Light",
            Self::Swim => "Swim",
            Self::Energy => "Energy",
            Self::Regen => "Regen",
            Self::Health => "Health",
            Self::Time => "Time",
            Self::Lava => "Lava",
            Self::Shield => "Shield",
            Self::Haste => "Haste",
            Self::Escape => "Escape",
        }
    }
}
