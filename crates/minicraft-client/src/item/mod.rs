//! Java item catalog: ItemId, food/armor/potion/fishing lookups.
//! Submodules mirror Java packages: inventory, tool, recipe, armor, potion.
use std::fmt;

use serde::{Deserialize, Serialize};

pub mod armor;
pub mod crafting;
pub mod inventory;
pub mod potion;
pub mod recipe;
pub mod tool;

pub use armor::ArmorKind;
pub use crafting::{RecipeOutput, RecipeView, ordered_recipe_views, recipe_can_craft, recipe_views};
pub use inventory::{Inventory, ItemStack};
pub use potion::PotionKind;
pub use recipe::{
    ANVIL_RECIPES, ANVIL_TOOL_RECIPES, ENCHANTER_RECIPES, FURNACE_RECIPES, HAND_RECIPES,
    LOOM_RECIPES, OVEN_RECIPES, Recipe, ToolRecipe, WORKBENCH_STATION_RECIPES,
    WORKBENCH_TOOL_RECIPES,
};
pub use tool::{ToolItem, ToolKind, ToolTier};

pub const MAX_STACK: u16 = 999;
macro_rules! item_registry {
    ($( $variant:ident => ($asset:literal, $display:literal) ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum ItemId { $( $variant ),+ }

        impl ItemId {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn asset_name(self) -> &'static str {
                match self { $(Self::$variant => $asset),+ }
            }

            pub const fn display_name(self) -> &'static str {
                match self { $(Self::$variant => $display),+ }
            }
        }
    };
}

