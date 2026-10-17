//! Java Renderer HUD layer: hearts, stamina, score, quests, potions, boss bar.
use super::{ActiveItem, Assets, GameMode, HEIGHT, Player, Screen, WIDTH, World};
use crate::gfx::gui::render_menu_frame;
use crate::item::{ItemId, PotionKind, ToolKind};

pub(crate) fn render_score_overlay(
    screen: &mut Screen,
    assets: &Assets,
    ticks: i32,
    score: u32,
    multiplier: u8,
) {
    let total_seconds = (f64::from(ticks) / 60.0).ceil() as i32;
    let total_minutes = total_seconds / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let seconds = total_seconds % 60;
    let time = format!(
        "TIME LEFT {}{}m {}s",
        if hours > 0 {
            format!("{hours}h ")
        } else {
            String::new()
        },
        minutes,
        seconds
    );
    screen.text(assets.font(), &time, WIDTH as i32 / 2 - 72, 2);
    let score = format!("CURRENT SCORE: {score}");
    screen.text(
        assets.font(),
        &score,
        WIDTH as i32 - crate::gfx::font::text_width(&score) - 2,
        11,
    );
    if multiplier > 1 {
        let multiplier = format!("X{multiplier}");
        screen.text(
            assets.font(),
            &multiplier,
            WIDTH as i32 - crate::gfx::font::text_width(&multiplier) - 2,
            20,
        );
    }
}

pub(crate) fn render_quest_overlay(screen: &mut Screen, assets: &Assets, progress: &str) {
    let label = progress.chars().take(30).collect::<String>();
    let width =
        (((label.chars().count().max(6) as i32 * 8 + 32) / 8) * 8).clamp(72, WIDTH as i32 - 18);
    let x = WIDTH as i32 - 9 - width;
    render_menu_frame(screen, assets.hud(), x, 9, width, 24);
    screen.text(assets.font(), "QUESTS", x + (width - 48) / 2, 9);
    screen.text(assets.font(), &label, x + 16, 17);
}

pub(crate) fn render_potion_overlay(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    offset: i32,
) {
    for (row, kind) in PotionKind::ALL
        .into_iter()
        .filter(|kind| player.potion_effects[kind.id()] > 0)
        .enumerate()
    {
        let total_seconds = u32::from(player.potion_effects[kind.id()]) / 60;
        let label = format!(
            "{} ({}:{:02})",
            kind.display_name(),
            total_seconds / 60,
            total_seconds % 60
        );
        let x = 180;
        let y = 17 + offset + row as i32 * 8;
        screen.rect(
            x - 2,
            y - 1,
            crate::gfx::font::text_width(&label) + 4,
            10,
            0x101018,
        );
        screen.text(assets.font(), &label, x, y);
    }
}

