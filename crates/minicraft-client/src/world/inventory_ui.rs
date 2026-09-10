//! Java PlayerInvDisplay / CraftingDisplay / ContainerDisplay compatibility rendering.
use super::{Assets, CreativeEntry, FurnitureKind, GameMode, Player, Screen, WIDTH, World};
use crate::item::{ItemId, ItemStack, ToolItem, ToolKind, ToolTier, ordered_recipe_views};

/// Java ContainerDisplay: dual player/chest inventory panels with independent selection.
pub(crate) fn render_container(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    chest_slots: &[ItemStack],
    player_selection: usize,
    chest_selection: usize,
    focus: usize,
) {
    let player_entries = player
        .inventory
        .slots()
        .iter()
        .map(|stack| {
            (
                assets.item(stack.item),
                format!(" {} {}", stack.count.min(999), stack.item.display_name()),
            )
        })
        .collect::<Vec<_>>();
    let chest_entries = chest_slots
        .iter()
        .map(|stack| {
            (
                assets.item(stack.item),
                format!(" {} {}", stack.count.min(999), stack.item.display_name()),
            )
        })
        .collect::<Vec<_>>();
    let player_width = inventory_panel_width(&player_entries, "Inventory");
    let chest_width = inventory_panel_width(&chest_entries, "Chest");
    let player_x = 9;
    let chest_x = WIDTH as i32 - 9 - chest_width;
    render_inventory_panel(
        screen,
        assets,
        &player_entries,
        "Inventory",
        player_x,
        Some(player_selection),
    );
    render_inventory_panel(
        screen,
        assets,
        &chest_entries,
        "Chest",
        chest_x,
        Some(chest_selection),
    );
    crate::gfx::gui::render_inventory_counter(
        screen,
        assets.inventory_counter(),
        player_x + player_width,
        9,
        player_entries.len(),
        player.inventory.capacity(),
    );
    crate::gfx::gui::render_inventory_counter_minimized(
        screen,
        assets.inventory_counter(),
        chest_x + chest_width,
        9,
        chest_entries.len(),
    );
    let _ = focus;
    screen.text_colored(
        assets.font(),
        "LEFT bag RIGHT chest  C move  ESC close",
        8,
        120,
        0x999999,
    );
}

pub(crate) fn render_inventory(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    item_selection: usize,
    craft_selection: usize,
    pane: usize,
    station: Option<FurnitureKind>,
    crafting_order: &[usize],
    personal_crafting: bool,
    mode: GameMode,
) {
    if personal_crafting || station.is_some() {
        render_crafting_menu(
            screen,
            assets,
            player,
            station,
            craft_selection,
            mode == GameMode::Creative,
            crafting_order,
        );
        return;
    }

    let mut player_entries = player
        .inventory
        .slots()
        .iter()
        .map(|stack| {
            (
                assets.item(stack.item),
                format!(" {} {}", stack.count.min(999), stack.item.display_name()),
            )
        })
        .collect::<Vec<_>>();
    player_entries.extend(
        player
            .inventory
            .tools()
            .iter()
            .map(|tool| (assets.tool(*tool), format!(" {}", tool.display_name()))),
    );

    let player_width = inventory_panel_width(&player_entries, "Inventory");
    if mode != GameMode::Creative || pane == 0 {
        render_inventory_panel(
            screen,
            assets,
            &player_entries,
            "Inventory",
            9,
            Some(item_selection),
        );
        crate::gfx::gui::render_inventory_counter(
            screen,
            assets.inventory_counter(),
            9 + player_width,
            9,
            player_entries.len(),
            player.inventory.capacity(),
        );
        if mode != GameMode::Creative
            && let Some(description) = inventory_description(player, item_selection)
        {
            render_inventory_description(screen, assets, &description, 10, 105);
            screen.text(assets.font(), "(SHIFT-F) to search.", 12, 137);
        }
        return;
    }

    let creative = creative_entries();
    let creative_entries = creative
        .iter()
        .map(|entry| match entry {
            CreativeEntry::Stack(item) => (assets.item(*item), format!(" {}", item.display_name())),
            CreativeEntry::Tool(tool) => (assets.tool(*tool), format!(" {}", tool.display_name())),
        })
        .collect::<Vec<_>>();
    let creative_width = inventory_panel_width(&creative_entries, "Items");
    let creative_x = WIDTH as i32 - 10 - creative_width;
    let player_x = creative_x - 10 - player_width;
    render_inventory_panel(screen, assets, &player_entries, "Inventory", player_x, None);
    render_inventory_panel(
        screen,
        assets,
        &creative_entries,
        "Items",
        creative_x,
        Some(craft_selection),
    );
    crate::gfx::gui::render_inventory_counter_minimized(
        screen,
        assets.inventory_counter(),
        player_x + player_width,
        9,
        player_entries.len(),
    );
}

