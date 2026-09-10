//! Java ToolType, ToolTier, and ToolItem durability/damage formulas.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)] // The remaining variants are registered before their station recipes land.
pub enum ToolKind {
    Shovel,
    Hoe,
    Sword,
    Pickaxe,
    Axe,
    Bow,
    Claymore,
    Shears,
}

impl ToolKind {
    pub const ALL: [Self; 8] = [
        Self::Shovel,
        Self::Hoe,
        Self::Sword,
        Self::Pickaxe,
        Self::Axe,
        Self::Bow,
        Self::Claymore,
        Self::Shears,
    ];

    pub const fn asset_name(self, tier: ToolTier) -> &'static str {
        match (self, tier) {
            (Self::Shears, _) => "shears",
            (Self::Shovel, ToolTier::Wood) => "wooden_shovel",
            (Self::Shovel, ToolTier::Rock) => "stone_shovel",
            (Self::Shovel, ToolTier::Iron) => "iron_shovel",
            (Self::Shovel, ToolTier::Gold) => "gold_shovel",
            (Self::Shovel, ToolTier::Gem) => "gem_shovel",
            (Self::Hoe, ToolTier::Wood) => "wooden_hoe",
            (Self::Hoe, ToolTier::Rock) => "stone_hoe",
            (Self::Hoe, ToolTier::Iron) => "iron_hoe",
            (Self::Hoe, ToolTier::Gold) => "gold_hoe",
            (Self::Hoe, ToolTier::Gem) => "gem_hoe",
            (Self::Sword, ToolTier::Wood) => "wooden_sword",
            (Self::Sword, ToolTier::Rock) => "stone_sword",
            (Self::Sword, ToolTier::Iron) => "iron_sword",
            (Self::Sword, ToolTier::Gold) => "gold_sword",
            (Self::Sword, ToolTier::Gem) => "gem_sword",
            (Self::Pickaxe, ToolTier::Wood) => "wooden_pickaxe",
            (Self::Pickaxe, ToolTier::Rock) => "stone_pickaxe",
            (Self::Pickaxe, ToolTier::Iron) => "iron_pickaxe",
            (Self::Pickaxe, ToolTier::Gold) => "gold_pickaxe",
            (Self::Pickaxe, ToolTier::Gem) => "gem_pickaxe",
            (Self::Axe, ToolTier::Wood) => "wooden_axe",
            (Self::Axe, ToolTier::Rock) => "stone_axe",
            (Self::Axe, ToolTier::Iron) => "iron_axe",
            (Self::Axe, ToolTier::Gold) => "gold_axe",
            (Self::Axe, ToolTier::Gem) => "gem_axe",
            (Self::Bow, ToolTier::Wood) => "wooden_bow",
            (Self::Bow, ToolTier::Rock) => "stone_bow",
            (Self::Bow, ToolTier::Iron) => "iron_bow",
            (Self::Bow, ToolTier::Gold) => "gold_bow",
            (Self::Bow, ToolTier::Gem) => "gem_bow",
            (Self::Claymore, ToolTier::Wood) => "wooden_claymore",
            (Self::Claymore, ToolTier::Rock) => "stone_claymore",
            (Self::Claymore, ToolTier::Iron) => "iron_claymore",
            (Self::Claymore, ToolTier::Gold) => "gold_claymore",
            (Self::Claymore, ToolTier::Gem) => "gem_claymore",
        }
    }

    pub const fn base_durability(self) -> u16 {
        match self {
            Self::Shovel => 34,
            Self::Hoe | Self::Bow => 30,
            Self::Sword => 52,
            Self::Pickaxe => 38,
            Self::Axe => 34,
            Self::Claymore => 44,
            Self::Shears => 42,
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Shovel => "Shovel",
            Self::Hoe => "Hoe",
            Self::Sword => "Sword",
            Self::Pickaxe => "Pickaxe",
            Self::Axe => "Axe",
            Self::Bow => "Bow",
            Self::Claymore => "Claymore",
            Self::Shears => "Shears",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[allow(dead_code)] // Higher tiers are consumed by the upcoming anvil/enchanter lists.
pub enum ToolTier {
    Wood = 0,
    Rock = 1,
    Iron = 2,
    Gold = 3,
    Gem = 4,
}

impl ToolTier {
    pub const ALL: [Self; 5] = [Self::Wood, Self::Rock, Self::Iron, Self::Gold, Self::Gem];

    pub const fn level(self) -> u8 {
        self as u8
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Wood => "Wood",
            Self::Rock => "Rock",
            Self::Iron => "Iron",
            Self::Gold => "Gold",
            Self::Gem => "Gem",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolItem {
    pub kind: ToolKind,
    pub tier: ToolTier,
    pub durability: u16,
    pub max_durability: u16,
}

impl ToolItem {
    pub const fn new(kind: ToolKind, tier: ToolTier) -> Self {
        let max_durability = if matches!(kind, ToolKind::Shears) {
            kind.base_durability()
        } else {
            kind.base_durability() * (tier.level() as u16 + 1)
        };
        Self {
            kind,
            tier,
            durability: max_durability,
            max_durability,
        }
    }

    pub fn display_name(self) -> String {
        if self.kind == ToolKind::Shears {
            self.kind.display_name().to_owned()
        } else {
            format!("{} {}", self.tier.display_name(), self.kind.display_name())
        }
    }

    pub fn is_depleted(self) -> bool {
        self.durability == 0
    }

    pub fn pay_durability(&mut self) -> bool {
        if self.is_depleted() {
            return false;
        }
        self.durability -= 1;
        true
    }

    /// Java ToolItem.getDamage: tier * 5 + 10 plus a roll from 0 through 4.
    pub fn terrain_damage(self, roll: u8) -> u16 {
        self.tier.level() as u16 * 5 + 10 + roll.min(4) as u16
    }

    /// Java ToolItem melee bonus. The caller supplies the already bounded roll.
    pub fn melee_bonus(self, roll: u8) -> u8 {
        let level = self.tier.level();
        match self.kind {
            ToolKind::Axe => (level + 1) * 2 + roll.min(3),
            ToolKind::Sword => (level + 1) * 3 + roll.min(1 + level * level),
            ToolKind::Claymore => (level + 1) * 3 + roll.min(3 + level * level * 3),
            ToolKind::Pickaxe => level + 1 + roll.min(1),
            ToolKind::Shears => 0,
            _ => 1,
        }
    }
}