item_registry! {
    PowerGlove => ("missing_item", "Power Glove"),
    Wood => ("wood", "Wood"),
    Stone => ("stone", "Stone"),
    Coal => ("coal", "Coal"),
    Slime => ("slime", "Slime"),
    Cloth => ("cloth", "Cloth"),
    Leather => ("leather", "Leather"),
    RawBeef => ("beef", "Raw Beef"),
    RawPork => ("pork", "Raw Pork"),
    Wool => ("wool", "Wool"),
    Bone => ("bone", "Bone"),
    Arrow => ("arrow", "Arrow"),
    Gunpowder => ("gunpowder", "Gunpowder"),
    Scale => ("scale", "Scale"),
    Shard => ("shard", "Shard"),
    Plank => ("plank", "Plank"),
    Torch => ("torch", "Torch"),
    Workbench => ("workbench", "Workbench"),
    IronOre => ("iron_ore", "Iron Ore"),
    GoldOre => ("gold_ore", "Gold Ore"),
    Gem => ("gem", "Gem"),
    Lapis => ("lapis", "Lapis"),
    CloudOre => ("cloud_ore", "Cloud Ore"),
    IronIngot => ("iron_ingot", "Iron"),
    GoldIngot => ("gold_ingot", "Gold"),
    Oven => ("oven", "Oven"),
    Furnace => ("furnace", "Furnace"),
    Anvil => ("anvil", "Anvil"),
    Enchanter => ("enchanter", "Enchanter"),
    Loom => ("loom", "Loom"),
    String => ("string", "String"),
    CookedPork => ("cooked_pork", "Cooked Pork"),
    Steak => ("cooked_beef", "Steak"),
    ArcaneFertilizer => ("arcane_fertilizer", "Arcane Fertilizer"),
    Apple => ("apple", "Apple"),
    RawFish => ("fish", "Raw Fish"),
    Bread => ("bread", "Bread"),
    CookedFish => ("cooked_fish", "Cooked Fish"),
    GoldenApple => ("golden_apple", "Gold Apple"),
    Potato => ("potato", "Potato"),
    Tomato => ("tomato", "Tomato"),
    BakedPotato => ("baked_potato", "Baked Potato"),
    Wheat => ("wheat", "Wheat"),
    Key => ("key", "Key"),
    Rose => ("rose", "Rose"),
    Flower => ("oxeye_daisy", "Oxeye Daisy"),
    Cactus => ("cactus", "Cactus"),
    Sand => ("sand", "Sand"),
    Glass => ("glass", "Glass"),
    GlassBottle => ("glass_bottle", "Glass Bottle"),
    Fertilizer => ("fertilizer", "Fertilizer"),
    Dirt => ("dirt", "Dirt"),
    Acorn => ("acorn", "Acorn"),
    NaturalRock => ("stone", "Natural Rock"),
    Cloud => ("cloud", "Cloud"),
    PlankWall => ("plank_wall", "Plank Wall"),
    WoodDoor => ("wood_door", "Wood Door"),
    WoodFence => ("wood_fence", "Wood Fence"),
    StoneBrick => ("stone_brick", "Stone Brick"),
    OrnateStone => ("stone_brick", "Ornate Stone"),
    StoneWall => ("stone_wall", "Stone Wall"),
    StoneDoor => ("stone_door", "Stone Door"),
    StoneFence => ("stone_fence", "Stone Fence"),
    RawObsidian => ("obsidian", "Raw Obsidian"),
    ObsidianBrick => ("obsidian_brick", "Obsidian Brick"),
    OrnateObsidian => ("obsidian_brick", "Ornate Obsidian"),
    ObsidianWall => ("obsidian_wall", "Obsidian Wall"),
    ObsidianDoor => ("obsidian_door", "Obsidian Door"),
    ObsidianFence => ("obsidian_fence", "Obsidian Fence"),
    RedWool => ("red_wool", "Red Wool"),
    BlueWool => ("blue_wool", "Blue Wool"),
    GreenWool => ("green_wool", "Green Wool"),
    YellowWool => ("yellow_wool", "Yellow Wool"),
    BlackWool => ("black_wool", "Black Wool"),
    RedClothes => ("red_clothes", "Red Clothes"),
    BlueClothes => ("blue_clothes", "Blue Clothes"),
    GreenClothes => ("green_clothes", "Green Clothes"),
    YellowClothes => ("yellow_clothes", "Yellow Clothes"),
    BlackClothes => ("black_clothes", "Black Clothes"),
    OrangeClothes => ("orange_clothes", "Orange Clothes"),
    PurpleClothes => ("purple_clothes", "Purple Clothes"),
    CyanClothes => ("cyan_clothes", "Cyan Clothes"),
    RegularClothes => ("reg_clothes", "Reg Clothes"),
    LeatherArmor => ("leather_armor", "Leather Armor"),
    SnakeArmor => ("snake_armor", "Snake Armor"),
    IronArmor => ("iron_armor", "Iron Armor"),
    GoldArmor => ("gold_armor", "Gold Armor"),
    GemArmor => ("gem_armor", "Gem Armor"),
    EmptyBucket => ("bucket", "Empty Bucket"),
    WaterBucket => ("water_bucket", "Water Bucket"),
    LavaBucket => ("lava_bucket", "Lava Bucket"),
    AwkwardPotion => ("potion", "Awkward Potion"),
    SpeedPotion => ("potion", "Speed Potion"),
    LightPotion => ("potion", "Light Potion"),
    SwimPotion => ("potion", "Swim Potion"),
    HastePotion => ("potion", "Haste Potion"),
    LavaPotion => ("potion", "Lava Potion"),
    EnergyPotion => ("potion", "Energy Potion"),
    RegenPotion => ("potion", "Regen Potion"),
    HealthPotion => ("potion", "Health Potion"),
    EscapePotion => ("potion", "Escape Potion"),
    ShieldPotion => ("potion", "Shield Potion"),
    TimePotion => ("potion", "Time Potion"),
    AirTotem => ("air_totem", "Totem of Air"),
    ObsidianPoppet => ("knight_statue", "Obsidian Poppet"),
    ObsidianHeart => ("obsidian_heart", "Obsidian Heart"),
    WoodFishingRod => ("wooden_fishing_rod", "Wood Fishing Rod"),
    IronFishingRod => ("iron_fishing_rod", "Iron Fishing Rod"),
    GoldFishingRod => ("gold_fishing_rod", "Gold Fishing Rod"),
    GemFishingRod => ("gem_fishing_rod", "Gem Fishing Rod"),
    WateringCan => ("watering_can", "Watering Can"),
    WheatSeeds => ("seed", "Wheat Seeds"),
    Carrot => ("carrot", "Carrot"),
    TomatoSeeds => ("seed", "Tomato Seeds"),
    HeavenlyBerries => ("heavenly_berries", "Heavenly Berries"),
    HellishBerries => ("hellish_berries", "Hellish Berries"),
    GrassSeeds => ("seed", "Grass Seeds"),
    Sign => ("sign", "Sign"),
    FarmlandItem => ("missing_item", "Farmland"),
    HoleItem => ("missing_item", "Hole"),
    LavaItem => ("missing_item", "Lava"),
    PathItem => ("missing_item", "Path"),
    WaterItem => ("missing_item", "Water"),
    Chest => ("chest", "Chest"),
    DungeonChest => ("dungeon_chest", "Dungeon Chest"),
    Tnt => ("tnt", "TNT"),
    Bed => ("bed", "Bed"),
    Composter => ("composter", "Composter"),
    Lantern => ("lantern", "Lantern"),
    IronLantern => ("iron_lantern", "Iron Lantern"),
    GoldLantern => ("gold_lantern", "Gold Lantern"),
    CowSpawner => ("cow_spawner", "Cow Spawner"),
    PigSpawner => ("pig_spawner", "Pig Spawner"),
    SheepSpawner => ("sheep_spawner", "Sheep Spawner"),
    SlimeSpawner => ("slime_spawner", "Slime Spawner"),
    ZombieSpawner => ("zombie_spawner", "Zombie Spawner"),
    CreeperSpawner => ("creeper_spawner", "Creeper Spawner"),
    SkeletonSpawner => ("skeleton_spawner", "Skeleton Spawner"),
    SnakeSpawner => ("snake_spawner", "Snake Spawner"),
    KnightSpawner => ("knight_spawner", "Knight Spawner"),
    Book => ("book", "Book"),
    AntidiousBook => ("antidious_book", "Antidious"),
}