pub(crate) fn render_boss_bar(
    screen: &mut Screen,
    assets: &Assets,
    health: u16,
    max_health: u16,
    title: &str,
) {
    let x = WIDTH as i32 / 4 - 24;
    let y = HEIGHT as i32 / 8 - 24;
    let length = if max_health == 0 {
        0
    } else {
        usize::from(health) * 100 / usize::from(max_health)
    };
    screen.blit_region_transformed(assets.hud(), 0, 32, 8, 8, x + 200, y, true, false);
    for column in 0..100 {
        screen.blit_region(assets.hud(), x + column * 2, y, 24, 32, 8, 8, false);
    }
    screen.blit_region(assets.hud(), x - 5, y, 0, 40, 8, 8, false);
    for column in 0..length {
        screen.blit_region(assets.hud(), x + column as i32 * 2, y, 24, 40, 8, 8, false);
    }
    screen.centered_text(assets.font(), title, y + 8);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_hud(
    screen: &mut Screen,
    assets: &Assets,
    player: &Player,
    mode: GameMode,
    _seed: i64,
    _depth: i8,
    _day_tick: u32,
    _days: u32,
) {
    let bottom = HEIGHT as i32;

    if mode != GameMode::Creative || player.active_item.is_some() {
        for x in 10..26 {
            screen.blit_region(assets.hud(), x * 8, bottom - 8, 40, 16, 8, 8, false);
        }
    }

    if let Some(active) = player.active_item {
        let (image, label) = match active {
            ActiveItem::Stack(item) => (
                Some(assets.item(item)),
                format!(
                    " {} {}",
                    player.inventory.count(item).clamp(1, 999),
                    item.display_name()
                ),
            ),
            ActiveItem::Tool(index) => player
                .inventory
                .tools()
                .get(index)
                .map(|tool| (assets.tool(*tool), format!(" {}", tool.display_name())))
                .map_or((None, String::new()), |(image, label)| (Some(image), label)),
            ActiveItem::Slot { .. } => player
                .stack_for_active(active)
                .map(|stack| {
                    (
                        Some(assets.item_stack(stack)),
                        format!(" {} {}", stack.count, stack.item.display_name()),
                    )
                })
                .unwrap_or((None, String::new())),
        };
        if let Some(image) = image {
            screen.blit(image, 80, bottom - 8);
            draw_hud_text_background(screen, assets, &label, 88, bottom - 8, 0xFFFFFF);
        }
    }

    if let Some(ActiveItem::Tool(index)) = player.active_item
        && let Some(tool) = player.inventory.tools().get(index)
    {
        let durability = u32::from(tool.durability) * 100 / u32::from(tool.max_durability.max(1));
        draw_hud_text_background(
            screen,
            assets,
            &format!("{durability}%"),
            164,
            bottom - 16,
            hud_percentage_color(durability),
        );
        if tool.kind == ToolKind::Bow {
            draw_hud_text_background(
                screen,
                assets,
                &format!(" x{}", player.inventory.count(ItemId::Arrow)),
                84,
                bottom - 16,
                0xFFFFFF,
            );
            screen.blit_region(assets.hud(), 84, bottom - 16, 32, 8, 8, 8, false);
        }
    }
    if let Some((_, stack)) = player.active_slot()
        && stack.item == ItemId::WateringCan
    {
        let content = stack
            .watering_content
            .map(|water| u32::from(water) * 100 / 1_800);
        let label = content.map_or_else(|| "?".to_owned(), |water| format!("{water}%"));
        draw_hud_text_background(
            screen,
            assets,
            &label,
            164,
            bottom - 16,
            content.map_or(0xFFFFFF, hud_percentage_color),
        );
    }

    let effects = PotionKind::ALL
        .into_iter()
        .filter(|effect| player.potion_effects[effect.id()] > 0)
        .map(|effect| {
            format!(
                "{} {}",
                &effect.display_name()[..effect.display_name().len().min(3)],
                player.potion_effects[effect.id()].div_ceil(60)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    if !effects.is_empty() {
        let width = crate::gfx::font::text_width(&effects) + 4;
        screen.rect(WIDTH as i32 - width - 2, 8, width, 10, 0x101018);
        screen.text(assets.font(), &effects, WIDTH as i32 - width, 9);
    }

    for index in 0..10 {
        let health_source_y = if player.health > 20 {
            24
        } else if player.health > 10 {
            16
        } else {
            0
        };
        screen.blit_region(assets.hud(), index * 8, bottom - 16, 0, 8, 8, 8, false);
        let tier_start = if player.health > 20 {
            20
        } else if player.health > 10 {
            10
        } else {
            0
        };
        if usize::from(player.health.saturating_sub(tier_start)) > index as usize {
            screen.blit_region(
                assets.hud(),
                index * 8,
                bottom - 16,
                0,
                health_source_y,
                8,
                8,
                false,
            );
        }

        if let Some(armor) = player.armor_kind {
            let armor_icons = u16::from(player.armor) * 10 / u16::from(armor.durability().max(1));
            if index <= i32::from(armor_icons) {
                screen.blit(assets.item(armor.item()), index * 8, bottom - 24);
            }
        }

        let stamina_source_y = if player.stamina_recharge_delay > 0 {
            if (player.stamina_recharge_delay / 4).is_multiple_of(2) {
                16
            } else {
                8
            }
        } else if index < i32::from(player.stamina) {
            0
        } else {
            8
        };
        screen.blit_region(
            assets.hud(),
            index * 8,
            bottom - 8,
            8,
            stamina_source_y,
            8,
            8,
            false,
        );
        screen.blit_region(
            assets.hud(),
            WIDTH as i32 - 80 + index * 8,
            bottom - 16,
            16,
            if index < i32::from(player.hunger) {
                0
            } else {
                8
            },
            8,
            8,
            false,
        );
    }
}

pub(crate) fn draw_hud_text_background(
    screen: &mut Screen,
    assets: &Assets,
    text: &str,
    x: i32,
    y: i32,
    color: u32,
) {
    for (index, _) in text.chars().enumerate() {
        screen.blit_region(assets.hud(), x + index as i32 * 8, y, 40, 16, 8, 8, false);
    }
    screen.text_colored(assets.font(), text, x, y, color);
}

pub(crate) fn hud_percentage_color(percent: u32) -> u32 {
    let green = (percent.min(100) as f32 * 2.55) as u32;
    (255 - green) << 16 | green << 8
}

/// Renders only the live gameplay HUD for deterministic pixel comparison.
pub fn render_hud_parity(screen: &mut Screen, assets: &Assets, world: &World) {
    render_hud(
        screen,
        assets,
        &world.player,
        world.mode,
        world.seed,
        world.levels[world.current_level].depth,
        world.clock.get().day_tick,
        world.days,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{Inventory, ItemStack};

    #[test]
    fn water_hud_renders_individual_contents_and_ignores_stale_slot_selection() {
        let assets = Assets::load(&[]).unwrap();
        let mut world = super::super::registry_tests::tiny_world();
        world.player.inventory = Inventory::new(4);
        world.player.inventory.add_stack(ItemStack::with_water(731));
        world.player.inventory.add_stack(ItemStack::with_water(23));
        let revision = world.player.inventory.slot_revision();
        let mut previous = Vec::new();
        for content in [None, Some(0), Some(23), Some(731)] {
            world.player.inventory.slot_mut(0).unwrap().watering_content = content;
            world.player.active_item = Some(ActiveItem::Slot { index: 0, revision });
            let before = world.player.inventory.clone();
            let mut screen = Screen::new();
            render_hud_parity(&mut screen, &assets, &world);
            assert_eq!(world.player.inventory, before);
            let rendered = screen.pixels().to_vec();
            assert!(previous.iter().all(|pixels| pixels != &rendered));
            previous.push(rendered);
        }
        world.player.active_item = Some(ActiveItem::Slot { index: 1, revision });
        world.player.inventory.remove_slot(0, 1).unwrap();
        let mut stale = Screen::new();
        render_hud_parity(&mut stale, &assets, &world);
        world.player.active_item = None;
        let mut empty = Screen::new();
        render_hud_parity(&mut empty, &assets, &world);
        assert!(stale.pixels() == empty.pixels());
        assert_eq!(world.player.inventory.slots(), &[ItemStack::with_water(23)]);
    }
}
