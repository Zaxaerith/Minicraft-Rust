//! End-to-end crafting/progression compatibility using real World actions.
use super::registry_tests::tiny_world;
use super::{FurnitureKind, World};
use crate::item::{Inventory, ItemId, RecipeOutput, ordered_recipe_views};

fn select(world: &mut World, output: ItemId, first_cost: ItemId) {
    world.refresh_crafting_order();
    let recipes = ordered_recipe_views(
        world.crafting_station,
        &world.player.inventory,
        false,
        &world.crafting_order,
        &world.progress,
    );
    world.inventory_selection = recipes
        .iter()
        .position(|recipe| {
            matches!(recipe.output, RecipeOutput::Stack(stack) if stack.item == output)
                && recipe
                    .costs
                    .first()
                    .is_some_and(|cost| cost.item == first_cost)
        })
        .expect("expected recipe is unlocked in the actual menu");
}

#[test]
fn java230_dye_vat_crafting_and_full_bag_are_transactional() {
    let mut world = tiny_world();
    world.progress.begin_new_world();
    world.crafting_station = Some(FurnitureKind::DyeVat);
    world.player.inventory = Inventory::new(1);
    world.player.inventory.add(ItemId::Coal, 2);
    select(&mut world, ItemId::BlackDye, ItemId::Coal);
    world.craft_selected_recipe();
    assert_eq!(world.player.inventory.count(ItemId::Coal), 2);
    assert_eq!(world.player.inventory.count(ItemId::BlackDye), 0);
    world.player.inventory.set_capacity(27);
    world.craft_selected_recipe();
    assert_eq!(world.player.inventory.count(ItemId::Coal), 1);
    assert_eq!(world.player.inventory.count(ItemId::BlackDye), 1);
    world.player.inventory.remove(ItemId::Coal, 1);
    world.craft_selected_recipe();
    assert_eq!(world.player.inventory.count(ItemId::BlackDye), 1);
    let world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert_eq!(world.player.inventory.count(ItemId::BlackDye), 1);
}

#[test]
fn java230_alternative_dye_unlocks_survive_save_and_old_catalog_migration() {
    let mut world = tiny_world();
    world.progress.begin_new_world();
    world.crafting_station = Some(FurnitureKind::DyeVat);
    world.player.inventory.add(ItemId::Lapis, 2);
    world.player.inventory.add(ItemId::Cornflower, 2);
    world.refresh_crafting_order();
    let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    for input in [ItemId::Lapis, ItemId::Cornflower] {
        select(&mut world, ItemId::BlueDye, input);
        world.craft_selected_recipe();
    }
    assert_eq!(world.player.inventory.count(ItemId::BlueDye), 2);
    assert_eq!(world.player.inventory.count(ItemId::Lapis), 1);
    assert_eq!(world.player.inventory.count(ItemId::Cornflower), 1);
    let mut old: serde_json::Value =
        serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    old["world"]["progress"]
        .as_object_mut()
        .unwrap()
        .remove("recipe_catalog_version");
    old["world"]["progress"]
        .as_object_mut()
        .unwrap()
        .remove("recipe_variants");
    old["world"]["progress"]["recipe_elements"]
        .as_array_mut()
        .unwrap()
        .retain(|element| !element["key"].as_str().unwrap().starts_with("java230:"));
    let mut restored = World::from_save_string(&old.to_string()).unwrap();
    restored.player.inventory.add(ItemId::Coal, 2);
    restored.refresh_crafting_order();
    select(&mut restored, ItemId::BlackDye, ItemId::Coal);
    restored.craft_selected_recipe();
    assert_eq!(restored.player.inventory.count(ItemId::BlueDye), 2);
    assert_eq!(restored.player.inventory.count(ItemId::BlackDye), 1);
}

