//! Java ArmorItem levels and durability.
use serde::{Deserialize, Serialize};

use super::ItemId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ArmorKind {
    Leather,
    Snake,
    Iron,
    Gold,
    Gem,
}

impl ArmorKind {
    pub const fn level(self) -> u8 {
        self as u8 + 1
    }

    pub const fn durability(self) -> u8 {
        match self {
            Self::Leather => 30,
            Self::Snake => 40,
            Self::Iron => 50,
            Self::Gold => 70,
            Self::Gem => 100,
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Leather => "Leather",
            Self::Snake => "Snake",
            Self::Iron => "Iron",
            Self::Gold => "Gold",
            Self::Gem => "Gem",
        }
    }

    pub const fn item(self) -> ItemId {
        match self {
            Self::Leather => ItemId::LeatherArmor,
            Self::Snake => ItemId::SnakeArmor,
            Self::Iron => ItemId::IronArmor,
            Self::Gold => ItemId::GoldArmor,
            Self::Gem => ItemId::GemArmor,
        }
    }
}