impl ItemId {
    pub const fn food_value(self) -> Option<u8> {
        match self {
            Self::BakedPotato | Self::Apple | Self::RawPork | Self::RawFish | Self::RawBeef => {
                Some(1)
            }
            Self::Bread => Some(2),
            Self::CookedFish | Self::CookedPork | Self::Steak => Some(3),
            Self::GoldenApple => Some(10),
            _ => None,
        }
    }

    pub const fn armor_kind(self) -> Option<ArmorKind> {
        match self {
            Self::LeatherArmor => Some(ArmorKind::Leather),
            Self::SnakeArmor => Some(ArmorKind::Snake),
            Self::IronArmor => Some(ArmorKind::Iron),
            Self::GoldArmor => Some(ArmorKind::Gold),
            Self::GemArmor => Some(ArmorKind::Gem),
            _ => None,
        }
    }

    pub const fn potion_kind(self) -> Option<PotionKind> {
        match self {
            Self::AwkwardPotion => Some(PotionKind::Awkward),
            Self::SpeedPotion => Some(PotionKind::Speed),
            Self::LightPotion => Some(PotionKind::Light),
            Self::SwimPotion => Some(PotionKind::Swim),
            Self::HastePotion => Some(PotionKind::Haste),
            Self::LavaPotion => Some(PotionKind::Lava),
            Self::EnergyPotion => Some(PotionKind::Energy),
            Self::RegenPotion => Some(PotionKind::Regen),
            Self::HealthPotion => Some(PotionKind::Health),
            Self::EscapePotion => Some(PotionKind::Escape),
            Self::ShieldPotion => Some(PotionKind::Shield),
            Self::TimePotion => Some(PotionKind::Time),
            _ => None,
        }
    }