pub(crate) fn creative_entries() -> Vec<CreativeEntry> {
    let mut entries = ItemId::ALL
        .iter()
        .copied()
        .filter(|item| *item != ItemId::PowerGlove)
        .map(CreativeEntry::Stack)
        .collect::<Vec<_>>();
    for kind in ToolKind::ALL {
        for tier in ToolTier::ALL {
            if kind != ToolKind::Shears || tier == ToolTier::Wood {
                entries.push(CreativeEntry::Tool(ToolItem::new(kind, tier)));
            }
        }
    }
    entries
}

pub(crate) fn inventory_panel_width(entries: &[(&crate::gfx::Image, String)], title: &str) -> i32 {
    let content_width = entries
        .iter()
        .map(|(_, label)| label.chars().count())
        .chain([title.len()])
        .max()
        .unwrap_or(title.len()) as i32
        * 8;
    ((content_width + 48).clamp(80, WIDTH as i32 - 18) / 8) * 8
}

pub(crate) fn inventory_description(player: &Player, selection: usize) -> Option<String> {
    if let Some(stack) = player.inventory.slots().get(selection) {
        return Some(stack.item.display_name().to_owned());
    }
    player
        .inventory
        .tools()
        .get(selection.saturating_sub(player.inventory.slots().len()))
        .map(|tool| tool.display_name())
}

pub(crate) fn render_inventory_description(
    screen: &mut Screen,
    assets: &Assets,
    description: &str,
    x: i32,
    y: i32,
) {
    let width = ((crate::gfx::font::text_width(&description) + 16).max(32) + 7) / 8 * 8;
    render_menu_frame(screen, assets.hud(), x, y, width, 24);
    screen.text(assets.font(), description, x + 8, y + 8);
}

pub(crate) fn render_inventory_panel(
    screen: &mut Screen,
    assets: &Assets,
    entries: &[(&crate::gfx::Image, String)],
    title: &str,
    x: i32,
    selection: Option<usize>,
) {
    let width = inventory_panel_width(entries, title);
    let (y, height) = (9, 88);
    render_menu_frame(screen, assets.hud(), x, y, width, height);
    render_menu_title(screen, assets, title, x, y, width, 0xFFFF00);

    let selected = selection.unwrap_or(0);
    let first = selected
        .saturating_sub(4)
        .min(entries.len().saturating_sub(9));
    for (row, (image, label)) in entries.iter().skip(first).take(9).enumerate() {
        let index = first + row;
        let entry_x = x + 24;
        let entry_y = y + 8 + row as i32 * 8;
        if selection == Some(index) {
            screen.text(assets.font(), "> ", entry_x - 16, entry_y);
            screen.text(
                assets.font(),
                " <",
                entry_x + crate::gfx::font::text_width(&label),
                entry_y,
            );
        }
        screen.text(assets.font(), label, entry_x, entry_y);
        screen.blit(image, entry_x, entry_y);
    }
}

/// Renders the live inventory overlay without a world background for pixel parity.
pub fn render_inventory_parity(screen: &mut Screen, assets: &Assets, world: &World) {
    render_inventory(
        screen,
        assets,
        &world.player,
        world.inventory_item_selection,
        world.inventory_selection,
        world.inventory_pane,
        world.crafting_station,
        &world.crafting_order,
        world.personal_crafting,
        world.mode,
    );
}

