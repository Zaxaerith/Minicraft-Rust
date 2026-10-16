//! Java `Recipe` + `Recipes` station tables (hand/workbench/furnace/oven/loom/anvil/enchanter).
//! Instance order matches Java `Recipes` static blocks so craft-menu indices stay 1:1.
use super::{Inventory, ItemId, ItemStack, ToolItem, ToolKind, ToolTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recipe {
    pub output: ItemStack,
    pub costs: &'static [ItemStack],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolRecipe {
    pub output: ToolItem,
    pub costs: &'static [ItemStack],
    pub tool_cost: Option<(ToolKind, ToolTier)>,
}

impl ToolRecipe {
    pub fn can_craft(self, inventory: &Inventory) -> bool {
        self.costs
            .iter()
            .all(|cost| inventory.count(cost.item) >= cost.count)
            && self.tool_cost.is_none_or(|(kind, tier)| {
                inventory
                    .tools
                    .iter()
                    .any(|tool| tool.kind == kind && tool.tier == tier)
            })
    }

    pub fn craft(self, inventory: &mut Inventory) -> Option<usize> {
        if !self.can_craft(inventory) {
            return None;
        }
        let snapshot = inventory.clone();
        for cost in self.costs {
            if !inventory.remove(cost.item, cost.count) {
                *inventory = snapshot;
                return None;
            }
        }
        if let Some((kind, tier)) = self.tool_cost
            && !inventory.remove_tool(kind, tier)
        {
            *inventory = snapshot;
            return None;
        }
        let Some(index) = inventory.add_tool(self.output) else {
            *inventory = snapshot;
            return None;
        };
        Some(index)
    }
}

impl Recipe {
    pub fn can_craft(self, inventory: &Inventory) -> bool {
        self.costs
            .iter()
            .all(|cost| inventory.count(cost.item) >= cost.count)
    }

    pub fn craft(self, inventory: &mut Inventory) -> bool {
        if !self.can_craft(inventory) {
            return false;
        }
        let snapshot = inventory.clone();
        for cost in self.costs {
            if !inventory.remove(cost.item, cost.count) {
                *inventory = snapshot;
                return false;
            }
        }
        if inventory.add(self.output.item, self.output.count) != 0 {
            *inventory = snapshot;
            return false;
        }
        true
    }
}

macro_rules! recipe {
    ($output:ident, $count:expr; $( $cost:ident, $amount:expr ),+ $(,)?) => {
        Recipe {
            output: ItemStack::new(ItemId::$output, $count),
            costs: &[$(ItemStack::new(ItemId::$cost, $amount)),+],
        }
    };
}

macro_rules! tool_recipe {
    ($kind:ident, $tier:ident; $( $cost:ident, $amount:expr ),+ $(,)?) => {
        ToolRecipe {
            output: ToolItem::new(ToolKind::$kind, ToolTier::$tier),
            costs: &[$(ItemStack::new(ItemId::$cost, $amount)),+],
            tool_cost: None,
        }
    };
    ($kind:ident, $tier:ident, consumes $input_kind:ident, $input_tier:ident; $( $cost:ident, $amount:expr ),+ $(,)?) => {
        ToolRecipe {
            output: ToolItem::new(ToolKind::$kind, ToolTier::$tier),
            costs: &[$(ItemStack::new(ItemId::$cost, $amount)),+],
            tool_cost: Some((ToolKind::$input_kind, ToolTier::$input_tier)),
        }
    };
}

pub const HAND_RECIPES: [Recipe; 6] = [
    recipe!(Workbench, 1; Wood, 10),
    recipe!(Torch, 2; Wood, 1, Coal, 1),
    recipe!(Plank, 2; Wood, 1),
    recipe!(PlankWall, 1; Plank, 3),
    recipe!(WoodDoor, 1; Plank, 5),
    recipe!(WoodFence, 1; Plank, 3),
];

pub const WORKBENCH_STATION_RECIPES: [Recipe; 31] = [
    recipe!(Workbench, 1; Wood, 10),
    recipe!(Torch, 2; Wood, 1, Coal, 1),
    recipe!(Plank, 2; Wood, 1),
    recipe!(PlankWall, 1; Plank, 3),
    recipe!(WoodDoor, 1; Plank, 5),
    recipe!(WoodFence, 1; Plank, 3),
    recipe!(Lantern, 1; Wood, 8, Slime, 4, Glass, 3),
    recipe!(StoneBrick, 1; Stone, 2),
    recipe!(OrnateStone, 1; Stone, 2),
    recipe!(StoneWall, 1; StoneBrick, 3),
    recipe!(StoneDoor, 1; StoneBrick, 5),
    recipe!(StoneFence, 1; StoneBrick, 3),
    recipe!(ObsidianBrick, 1; RawObsidian, 2),
    recipe!(OrnateObsidian, 1; RawObsidian, 2),
    recipe!(ObsidianWall, 1; ObsidianBrick, 3),
    recipe!(ObsidianDoor, 1; ObsidianBrick, 5),
    recipe!(ObsidianFence, 1; ObsidianBrick, 3),
    recipe!(Oven, 1; Stone, 15),
    recipe!(Furnace, 1; Stone, 20),
    recipe!(Enchanter, 1; Wood, 5, String, 2, Lapis, 10),
    recipe!(Chest, 1; Wood, 20),
    recipe!(Anvil, 1; IronIngot, 5),
    recipe!(Tnt, 1; Gunpowder, 10, Sand, 8),
    recipe!(Loom, 1; Wood, 10, Wool, 5),
    recipe!(WoodFishingRod, 1; Wood, 10, String, 3),
    recipe!(IronFishingRod, 1; IronIngot, 10, String, 3),
    recipe!(GoldFishingRod, 1; GoldIngot, 10, String, 3),
    recipe!(GemFishingRod, 1; Gem, 10, String, 3),
    recipe!(Arrow, 3; Wood, 2, Stone, 2),
    recipe!(LeatherArmor, 1; Leather, 10),
    recipe!(SnakeArmor, 1; Scale, 15),
];

pub const WORKBENCH_TOOL_RECIPES: [ToolRecipe; 12] = [
    tool_recipe!(Sword, Wood; Wood, 5),
    tool_recipe!(Axe, Wood; Wood, 5),
    tool_recipe!(Hoe, Wood; Wood, 5),
    tool_recipe!(Pickaxe, Wood; Wood, 5),
    tool_recipe!(Shovel, Wood; Wood, 5),
    tool_recipe!(Bow, Wood; Wood, 5, String, 2),
    tool_recipe!(Sword, Rock; Wood, 5, Stone, 5),
    tool_recipe!(Axe, Rock; Wood, 5, Stone, 5),
    tool_recipe!(Hoe, Rock; Wood, 5, Stone, 5),
    tool_recipe!(Pickaxe, Rock; Wood, 5, Stone, 5),
    tool_recipe!(Shovel, Rock; Wood, 5, Stone, 5),
    tool_recipe!(Bow, Rock; Wood, 5, Stone, 5, String, 2),
];

pub const FURNACE_RECIPES: [Recipe; 4] = [
    recipe!(IronIngot, 1; IronOre, 3, Coal, 1),
    recipe!(GoldIngot, 1; GoldOre, 3, Coal, 1),
    recipe!(Glass, 1; Sand, 4, Coal, 1),
    recipe!(GlassBottle, 1; Glass, 3),
];

pub const OVEN_RECIPES: [Recipe; 5] = [
    recipe!(CookedPork, 1; RawPork, 1, Coal, 1),
    recipe!(Steak, 1; RawBeef, 1, Coal, 1),
    recipe!(CookedFish, 1; RawFish, 1, Coal, 1),
    recipe!(Bread, 1; Wheat, 4),
    recipe!(BakedPotato, 1; Potato, 1),
];

// Java 2.3.0-dev 4dc00332 Recipes. Keep old ItemId::Wool as white wool.
pub const LOOM_RECIPES: [Recipe; 57] = [
    recipe!(String, 2; Wool, 1),
    recipe!(Wool, 1; String, 3),
    recipe!(BlackWool, 1; Wool, 1, BlackDye, 1),
    recipe!(RedWool, 1; Wool, 1, RedDye, 1),
    recipe!(GreenWool, 1; Wool, 1, GreenDye, 1),
    recipe!(BrownWool, 1; Wool, 1, BrownDye, 1),
    recipe!(BlueWool, 1; Wool, 1, BlueDye, 1),
    recipe!(PurpleWool, 1; Wool, 1, PurpleDye, 1),
    recipe!(CyanWool, 1; Wool, 1, CyanDye, 1),
    recipe!(LightGrayWool, 1; Wool, 1, LightGrayDye, 1),
    recipe!(GrayWool, 1; Wool, 1, GrayDye, 1),
    recipe!(PinkWool, 1; Wool, 1, PinkDye, 1),
    recipe!(LimeWool, 1; Wool, 1, LimeDye, 1),
    recipe!(YellowWool, 1; Wool, 1, YellowDye, 1),
    recipe!(LightBlueWool, 1; Wool, 1, LightBlueDye, 1),
    recipe!(MagentaWool, 1; Wool, 1, MagentaDye, 1),
    recipe!(OrangeWool, 1; Wool, 1, OrangeDye, 1),
    recipe!(WhiteBed, 1; Wood, 5, Wool, 3),
    recipe!(BlackBed, 1; Wood, 5, BlackWool, 3),
    recipe!(RedBed, 1; Wood, 5, RedWool, 3),
    recipe!(GreenBed, 1; Wood, 5, GreenWool, 3),
    recipe!(BrownBed, 1; Wood, 5, BrownWool, 3),
    recipe!(BlueBed, 1; Wood, 5, BlueWool, 3),
    recipe!(PurpleBed, 1; Wood, 5, PurpleWool, 3),
    recipe!(CyanBed, 1; Wood, 5, CyanWool, 3),
    recipe!(LightGrayBed, 1; Wood, 5, LightGrayWool, 3),
    recipe!(GrayBed, 1; Wood, 5, GrayWool, 3),
    recipe!(PinkBed, 1; Wood, 5, PinkWool, 3),
    recipe!(LimeBed, 1; Wood, 5, LimeWool, 3),
    recipe!(YellowBed, 1; Wood, 5, YellowWool, 3),
    recipe!(LightBlueBed, 1; Wood, 5, LightBlueWool, 3),
    recipe!(MagentaBed, 1; Wood, 5, MagentaWool, 3),
    recipe!(OrangeBed, 1; Wood, 5, OrangeWool, 3),
    recipe!(BlackBed, 1; WhiteBed, 1, BlackDye, 1),
    recipe!(RedBed, 1; WhiteBed, 1, RedDye, 1),
    recipe!(GreenBed, 1; WhiteBed, 1, GreenDye, 1),
    recipe!(BrownBed, 1; WhiteBed, 1, BrownDye, 1),
    recipe!(BlueBed, 1; WhiteBed, 1, BlueDye, 1),
    recipe!(PurpleBed, 1; WhiteBed, 1, PurpleDye, 1),
    recipe!(CyanBed, 1; WhiteBed, 1, CyanDye, 1),
    recipe!(LightGrayBed, 1; WhiteBed, 1, LightGrayDye, 1),
    recipe!(GrayBed, 1; WhiteBed, 1, GrayDye, 1),
    recipe!(PinkBed, 1; WhiteBed, 1, PinkDye, 1),
    recipe!(LimeBed, 1; WhiteBed, 1, LimeDye, 1),
    recipe!(YellowBed, 1; WhiteBed, 1, YellowDye, 1),
    recipe!(LightBlueBed, 1; WhiteBed, 1, LightBlueDye, 1),
    recipe!(MagentaBed, 1; WhiteBed, 1, MagentaDye, 1),
    recipe!(OrangeBed, 1; WhiteBed, 1, OrangeDye, 1),
    recipe!(BlueClothes, 1; Cloth, 5, BlueDye, 1),
    recipe!(GreenClothes, 1; Cloth, 5, GreenDye, 1),
    recipe!(YellowClothes, 1; Cloth, 5, YellowDye, 1),
    recipe!(BlackClothes, 1; Cloth, 5, BlackDye, 1),
    recipe!(OrangeClothes, 1; Cloth, 5, OrangeDye, 1),
    recipe!(PurpleClothes, 1; Cloth, 5, PurpleDye, 1),
    recipe!(CyanClothes, 1; Cloth, 5, CyanDye, 1),
    recipe!(RegularClothes, 1; Cloth, 5),
    recipe!(LeatherArmor, 1; Leather, 10),
];

pub const ANVIL_RECIPES: [Recipe; 7] = [
    recipe!(IronArmor, 1; IronIngot, 10),
    recipe!(GoldArmor, 1; GoldIngot, 10),
    recipe!(GemArmor, 1; Gem, 65),
    recipe!(EmptyBucket, 1; IronIngot, 5),
    recipe!(IronLantern, 1; IronIngot, 8, Slime, 5, Glass, 4),
    recipe!(GoldLantern, 1; GoldIngot, 10, Slime, 5, Glass, 4),
    recipe!(WateringCan, 1; IronIngot, 3),
];

pub const ANVIL_TOOL_RECIPES: [ToolRecipe; 22] = [
    tool_recipe!(Sword, Iron; Wood, 5, IronIngot, 5),
    tool_recipe!(Claymore, Iron, consumes Sword, Iron; Shard, 15),
    tool_recipe!(Axe, Iron; Wood, 5, IronIngot, 5),
    tool_recipe!(Hoe, Iron; Wood, 5, IronIngot, 5),
    tool_recipe!(Pickaxe, Iron; Wood, 5, IronIngot, 5),
    tool_recipe!(Shovel, Iron; Wood, 5, IronIngot, 5),
    tool_recipe!(Bow, Iron; Wood, 5, IronIngot, 5, String, 2),
    tool_recipe!(Sword, Gold; Wood, 5, GoldIngot, 5),
    tool_recipe!(Claymore, Gold, consumes Sword, Gold; Shard, 15),
    tool_recipe!(Axe, Gold; Wood, 5, GoldIngot, 5),
    tool_recipe!(Hoe, Gold; Wood, 5, GoldIngot, 5),
    tool_recipe!(Pickaxe, Gold; Wood, 5, GoldIngot, 5),
    tool_recipe!(Shovel, Gold; Wood, 5, GoldIngot, 5),
    tool_recipe!(Bow, Gold; Wood, 5, GoldIngot, 5, String, 2),
    tool_recipe!(Sword, Gem; Wood, 5, Gem, 50),
    tool_recipe!(Claymore, Gem, consumes Sword, Gem; Shard, 15),
    tool_recipe!(Axe, Gem; Wood, 5, Gem, 50),
    tool_recipe!(Hoe, Gem; Wood, 5, Gem, 50),
    tool_recipe!(Pickaxe, Gem; Wood, 5, Gem, 50),
    tool_recipe!(Shovel, Gem; Wood, 5, Gem, 50),
    tool_recipe!(Bow, Gem; Wood, 5, Gem, 50, String, 2),
    tool_recipe!(Shears, Iron; IronIngot, 4),
];

pub const ENCHANTER_RECIPES: [Recipe; 14] = [
    recipe!(GoldenApple, 1; Apple, 1, GoldIngot, 8),
    recipe!(AwkwardPotion, 1; GlassBottle, 1, Lapis, 3),
    recipe!(SpeedPotion, 1; AwkwardPotion, 1, Cactus, 5),
    recipe!(LightPotion, 1; AwkwardPotion, 1, Slime, 5),
    recipe!(SwimPotion, 1; AwkwardPotion, 1, RawFish, 5),
    recipe!(HastePotion, 1; AwkwardPotion, 1, Wood, 5, Stone, 5),
    recipe!(LavaPotion, 1; AwkwardPotion, 1, LavaBucket, 1),
    recipe!(EnergyPotion, 1; AwkwardPotion, 1, Gem, 25),
    recipe!(RegenPotion, 1; AwkwardPotion, 1, GoldenApple, 1),
    recipe!(HealthPotion, 1; AwkwardPotion, 1, Gunpowder, 2, LeatherArmor, 1),
    recipe!(EscapePotion, 1; AwkwardPotion, 1, Gunpowder, 3, Lapis, 7),
    recipe!(AirTotem, 1; GoldIngot, 10, Gem, 10, Lapis, 5, CloudOre, 5),
    recipe!(ObsidianPoppet, 1; GoldIngot, 10, Gem, 10, Lapis, 5, Shard, 15),
    recipe!(ArcaneFertilizer, 3; Lapis, 6, Bone, 2),
];

/// Java 2.3.0-dev Dye Vat recipes, preserving registration order and costs.
pub const DYE_VAT_RECIPES: [Recipe; 35] = [
    recipe!(WhiteDye, 1; WhiteLily, 1),
    recipe!(LightGrayDye, 1; Flower, 1),
    recipe!(LightGrayDye, 1; Hydrangea, 1),
    recipe!(LightGrayDye, 1; WhiteTulip, 1),
    recipe!(BlueDye, 1; Lapis, 1),
    recipe!(BlueDye, 1; Cornflower, 1),
    recipe!(BlueDye, 1; Iris, 1),
    recipe!(GreenDye, 1; Cactus, 1),
    recipe!(YellowDye, 1; Sunflower, 1),
    recipe!(YellowDye, 1; Dandelion, 1),
    recipe!(LightBlueDye, 1; BlueOrchid, 1),
    recipe!(LightBlueDye, 1; Periwinkle, 1),
    recipe!(BlackDye, 1; Coal, 1),
    recipe!(RedDye, 1; Rose, 1),
    recipe!(RedDye, 1; RedTulip, 1),
    recipe!(RedDye, 1; Poppy, 1),
    recipe!(MagentaDye, 1; Allium, 1),
    recipe!(OrangeDye, 1; OrangeTulip, 1),
    recipe!(PinkDye, 1; PinkTulip, 1),
    recipe!(PinkDye, 1; Peony, 1),
    recipe!(PinkDye, 1; PinkLily, 1),
    recipe!(PurpleDye, 1; Violet, 1),
    recipe!(OrangeDye, 2; RedDye, 1, YellowDye, 1),
    recipe!(PurpleDye, 2; BlueDye, 1, RedDye, 1),
    recipe!(CyanDye, 2; BlueDye, 1, GreenDye, 1),
    recipe!(BrownDye, 2; GreenDye, 1, RedDye, 1),
    recipe!(PinkDye, 2; WhiteDye, 1, RedDye, 1),
    recipe!(LightBlueDye, 2; WhiteDye, 1, BlueDye, 1),
    recipe!(LimeDye, 2; WhiteDye, 1, GreenDye, 1),
    recipe!(GrayDye, 2; WhiteDye, 1, BlackDye, 1),
    recipe!(LightGrayDye, 2; WhiteDye, 1, GrayDye, 1),
    recipe!(LightGrayDye, 3; WhiteDye, 2, BlackDye, 1),
    recipe!(MagentaDye, 2; PurpleDye, 1, PinkDye, 1),
    recipe!(MagentaDye, 4; RedDye, 2, WhiteDye, 1, BlueDye, 1),
    recipe!(MagentaDye, 4; PinkDye, 1, RedDye, 1, BlueDye, 1),
];

pub const DYE_VAT_WORKBENCH_RECIPE: Recipe = recipe!(DyeVat, 1; Stone, 10, IronIngot, 5);