    pub const fn fishing_level(self) -> Option<u8> {
        match self {
            Self::WoodFishingRod => Some(0),
            Self::IronFishingRod => Some(1),
            Self::GoldFishingRod => Some(2),
            Self::GemFishingRod => Some(3),
            _ => None,
        }
    }
}
impl fmt::Display for ItemId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_stacks_removes_and_refuses_partial_removal() {
        let mut inventory = Inventory::new(2);
        assert_eq!(inventory.add(ItemId::Wood, 1_100), 0);
        assert_eq!(inventory.count(ItemId::Wood), 1_100);
        assert!(inventory.remove(ItemId::Wood, 101));
        assert_eq!(inventory.count(ItemId::Wood), 999);
        assert!(!inventory.remove(ItemId::Wood, 1_000));
        assert_eq!(inventory.count(ItemId::Wood), 999);
    }

    #[test]
    fn crafting_is_transactional() {
        let mut inventory = Inventory::new(2);
        inventory.add(ItemId::Wood, 10);
        assert!(HAND_RECIPES[0].craft(&mut inventory));
        assert_eq!(inventory.count(ItemId::Wood), 0);
        assert_eq!(inventory.count(ItemId::Workbench), 1);
        assert!(!HAND_RECIPES[0].craft(&mut inventory));
    }

    #[test]
    fn food_values_match_the_java_registry() {
        assert_eq!(ItemId::RawPork.food_value(), Some(1));
        assert_eq!(ItemId::Bread.food_value(), Some(2));
        assert_eq!(ItemId::CookedPork.food_value(), Some(3));
        assert_eq!(ItemId::GoldenApple.food_value(), Some(10));
        assert_eq!(ItemId::Coal.food_value(), None);
    }

    #[test]
    fn tool_durability_and_melee_match_java() {
        let mut wood_axe = ToolItem::new(ToolKind::Axe, ToolTier::Wood);
        assert_eq!(wood_axe.max_durability, 34);
        assert_eq!(wood_axe.terrain_damage(4), 14);
        assert_eq!(wood_axe.melee_bonus(3), 5);
        assert!(wood_axe.pay_durability());
        assert_eq!(wood_axe.durability, 33);
    }

    #[test]
    fn workbench_list_interleaves_items_tools_and_tail() {
        let views = recipe_views(Some(crate::entity::furniture::FurnitureKind::Workbench));
        assert_eq!(views.len(), 43);
        assert!(matches!(views[0].output, RecipeOutput::Stack(_)));
        assert!(matches!(views[28].output, RecipeOutput::Tool(_)));
        assert!(matches!(views[40].output, RecipeOutput::Stack(_)));
    }

    #[test]
    fn sorted_recipes_put_craftable_first() {
        let mut inventory = Inventory::new(27);
        inventory.add(ItemId::Wood, 10);
        let sorted = crafting::sorted_recipe_views(None, &inventory, false);
        assert!(recipe_can_craft(&sorted[0], &inventory, false));
    }

    #[test]
    fn item_registry_matches_java_2_2_4_scale_and_has_no_duplicate_displays() {
        // Java non-tool identities plus the recipe-only Arcane Fertilizer and the
        // Power Glove placeholder retained by the port inventory tables.
        assert_eq!(ItemId::ALL.len(), 142);
        let mut displays = std::collections::HashSet::new();
        for item in ItemId::ALL {
            assert!(!item.display_name().is_empty());
            assert!(
                displays.insert(item.display_name()),
                "duplicate display {}",
                item.display_name()
            );
        }
        // Placeholder asset keys are allowed only for items that are not drawn
        // as inventory textures yet; every real texture key must be non-empty.
        for item in ItemId::ALL {
            assert!(
                !item.asset_name().is_empty(),
                "empty asset for {}",
                item.display_name()
            );
        }
    }

    #[test]
    fn recipe_tables_only_reference_registered_items_and_positive_costs() {
        let stacks: Vec<ItemStack> = HAND_RECIPES
            .iter()
            .flat_map(|recipe| {
                std::iter::once(recipe.output).chain(recipe.costs.iter().copied())
            })
            .chain(WORKBENCH_STATION_RECIPES.iter().flat_map(|recipe| {
                std::iter::once(recipe.output).chain(recipe.costs.iter().copied())
            }))
            .chain(WORKBENCH_TOOL_RECIPES.iter().flat_map(|recipe| {
                recipe.costs.iter().copied().chain(std::iter::once(ItemStack {
                    item: ItemId::Wood, // tool outputs are not stacks; costs only
                    count: 0,
                }))
            }))
            .collect();
        for stack in stacks {
            if stack.count > 0 {
                assert!(!stack.item.display_name().is_empty());
            }
        }
        for recipe in HAND_RECIPES {
            assert!(recipe.output.count > 0);
            assert!(recipe.costs.iter().all(|cost| cost.count > 0));
        }
        for recipe in FURNACE_RECIPES {
            assert!(recipe.output.count > 0);
            assert!(recipe.costs.iter().all(|cost| cost.count > 0));
        }
        for recipe in OVEN_RECIPES {
            assert!(recipe.output.count > 0);
            assert!(recipe.costs.iter().all(|cost| cost.count > 0));
        }
        for recipe in LOOM_RECIPES {
            assert!(recipe.output.count > 0);
        }
        for recipe in ANVIL_RECIPES {
            assert!(recipe.output.count > 0);
        }
        for recipe in ENCHANTER_RECIPES {
            assert!(recipe.output.count > 0);
        }
    }

    #[test]
    fn potion_and_tool_kind_tables_match_java_counts() {
        assert_eq!(PotionKind::ALL.len(), 12);
        assert_eq!(ToolKind::ALL.len(), 8);
        assert_eq!(ToolKind::ALL.len(), 8);
        let mut tiers = std::collections::HashSet::new();
        for kind in ToolKind::ALL {
            for tier in [ToolTier::Wood, ToolTier::Rock, ToolTier::Iron, ToolTier::Gold, ToolTier::Gem] {
                assert!(!kind.asset_name(tier).is_empty());
                assert!(tiers.insert((kind, tier)));
            }
        }
        assert_eq!(tiers.len(), 40);
    }
}
