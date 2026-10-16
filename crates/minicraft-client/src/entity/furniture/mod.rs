//! Java entity/furniture/* — FurnitureKind plus tick/push/place helpers.
use crate::entity::mob::NaturalMob;
use serde::{Deserialize, Serialize};

pub mod bed;
pub mod chest;
pub mod composter;
pub mod spawner;
pub mod statue;
pub mod tnt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FurnitureKind {
    Workbench,
    Oven,
    Furnace,
    Anvil,
    Enchanter,
    Loom,
    Chest,
    DungeonChest,
    Lantern,
    IronLantern,
    GoldLantern,
    Tnt,
    Bed,
    Composter,
    KnightStatue,
    CowSpawner,
    PigSpawner,
    SheepSpawner,
    SlimeSpawner,
    ZombieSpawner,
    CreeperSpawner,
    SkeletonSpawner,
    SnakeSpawner,
    KnightSpawner,
    DyeVat,
}

impl FurnitureKind {
    pub const ALL: [Self; 25] = [
        Self::Workbench,
        Self::Oven,
        Self::Furnace,
        Self::Anvil,
        Self::Enchanter,
        Self::Loom,
        Self::Chest,
        Self::DungeonChest,
        Self::Lantern,
        Self::IronLantern,
        Self::GoldLantern,
        Self::Tnt,
        Self::Bed,
        Self::Composter,
        Self::KnightStatue,
        Self::CowSpawner,
        Self::PigSpawner,
        Self::SheepSpawner,
        Self::SlimeSpawner,
        Self::ZombieSpawner,
        Self::CreeperSpawner,
        Self::SkeletonSpawner,
        Self::SnakeSpawner,
        Self::KnightSpawner,
        Self::DyeVat,
    ];

    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::DyeVat => "dyevat",
            Self::Workbench => "workbench",
            Self::Oven => "oven",
            Self::Furnace => "furnace",
            Self::Anvil => "anvil",
            Self::Enchanter => "enchanter",
            Self::Loom => "loom",
            Self::Chest => "chest",
            Self::DungeonChest => "dungeon_chest",
            Self::Lantern => "lantern",
            Self::IronLantern => "iron_lantern",
            Self::GoldLantern => "gold_lantern",
            Self::Tnt => "tnt",
            Self::Bed => "bed",
            Self::Composter => "composter",
            Self::KnightStatue => "knight_statue",
            Self::CowSpawner
            | Self::PigSpawner
            | Self::SheepSpawner
            | Self::SlimeSpawner
            | Self::ZombieSpawner
            | Self::CreeperSpawner
            | Self::SkeletonSpawner
            | Self::SnakeSpawner
            | Self::KnightSpawner => "spawner",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::DyeVat => "Dye Vat",
            Self::Workbench => "Workbench",
            Self::Oven => "Oven",
            Self::Furnace => "Furnace",
            Self::Anvil => "Anvil",
            Self::Enchanter => "Enchanter",
            Self::Loom => "Loom",
            Self::Chest => "Chest",
            Self::DungeonChest => "Dungeon Chest",
            Self::Lantern => "Lantern",
            Self::IronLantern => "Iron Lantern",
            Self::GoldLantern => "Gold Lantern",
            Self::Tnt => "TNT",
            Self::Bed => "Bed",
            Self::Composter => "Composter",
            Self::KnightStatue => "Knight Statue",
            Self::CowSpawner => "Cow Spawner",
            Self::PigSpawner => "Pig Spawner",
            Self::SheepSpawner => "Sheep Spawner",
            Self::SlimeSpawner => "Slime Spawner",
            Self::ZombieSpawner => "Zombie Spawner",
            Self::CreeperSpawner => "Creeper Spawner",
            Self::SkeletonSpawner => "Skeleton Spawner",
            Self::SnakeSpawner => "Snake Spawner",
            Self::KnightSpawner => "Knight Spawner",
        }
    }

    pub const fn crafting(self) -> bool {
        matches!(
            self,
            Self::Workbench
                | Self::Oven
                | Self::Furnace
                | Self::Anvil
                | Self::Enchanter
                | Self::Loom
                | Self::DyeVat
        )
    }

    pub const fn spawner_mob(self) -> Option<NaturalMob> {
        match self {
            Self::CowSpawner => Some(NaturalMob::Cow),
            Self::PigSpawner => Some(NaturalMob::Pig),
            Self::SheepSpawner => Some(NaturalMob::Sheep),
            Self::SlimeSpawner => Some(NaturalMob::Slime),
            Self::ZombieSpawner => Some(NaturalMob::Zombie),
            Self::CreeperSpawner => Some(NaturalMob::Creeper),
            Self::SkeletonSpawner => Some(NaturalMob::Skeleton),
            Self::SnakeSpawner => Some(NaturalMob::Snake),
            Self::KnightSpawner => Some(NaturalMob::Knight),
            _ => None,
        }
    }

    pub const fn light_radius(self) -> i32 {
        match self {
            Self::Lantern => 8,
            Self::IronLantern => 12,
            Self::GoldLantern => 15,
            _ => 0,
        }
    }

    /// Java Entity xr/yr collision half-extents (Furniture constructors).
    pub const fn collision_radius(self) -> (i32, i32) {
        match self {
            Self::DyeVat => (0, 0),
            Self::Workbench | Self::Oven | Self::Furnace | Self::Anvil => (3, 2),
            Self::Enchanter | Self::Loom => (7, 2),
            Self::Tnt
            | Self::Bed
            | Self::Lantern
            | Self::IronLantern
            | Self::GoldLantern
            | Self::KnightStatue => (3, 2),
            // Default Furniture / Chest / Spawner use xr=3, yr=3.
            _ => (3, 3),
        }
    }
}