pub(crate) fn render_crafting_menu(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    station: Option<FurnitureKind>,
    selection: usize,
    creative: bool,
    crafting_order: &[usize],
) {
    let recipes = ordered_recipe_views(station, &player.inventory, creative, crafting_order);
    let title = station.map_or("Crafting", FurnitureKind::display_name);
    let content_width = recipes
        .iter()
        .map(|recipe| recipe.label().chars().count())
        .chain([title.len()])
        .max()
        .unwrap_or(title.len()) as i32
        * 8;
    let width = (content_width + 48).clamp(96, 176);
    let (x, y, height) = (9, 9, 88);
    render_menu_frame(screen, assets.hud(), x, y, width, height);
    render_menu_title(screen, assets, title, x, y, width, 0xFFFF00);
    let first = selection
        .saturating_sub(4)
        .min(recipes.len().saturating_sub(9));
    for (row, recipe) in recipes.iter().skip(first).take(9).enumerate() {
        let index = first + row;
        let entry_x = x + 24;
        let entry_y = y + 8 + row as i32 * 8;
        let output = recipe.label();
        if index == selection {
            screen.text(assets.font(), "> ", entry_x - 16, entry_y);
            screen.text(
                assets.font(),
                " <",
                entry_x + crate::gfx::font::text_width(&output),
                entry_y,
            );
        }
        let craftable = creative
            || (recipe
                .costs
                .iter()
                .all(|cost| player.inventory.count(cost.item) >= cost.count)
                && recipe.tool_cost.is_none_or(|(kind, tier)| {
                    player
                        .inventory
                        .tools()
                        .iter()
                        .any(|tool| tool.kind == kind && tool.tier == tier)
                }));
        screen.text_colored(
            assets.font(),
            &output,
            entry_x,
            entry_y,
            if craftable { 0xFFFFFF } else { 0x999999 },
        );
        screen.blit(super::recipe_image(recipe, assets), entry_x, entry_y);
    }

    if let Some(recipe) = recipes.get(selection) {
        let costs = recipe.costs;
        let panel_x = x + width + 8;
        let available_width = ((WIDTH as i32 - panel_x - 9) / 8) * 8;
        let have_label = format!(" {}", super::recipe_output_count(recipe, player));
        let have_width =
            (("Have:".len().max(have_label.len()) as i32 * 8) + 16).min(available_width);
        if have_width >= 56 {
            render_menu_frame(screen, assets.hud(), panel_x, y, have_width, 24);
            render_menu_title_left(screen, assets, "Have:", panel_x, y, 0xFFFF00);
            screen.blit(super::recipe_image(recipe, assets), panel_x + 8, y + 8);
            screen.text(assets.font(), &have_label, panel_x + 8, y + 8);
            let cost_width = costs
                .iter()
                .map(|cost| format!(" {}/{}", player.inventory.count(cost.item), cost.count).len())
                .chain(["Cost:".len()])
                .max()
                .unwrap_or("Cost:".len()) as i32
                * 8
                + 16;
            let cost_width = cost_width.min(available_width);
            let cost_height = costs.len().max(1) as i32 * 8 + 16;
            let cost_y = y + height - cost_height;
            render_menu_frame(
                screen,
                assets.hud(),
                panel_x,
                cost_y,
                cost_width,
                cost_height,
            );
            render_menu_title_left(screen, assets, "Cost:", panel_x, cost_y, 0xFFFF00);
            for (row, cost) in costs.iter().enumerate() {
                let entry_y = cost_y + 8 + row as i32 * 8;
                screen.blit(assets.item(cost.item), panel_x + 8, entry_y);
                screen.text(
                    assets.font(),
                    &format!(" {}/{}", player.inventory.count(cost.item), cost.count),
                    panel_x + 8,
                    entry_y,
                );
            }
        }
    }
}

pub(crate) use crate::gfx::gui::render_menu_frame;

pub(crate) fn render_menu_title(
    screen: &mut Screen,
    assets: &Assets,
    title: &str,
    x: i32,
    y: i32,
    width: i32,
    color: u32,
) {
    let title_width = crate::gfx::font::text_width(&title);
    let title_x = x + (width - title_width) / 2;
    crate::gfx::gui::render_title(
        screen,
        assets.hud(),
        assets.font(),
        title,
        title_x,
        y,
        color,
    );
}

pub(crate) fn render_menu_title_left(
    screen: &mut Screen,
    assets: &Assets,
    title: &str,
    x: i32,
    y: i32,
    color: u32,
) {
    let title_x = x + 8;
    crate::gfx::gui::render_title(
        screen,
        assets.hud(),
        assets.font(),
        title,
        title_x,
        y,
        color,
    );
}
