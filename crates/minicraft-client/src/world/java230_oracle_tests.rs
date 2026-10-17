//! Golden data comes from actual pinned Java methods, not Rust expectations.
use super::{
    ActiveItem, FurnitureKind, Tile, World, entity::EntityKind, registry_tests::tiny_world,
};
use crate::{
    assets::Assets,
    entity::furniture::bed,
    item::{
        ItemId, RecipeOutput, RecipeView, ToolItem, ToolKind, ToolTier, dye::DyeColor, recipe_views,
    },
    level::tile::flower::FlowerVariant,
};
use serde_json::Value;
use std::collections::BTreeMap;

const TARGET: usize = 9 + 8 * 16;

fn canonical(name: &str) -> String {
    let name: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect();
    // Aliases retain existing Rust item/save identities.
    match name.as_str() {
        "whitewool" => "wool".to_owned(),
        "iron" => "ironingot".to_owned(),
        "gold" => "goldingot".to_owned(),
        _ => name,
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RecipeIdentity {
    product: String,
    amount: u16,
    costs: BTreeMap<String, u16>,
}

fn java_recipe(value: &Value) -> RecipeIdentity {
    RecipeIdentity {
        product: canonical(value["product"].as_str().unwrap()),
        amount: u16::try_from(value["amount"].as_u64().unwrap()).unwrap(),
        costs: value["costs"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, count)| {
                (
                    canonical(key),
                    u16::try_from(count.as_u64().unwrap()).unwrap(),
                )
            })
            .collect(),
    }
}

fn rust_recipe(value: RecipeView) -> RecipeIdentity {
    let (product, amount) = match value.output {
        RecipeOutput::Stack(stack) => (stack.item.display_name().to_owned(), stack.count),
        RecipeOutput::Tool(tool) => (tool.display_name(), 1),
    };
    let mut costs = BTreeMap::new();
    for cost in value.costs {
        *costs
            .entry(canonical(cost.item.display_name()))
            .or_insert(0) += cost.count;
    }
    if let Some((kind, tier)) = value.tool_cost {
        *costs
            .entry(canonical(&ToolItem::new(kind, tier).display_name()))
            .or_insert(0) += 1;
    }
    RecipeIdentity {
        product: canonical(&product),
        amount,
        costs,
    }
}

#[test]
fn java230_actual_java_recipe_tables_match_products_amounts_and_all_costs() {
    let tables: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/recipes.json")).unwrap();
    for (key, station, count) in [
        ("dyeVat", FurnitureKind::DyeVat, 35),
        ("loom", FurnitureKind::Loom, 57),
        ("workbenchDyeVat", FurnitureKind::Workbench, 1),
    ] {
        let mut java: Vec<_> = tables[key]
            .as_array()
            .unwrap()
            .iter()
            .map(java_recipe)
            .collect();
        assert_eq!(java.len(), count);
        let mut rust: Vec<_> = recipe_views(Some(station)).into_iter()
            .filter(|recipe| key != "workbenchDyeVat" || matches!(recipe.output, RecipeOutput::Stack(stack) if stack.item == ItemId::DyeVat))
            .map(rust_recipe).collect();
        // Recipe identity includes costs and output count; duplicate alternatives
        // remain duplicated. Display sorting is covered by the menu tests.
        java.sort();
        rust.sort();
        assert_eq!(rust, java, "complete Java {key} table");
    }
}

#[test]
fn java230_actual_java_palette_and_bed_instances_survive_native_save() {
    let colors: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/colors.json")).unwrap();
    let beds: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/beds.json")).unwrap();
    let furniture: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/furniture.json")).unwrap();
    let assets = Assets::load(&[]).unwrap();
    assert_eq!(colors.len(), DyeColor::ALL.len());
    assert_eq!(beds.len(), DyeColor::ALL.len());
    assert_eq!(
        bed::color_from_state(0) as u64,
        furniture["defaultBedColor"].as_u64().unwrap()
    );
    for ((color, reference), bed_reference) in DyeColor::ALL.into_iter().zip(colors).zip(beds) {
        assert_eq!(color as u64, reference["ordinal"].as_u64().unwrap());
        assert_eq!(
            u64::from(color.packed_color()),
            reference["packedColor"].as_u64().unwrap()
        );
        assert_eq!(color.display_name(), reference["name"].as_str().unwrap());
        assert_eq!(
            color.dye_item().display_name(),
            reference["dye"].as_str().unwrap()
        );
        assert_eq!(
            color.dye_item().asset_name(),
            reference["dyeSprite"].as_str().unwrap()
        );
        assert_eq!(
            canonical(color.wool_item().display_name()),
            canonical(reference["wool"].as_str().unwrap())
        );
        assert_eq!(
            color.bed_item().display_name(),
            bed_reference["name"].as_str().unwrap()
        );
        assert_eq!(
            assets.colored_bed_linked(color).key,
            bed_reference["entitySprite"].as_str().unwrap()
        );
        assert_eq!(
            color.bed_item().asset_name(),
            bed_reference["itemSprite"].as_str().unwrap()
        );
        assert_eq!(bed_reference["placed"], true);
        assert_eq!(bed_reference["itemDepleted"], true);
        assert_eq!(bed_reference["copyColor"], bed_reference["color"]);
        assert_eq!(bed_reference["restoredColor"], bed_reference["color"]);
        let radii = FurnitureKind::Bed.collision_radius();
        assert_eq!(
            (i64::from(radii.0), i64::from(radii.1)),
            (
                bed_reference["xr"].as_i64().unwrap(),
                bed_reference["yr"].as_i64().unwrap()
            )
        );
        let mut world = tiny_world();
        let item = color.bed_item();
        world.player.inventory.add(item, 1);
        world.player.active_item = Some(ActiveItem::Stack(item));
        world.attack();
        assert_eq!(world.player.inventory.count(item), 0);
        let loaded = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        let placed = loaded.levels[0]
            .entities
            .entities()
            .iter()
            .find(|entity| matches!(entity.kind, EntityKind::Furniture(FurnitureKind::Bed)))
            .unwrap();
        assert_eq!(
            bed::color_from_state(placed.state) as u64,
            bed_reference["restoredColor"].as_u64().unwrap()
        );
        assert_eq!(
            crate::item::furniture_item::item_for_furniture_state(FurnitureKind::Bed, placed.state),
            Some(item)
        );
        assert!(assets.colored_bed(color).width > 0);
    }
    assert_eq!(
        FurnitureKind::DyeVat.display_name(),
        furniture["dyeVatName"].as_str().unwrap()
    );
    assert_eq!(
        FurnitureKind::DyeVat.asset_name(),
        furniture["dyeVatSprite"].as_str().unwrap()
    );
    let radii = FurnitureKind::DyeVat.collision_radius();
    assert_eq!(
        (i64::from(radii.0), i64::from(radii.1)),
        (
            furniture["dyeVatXr"].as_i64().unwrap(),
            furniture["dyeVatYr"].as_i64().unwrap()
        )
    );
}

fn actual_drops(world: &World) -> Vec<(String, u16)> {
    world.levels[0]
        .entities
        .entities()
        .iter()
        .filter_map(|entity| match entity.kind {
            EntityKind::Item(stack) => Some((canonical(stack.item.display_name()), stack.count)),
            _ => None,
        })
        .collect()
}

fn java_drops(value: &Value) -> Vec<(String, u16)> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|value| {
            let (name, count) = value.as_str().unwrap().rsplit_once('_').unwrap();
            (canonical(name), count.parse().unwrap())
        })
        .collect()
}

