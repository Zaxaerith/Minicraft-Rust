//! Java `Recipes` craft-list views (CraftingDisplay) — station tables,
//! Have/Cost sort, and `can_craft` (Java `Recipe.checkCanCraft`).
use crate::entity::furniture::FurnitureKind;

use super::recipe::{
    ANVIL_RECIPES, ANVIL_TOOL_RECIPES, ENCHANTER_RECIPES, FURNACE_RECIPES, HAND_RECIPES,
    LOOM_RECIPES, OVEN_RECIPES,
};
use super::{
    Inventory, ItemStack, Recipe, ToolItem, ToolKind, ToolRecipe, ToolTier,
    WORKBENCH_STATION_RECIPES, WORKBENCH_TOOL_RECIPES,
};

#[derive(Clone, Copy)]
pub enum RecipeOutput {
    Stack(ItemStack),
    Tool(ToolItem),
}

#[derive(Clone, Copy)]
pub struct RecipeView {
    pub output: RecipeOutput,
    pub costs: &'static [ItemStack],
    pub tool_cost: Option<(ToolKind, ToolTier)>,
}

impl RecipeView {
    pub fn product_name(&self) -> String {
        match self.output {
            RecipeOutput::Stack(stack) => stack.item.display_name().to_owned(),
            RecipeOutput::Tool(tool) => tool.display_name(),
        }
    }

    pub fn label(&self) -> String {
        match self.output {
            RecipeOutput::Stack(stack) => {
                let suffix = if stack.count > 1 {
                    format!(" x{}", stack.count)
                } else {
                    String::new()
                };
                format!(" {}{}", stack.item.display_name(), suffix)
            }
            RecipeOutput::Tool(tool) => format!(" {}", tool.display_name()),
        }
    }
}

/// Java station recipe lists in registration order (workbench splits items/tools).
///
/// Java `Recipes.workbenchRecipes` interleaves the tool block after Gem Fishing Rod
/// and before arrow / leather / snake armor — the split index is 28.
pub fn recipe_views(station: Option<FurnitureKind>) -> Vec<RecipeView> {
    let stack = |recipe: &Recipe| RecipeView {
        output: RecipeOutput::Stack(recipe.output),
        costs: recipe.costs,
        tool_cost: None,
    };
    let tool = |recipe: &ToolRecipe| RecipeView {
        output: RecipeOutput::Tool(recipe.output),
        costs: recipe.costs,
        tool_cost: recipe.tool_cost,
    };
    match station {
        None => HAND_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Workbench) => WORKBENCH_STATION_RECIPES[..28]
            .iter()
            .map(stack)
            .chain(WORKBENCH_TOOL_RECIPES.iter().map(tool))
            .chain(WORKBENCH_STATION_RECIPES[28..].iter().map(stack))
            .collect(),
        Some(FurnitureKind::Oven) => OVEN_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Furnace) => FURNACE_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Anvil) => ANVIL_RECIPES
            .iter()
            .map(stack)
            .chain(ANVIL_TOOL_RECIPES.iter().map(tool))
            .collect(),
        Some(FurnitureKind::Enchanter) => ENCHANTER_RECIPES.iter().map(stack).collect(),
        Some(FurnitureKind::Loom) => LOOM_RECIPES.iter().map(stack).collect(),
        _ => Vec::new(),
    }
}

/// Java CraftingDisplay: craftable recipes sort above blocked ones.
pub fn sorted_recipe_views(
    station: Option<FurnitureKind>,
    inventory: &Inventory,
    creative: bool,
) -> Vec<RecipeView> {
    let mut recipes = recipe_views(station);
    recipes.sort_by(|left, right| {
        recipe_can_craft(right, inventory, creative)
            .cmp(&recipe_can_craft(left, inventory, creative))
    });
    recipes
}

pub fn ordered_recipe_views(
    station: Option<FurnitureKind>,
    inventory: &Inventory,
    creative: bool,
    order: &[usize],
    progress: &crate::progression::ProgressState,
) -> Vec<RecipeView> {
    let recipes = recipe_views(station);
    let ordered =
        if order.len() == recipes.len() && order.iter().all(|&index| index < recipes.len()) {
            order.iter().map(|&index| recipes[index]).collect()
        } else {
            sorted_recipe_views(station, inventory, creative)
        };
    ordered
        .into_iter()
        .filter(|recipe| progress.recipe_is_unlocked(recipe))
        .collect()
}

/// Same query used for Have/Cost display and craft gating.
pub fn recipe_can_craft(recipe: &RecipeView, inventory: &Inventory, creative: bool) -> bool {
    creative
        || (recipe
            .costs
            .iter()
            .all(|cost| inventory.count(cost.item) >= cost.count)
            && recipe.tool_cost.is_none_or(|(kind, tier)| {
                inventory
                    .tools()
                    .iter()
                    .any(|tool| tool.kind == kind && tool.tier == tier)
            }))
}