#[test]
fn java230_all_loom_and_dye_recipes_execute_real_menu_actions() {
    for station in [FurnitureKind::DyeVat, FurnitureKind::Loom] {
        for recipe in crate::item::recipe_views(Some(station)) {
            let RecipeOutput::Stack(output) = recipe.output else {
                panic!("stack recipes only");
            };
            let mut world = tiny_world();
            world.progress.begin_new_world();
            world.crafting_station = Some(station);
            for cost in recipe.costs {
                world.player.inventory.add(cost.item, cost.count);
            }
            select(&mut world, output.item, recipe.costs[0].item);
            // Several bed recipes have the same first cost Wood; select by full
            // definition as well, independently of sort order.
            let recipes = ordered_recipe_views(
                world.crafting_station,
                &world.player.inventory,
                false,
                &world.crafting_order,
                &world.progress,
            );
            world.inventory_selection = recipes
                .iter()
                .position(|candidate| {
                    matches!(candidate.output, RecipeOutput::Stack(stack) if stack == output)
                        && candidate.costs == recipe.costs
                })
                .unwrap();
            world.craft_selected_recipe();
            assert_eq!(
                world.player.inventory.count(output.item),
                output.count,
                "{}",
                output.item.display_name()
            );
            for cost in recipe.costs {
                assert_eq!(
                    world.player.inventory.count(cost.item),
                    0,
                    "{}",
                    cost.item.display_name()
                );
            }
            let restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
            assert_eq!(restored.player.inventory.count(output.item), output.count);
        }
    }
}

#[test]
fn java230_native_disk_save_quit_reload_and_unknown_items_fail_explicitly() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::OrangeDye, 17);
    world.player.inventory.add(ItemId::CyanWool, 4);
    world.player.inventory.add(ItemId::MagentaBed, 1);
    world.player.inventory.add(ItemId::DyeVat, 1);
    world.player.inventory.add(ItemId::Violet, 2);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parity/java230/native-save-test");
    std::fs::create_dir_all(&root).unwrap();
    let record = crate::worlds::WorldRecord {
        generation_version: crate::level::GenerationVersion::Legacy224,
        name: "java230-local-test".to_owned(),
        seed: world.seed,
        spec: super::WorldSpec::new(128, 0, 0),
        mode: world.mode,
        score_minutes: 20,
        directory: root.clone(),
    };
    // This is the same disk writer/loader called for Save & Quit and re-entry.
    crate::worlds::save_state(&record, &world).unwrap();
    drop(world);
    let world = crate::worlds::load_state(&record).unwrap().unwrap();
    for (item, amount) in [
        (ItemId::OrangeDye, 17),
        (ItemId::CyanWool, 4),
        (ItemId::MagentaBed, 1),
        (ItemId::DyeVat, 1),
        (ItemId::Violet, 2),
    ] {
        assert_eq!(world.player.inventory.count(item), amount);
    }
    let invalid = world
        .to_save_string()
        .unwrap()
        .replace("OrangeDye", "UnknownFutureDye");
    assert!(
        World::from_save_string(&invalid).is_err(),
        "unknown items must not be silently removed"
    );
    for name in ["state.json", "state.json.bak"] {
        let file = root.join(name);
        if file.exists() {
            std::fs::remove_file(file).unwrap();
        }
    }
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn java230_old_earned_costs_are_preserved_and_reimport_replaces_unlocks() {
    let mut world = tiny_world();
    world
        .progress
        .import_unlocked_recipes(serde_json::json!({"Blue Wool_1": ["Wool_1", "Lapis_1"]}))
        .unwrap();
    world.player.inventory.add(ItemId::Wool, 1);
    world.player.inventory.add(ItemId::BlueDye, 1);
    world.crafting_station = Some(FurnitureKind::Loom);
    select(&mut world, ItemId::BlueWool, ItemId::Wool);
    world.craft_selected_recipe();
    assert_eq!(world.player.inventory.count(ItemId::BlueWool), 1);
    let world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    let saved = serde_json::to_value(&world.progress).unwrap();
    let costs = saved["recipe_variants"]["BLUE WOOL_1"].as_array().unwrap();
    assert!(costs.iter().any(|costs| {
        costs
            .as_array()
            .unwrap()
            .iter()
            .any(|cost| cost.as_str().unwrap().eq_ignore_ascii_case("Lapis_1"))
    }));
    assert!(costs.iter().any(|costs| {
        costs
            .as_array()
            .unwrap()
            .iter()
            .any(|cost| cost.as_str().unwrap().eq_ignore_ascii_case("Blue Dye_1"))
    }));
    let mut progress = world.progress;
    progress
        .import_unlocked_recipes(serde_json::json!({}))
        .unwrap();
    assert!(
        crate::item::recipe_views(Some(FurnitureKind::Loom))
            .iter()
            .all(|recipe| !progress.recipe_is_unlocked(recipe))
    );
}
