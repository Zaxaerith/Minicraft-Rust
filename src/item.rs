use std::fmt;

use serde::{Deserialize, Serialize};

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

impl fmt::Display for ItemId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u16,
}

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

    pub const fn id(self) -> usize {
        self as usize
    }

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

impl ItemStack {
    pub const fn new(item: ItemId, count: u16) -> Self {
        Self { item, count }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    slots: Vec<ItemStack>,
    tools: Vec<ToolItem>,
    capacity: usize,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: Vec::new(),
            tools: Vec::new(),
            capacity,
        }
    }

    pub fn slots(&self) -> &[ItemStack] {
        &self.slots
    }

    pub fn tools(&self) -> &[ToolItem] {
        &self.tools
    }

    pub fn tools_mut(&mut self) -> &mut [ToolItem] {
        &mut self.tools
    }

    pub fn used_slots(&self) -> usize {
        self.slots.len() + self.tools.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Restores a fixed slot limit and returns stack slots that no longer fit. The
    /// world loader drops those stacks beside the player instead of deleting them.
    pub fn set_capacity(&mut self, capacity: usize) -> Vec<ItemStack> {
        self.capacity = capacity;
        let stack_capacity = capacity.saturating_sub(self.tools.len());
        if self.slots.len() > stack_capacity {
            self.slots.split_off(stack_capacity)
        } else {
            Vec::new()
        }
    }

    pub fn take_all(&mut self) -> Vec<ItemStack> {
        std::mem::take(&mut self.slots)
    }

    pub fn add_tool(&mut self, tool: ToolItem) -> Option<usize> {
        if self.used_slots() >= self.capacity {
            return None;
        }
        self.tools.push(tool);
        Some(self.tools.len() - 1)
    }

    fn remove_tool(&mut self, kind: ToolKind, tier: ToolTier) -> bool {
        let Some(index) = self
            .tools
            .iter()
            .position(|tool| tool.kind == kind && tool.tier == tier)
        else {
            return false;
        };
        self.tools.remove(index);
        true
    }

    pub fn count(&self, item: ItemId) -> u16 {
        self.slots
            .iter()
            .filter(|stack| stack.item == item)
            .map(|stack| stack.count)
            .sum()
    }

    /// Adds as much as possible and returns the uninserted remainder.
    pub fn add(&mut self, item: ItemId, mut count: u16) -> u16 {
        for stack in self.slots.iter_mut().filter(|stack| stack.item == item) {
            let room = MAX_STACK - stack.count;
            let inserted = room.min(count);
            stack.count += inserted;
            count -= inserted;
            if count == 0 {
                return 0;
            }
        }
        while count > 0 && self.used_slots() < self.capacity {
            let inserted = count.min(MAX_STACK);
            self.slots.push(ItemStack::new(item, inserted));
            count -= inserted;
        }
        count
    }

    pub fn remove(&mut self, item: ItemId, mut count: u16) -> bool {
        if self.count(item) < count {
            return false;
        }
        for stack in self.slots.iter_mut().rev() {
            if stack.item != item {
                continue;
            }
            let removed = stack.count.min(count);
            stack.count -= removed;
            count -= removed;
            if count == 0 {
                break;
            }
        }
        self.slots.retain(|stack| stack.count > 0);
        true
    }
}

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
            debug_assert!(inventory.remove(cost.item, cost.count));
        }
        if let Some((kind, tier)) = self.tool_cost {
            debug_assert!(inventory.remove_tool(kind, tier));
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
            debug_assert!(inventory.remove(cost.item, cost.count));
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

pub const LOOM_RECIPES: [Recipe; 17] = [
    recipe!(String, 2; Wool, 1),
    recipe!(RedWool, 1; Wool, 1, Rose, 1),
    recipe!(BlueWool, 1; Wool, 1, Lapis, 1),
    recipe!(GreenWool, 1; Wool, 1, Cactus, 1),
    recipe!(YellowWool, 1; Wool, 1, Flower, 1),
    recipe!(BlackWool, 1; Wool, 1, Coal, 1),
    recipe!(Bed, 1; Wood, 5, Wool, 3),
    recipe!(BlueClothes, 1; Cloth, 5, Lapis, 1),
    recipe!(GreenClothes, 1; Cloth, 5, Cactus, 1),
    recipe!(YellowClothes, 1; Cloth, 5, Flower, 1),
    recipe!(BlackClothes, 1; Cloth, 5, Coal, 1),
    recipe!(OrangeClothes, 1; Cloth, 5, Rose, 1, Flower, 1),
    recipe!(PurpleClothes, 1; Cloth, 5, Lapis, 1, Rose, 1),
    recipe!(CyanClothes, 1; Cloth, 5, Lapis, 1, Cactus, 1),
    recipe!(RegularClothes, 1; Cloth, 5),
    recipe!(RedClothes, 1; Cloth, 5, Rose, 1),
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
    fn tool_durability_and_damage_match_java_formulas() {
        let mut wood_axe = ToolItem::new(ToolKind::Axe, ToolTier::Wood);
        assert_eq!(wood_axe.max_durability, 34);
        assert_eq!(wood_axe.terrain_damage(4), 14);
        assert_eq!(wood_axe.melee_bonus(3), 5);
        assert!(wood_axe.pay_durability());
        assert_eq!(wood_axe.durability, 33);

        let gem_sword = ToolItem::new(ToolKind::Sword, ToolTier::Gem);
        assert_eq!(gem_sword.max_durability, 260);
        assert_eq!(gem_sword.melee_bonus(17), 32);
    }

    #[test]
    fn workbench_tool_recipe_is_transactional_and_uses_a_slot() {
        let mut inventory = Inventory::new(1);
        inventory.add(ItemId::Wood, 5);
        let index = WORKBENCH_TOOL_RECIPES[0].craft(&mut inventory).unwrap();
        assert_eq!(index, 0);
        assert_eq!(inventory.count(ItemId::Wood), 0);
        assert_eq!(inventory.tools()[0].kind, ToolKind::Sword);
        assert!(!WORKBENCH_TOOL_RECIPES[0].can_craft(&inventory));
    }

    #[test]
    fn furnace_workbench_and_anvil_form_an_iron_tool_chain() {
        let mut inventory = Inventory::new(12);
        inventory.add(ItemId::IronOre, 30);
        inventory.add(ItemId::Coal, 10);
        inventory.add(ItemId::Wood, 5);
        for _ in 0..10 {
            assert!(FURNACE_RECIPES[0].craft(&mut inventory));
        }
        assert_eq!(inventory.count(ItemId::IronIngot), 10);
        let anvil = WORKBENCH_STATION_RECIPES
            .iter()
            .find(|recipe| recipe.output.item == ItemId::Anvil)
            .expect("workbench must expose the anvil recipe");
        assert!(anvil.craft(&mut inventory));
        assert_eq!(inventory.count(ItemId::Anvil), 1);
        let iron_pickaxe = ANVIL_TOOL_RECIPES
            .iter()
            .find(|recipe| {
                recipe.output.kind == ToolKind::Pickaxe && recipe.output.tier == ToolTier::Iron
            })
            .expect("anvil must expose the iron pickaxe recipe");
        let tool = iron_pickaxe.craft(&mut inventory).unwrap();
        assert_eq!(inventory.tools()[tool].kind, ToolKind::Pickaxe);
        assert_eq!(inventory.tools()[tool].tier, ToolTier::Iron);
        assert_eq!(inventory.count(ItemId::IronIngot), 0);
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
    fn phase_five_catalog_and_recipe_groups_are_complete() {
        // Java registers 141 non-tool identities; its recipe-only Arcane
        // Fertilizer output is retained explicitly so that recipe is playable.
        assert_eq!(ItemId::ALL.len(), 142);
        assert_eq!(ToolKind::ALL.len(), 8);
        assert_eq!(ToolTier::ALL.len(), 5);
        assert_eq!(PotionKind::ALL.len(), 12);
        assert_eq!(HAND_RECIPES.len(), 6);
        assert_eq!(WORKBENCH_STATION_RECIPES.len(), 31);
        assert_eq!(WORKBENCH_TOOL_RECIPES.len(), 12);
        assert_eq!(FURNACE_RECIPES.len(), 4);
        assert_eq!(OVEN_RECIPES.len(), 5);
        assert_eq!(LOOM_RECIPES.len(), 17);
        assert_eq!(ANVIL_RECIPES.len(), 7);
        assert_eq!(ANVIL_TOOL_RECIPES.len(), 22);
        assert_eq!(ENCHANTER_RECIPES.len(), 14);
        assert!(ItemId::ALL.contains(&ItemId::ObsidianHeart));
        assert!(ItemId::ALL.contains(&ItemId::GemFishingRod));
        assert!(ItemId::ALL.contains(&ItemId::KnightSpawner));
    }

    #[test]
    fn claymore_upgrade_consumes_the_matching_sword() {
        let mut inventory = Inventory::new(8);
        inventory.add(ItemId::Shard, 15);
        inventory
            .add_tool(ToolItem::new(ToolKind::Sword, ToolTier::Iron))
            .unwrap();
        let recipe = ANVIL_TOOL_RECIPES
            .iter()
            .find(|recipe| {
                recipe.output.kind == ToolKind::Claymore && recipe.output.tier == ToolTier::Iron
            })
            .unwrap();
        let index = recipe.craft(&mut inventory).unwrap();
        assert_eq!(inventory.tools().len(), 1);
        assert_eq!(inventory.tools()[index].kind, ToolKind::Claymore);
        assert_eq!(inventory.count(ItemId::Shard), 0);
    }
}