#[test]
fn java230_actual_java_wool_placement_shearing_and_drop_instances_match() {
    let values: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/wool.json")).unwrap();
    assert_eq!(values.len(), DyeColor::ALL.len());
    for (color, reference) in DyeColor::ALL.into_iter().zip(values) {
        let mut world = tiny_world();
        world.levels[0].tiles[TARGET] = Tile::Hole;
        let item = color.wool_item();
        world.player.inventory.add(item, 1);
        world.player.active_item = Some(ActiveItem::Stack(item));
        world.attack();
        assert_eq!(reference["placed"], true);
        assert_eq!(
            canonical(world.levels[0].tiles[TARGET].asset_name()),
            canonical(reference["tile"].as_str().unwrap())
        );
        assert_eq!(
            u64::from(world.levels[0].data[TARGET]),
            reference["data"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(world.player.inventory.count(item)),
            reference["remaining"].as_u64().unwrap()
        );
        let mut shears = ToolItem::new(ToolKind::Shears, ToolTier::Iron);
        shears.durability = 17;
        let index = world.player.inventory.add_tool(shears).unwrap();
        world.player.active_item = Some(ActiveItem::Tool(index));
        world.attack();
        assert_eq!(reference["harvested"], true);
        assert_eq!(
            canonical(world.levels[0].tiles[TARGET].asset_name()),
            canonical(reference["afterTile"].as_str().unwrap())
        );
        assert_eq!(
            u64::from(world.player.inventory.tools()[index].durability),
            reference["durability"].as_u64().unwrap()
        );
        assert_eq!(actual_drops(&world), java_drops(&reference["drops"]));
    }
}

#[test]
fn java230_actual_java_flower_placement_ordinals_and_punch_harvest_match() {
    let values: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/java230/flowers.json")).unwrap();
    assert_eq!(values.len(), FlowerVariant::ALL.len());
    for (variant, reference) in FlowerVariant::ALL.into_iter().zip(values) {
        assert_eq!(variant as u64, reference["ordinal"].as_u64().unwrap());
        assert_eq!(
            variant.item().display_name(),
            reference["name"].as_str().unwrap()
        );
        assert_eq!(variant.asset_name(), reference["sprite"].as_str().unwrap());
        let mut world = tiny_world();
        world.player.inventory.add(variant.item(), 1);
        world.player.active_item = Some(ActiveItem::Stack(variant.item()));
        world.attack();
        assert_eq!(reference["placed"], true);
        assert_eq!(world.levels[0].tiles[TARGET], Tile::Flower230);
        assert_eq!(
            u64::from(world.levels[0].data[TARGET]),
            reference["data"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(world.player.inventory.count(variant.item())),
            reference["remaining"].as_u64().unwrap()
        );
        world.attack();
        assert_eq!(reference["harvested"], true);
        assert_eq!(
            canonical(world.levels[0].tiles[TARGET].asset_name()),
            canonical(reference["afterTile"].as_str().unwrap())
        );
        assert_eq!(actual_drops(&world), java_drops(&reference["drops"]));
    }
}

#[test]
fn java230_actual_ornate_wood_recipe_placement_mining_and_save_match() {
    let reference: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/java230/ornate-wood.json"
    ))
    .unwrap();
    assert_eq!(reference["javaTileId"], 59);
    assert_eq!(Tile::OrnateWood.id(), 70); // Append-only native registry.
    assert_eq!(
        ItemId::OrnateWood.asset_name(),
        reference["itemSprite"].as_str().unwrap()
    );
    let recipe = recipe_views(Some(FurnitureKind::Workbench))
        .into_iter()
        .find(
            |r| matches!(r.output, RecipeOutput::Stack(stack) if stack.item == ItemId::OrnateWood),
        )
        .unwrap();
    assert_eq!(rust_recipe(recipe), java_recipe(&reference["recipe"]));
    let assets = Assets::load(&[]).unwrap();
    assert_ne!(
        assets.tile(Tile::OrnateWood, 0),
        assets.tile(Tile::WoodFloor, 0)
    );
    for case in reference["placement"].as_array().unwrap() {
        let mut world = tiny_world();
        let base = match case["base"].as_str().unwrap() {
            "Hole" => Tile::Hole,
            "Water" => Tile::Water,
            "Cloud" => Tile::Cloud,
            "Lava" => Tile::Lava,
            _ => unreachable!(),
        };
        world.levels[0].tiles[TARGET] = base;
        world.player.inventory.add(ItemId::OrnateWood, 2);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::OrnateWood));
        world.attack();
        assert_eq!(
            canonical(world.levels[0].tiles[TARGET].asset_name()),
            canonical(case["after"].as_str().unwrap())
        );
        assert_eq!(
            u64::from(world.player.inventory.count(ItemId::OrnateWood)),
            case["remaining"].as_u64().unwrap()
        );
        let restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        assert_eq!(
            restored.levels[0].tiles[TARGET],
            world.levels[0].tiles[TARGET]
        );
        assert_eq!(
            restored.player.inventory.count(ItemId::OrnateWood),
            world.player.inventory.count(ItemId::OrnateWood)
        );
    }
    for case in reference["mining"].as_array().unwrap() {
        let mut world = tiny_world();
        world.levels[0].depth = case["depth"].as_i64().unwrap() as i8;
        world.levels[0].tiles[TARGET] = Tile::OrnateWood;
        let kind = if case["tool"] == "Iron Axe" {
            ToolKind::Axe
        } else {
            ToolKind::Pickaxe
        };
        let mut tool = ToolItem::new(kind, ToolTier::Iron);
        tool.durability = 1;
        world.player.inventory.add_tool(tool).unwrap();
        world.player.active_item = Some(ActiveItem::Tool(0));
        world.attack();
        assert_eq!(
            canonical(world.levels[0].tiles[TARGET].asset_name()),
            canonical(case["after"].as_str().unwrap())
        );
        assert_eq!(
            u64::from(world.player.stamina),
            case["stamina"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(
                world
                    .player
                    .inventory
                    .tools()
                    .first()
                    .map_or(0, |t| t.durability)
            ),
            case["durability"].as_u64().unwrap()
        );
        assert_eq!(actual_drops(&world), java_drops(&case["drops"]));
        assert_eq!(
            world.player.inventory.tools().is_empty(),
            kind == ToolKind::Axe
        );
        let restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        assert_eq!(actual_drops(&restored), actual_drops(&world));
        assert_eq!(
            restored.levels[0].tiles[TARGET],
            world.levels[0].tiles[TARGET]
        );
    }
}
