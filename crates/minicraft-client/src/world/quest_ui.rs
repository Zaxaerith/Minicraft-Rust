//! Java QuestsDisplay / QuestSeries / QuestDetail rendering (Pause pages).
use crate::{
    gfx::gui::render_menu_frame,
    screen::{pause::PausePage, rel_pos::RelPos},
};

use super::{Assets, HEIGHT, Screen, WIDTH, World, quest_tree};
use crate::resource::localization::Localization;

fn quest_description_lines(text: &str) -> Vec<String> {
    // Java StringEntry.useLines uses pixel width and keeps the final remainder.
    crate::gfx::font::get_lines(text, WIDTH as i32 - 20, HEIGHT as i32 * 2, 0, false)
}

fn render_root(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
    selection: usize,
) {
    // QuestsDisplay(clearScreen=true), including when drawn as a parent.
    screen.clear(0);
    screen.centered_text(
        assets.font(),
        &format!(
            "{}: {}",
            localization.text("minicraft.settings.quests"),
            if world.quests_enabled { "On" } else { "Off" }
        ),
        6,
    );
    let unlocked = localization.text("minicraft.displays.quests.display.header.unlocked");
    let completed = localization.text("minicraft.displays.quests.display.header.completed");
    screen.text_colored(
        assets.font(),
        unlocked,
        WIDTH as i32 / 2 - crate::gfx::font::text_width(unlocked) - 16,
        26,
        if world.pause.render_view().quest_tab == 0 {
            0xFFFFFF
        } else {
            0x999999
        },
    );
    screen.text_colored(
        assets.font(),
        completed,
        WIDTH as i32 / 2 + 16,
        26,
        if world.pause.render_view().quest_tab == 1 {
            0xFFFFFF
        } else {
            0x999999
        },
    );
    let groups = world.progress.quest_group_views(
        &world.inventory_progress(),
        world.pause.render_view().quest_tab == 1,
    );
    let first = selection
        .saturating_sub(2)
        .min(groups.len().saturating_sub(5));
    // The menu is built empty with displayLength=5. setEntries does not
    // recalculate its fixed 44px-high entry bounds, centered at (144,76).
    for (row, group) in groups.iter().skip(first).take(5).enumerate() {
        let label = localization.text(&group.id);
        let width = crate::gfx::font::text_width(label);
        let x = (WIDTH as i32 - width) / 2;
        let y = 54 + row as i32 * 9;
        if first + row == selection {
            screen.text(assets.font(), "> ", x - 16, y);
            screen.text(assets.font(), " <", x + width, y);
        }
        screen.text_colored(
            assets.font(),
            label,
            x,
            y,
            if group.completed {
                quest_tree::GREEN
            } else {
                0xFFFFFF
            },
        );
    }
    let description = groups
        .get(selection)
        .map(|group| localization.text(&group.description))
        .unwrap_or_else(|| localization.text("minicraft.displays.quests.display.no_quest_desc"));
    let lines = quest_description_lines(description);
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    crate::gfx::font_style::FontStyle::new(0xFFFFFF)
        .set_shadow_mask(0, 0)
        .draw_paragraph_layout(
            screen,
            assets.font(),
            &lines,
            &crate::gfx::font_style::ParagraphLayout {
                anchor: (WIDTH as i32 / 2, 127),
                block_position: RelPos::BOTTOM,
                line_position: RelPos::TOP,
                spacing: 0,
            },
        );
}

/// Menu.Builder(true, spacing, CENTER) anchored at (Screen.w/2, top), BOTTOM.
/// Titles here are StringEntry rows inside separate panels, not yellow frame titles.
fn render_info_panel(
    screen: &mut Screen,
    assets: &Assets,
    top: i32,
    spacing: i32,
    entries: &[(String, u32, bool)],
) {
    let selectable = entries.iter().any(|entry| entry.2);
    let cursor_inset = if selectable { 32 } else { 0 };
    let content_width = entries
        .iter()
        .map(|(text, _, select)| {
            (crate::gfx::font::text_width(text)
                - if selectable && !select {
                    cursor_inset
                } else {
                    0
                })
            .max(0)
        })
        .max()
        .unwrap_or(0);
    let width = content_width + cursor_inset + 16;
    let step = 8 + spacing;
    let visible = ((HEIGHT as i32 - top - 16 + spacing) / step).max(0) as usize;
    let count = entries.len().min(visible);
    let height = count as i32 * step - spacing + 16;
    render_menu_frame(
        screen,
        assets.hud(),
        (WIDTH as i32 - width) / 2,
        top,
        width,
        height,
    );
    for (row, (text, color, select)) in entries.iter().take(count).enumerate() {
        let x = (WIDTH as i32 - crate::gfx::font::text_width(text)) / 2;
        let y = top + 8 + row as i32 * step;
        screen.text_colored(assets.font(), text, x, y, *color);
        if *select {
            screen.text(assets.font(), "> ", x - 16, y);
            screen.text(
                assets.font(),
                " <",
                x + crate::gfx::font::text_width(text),
                y,
            );
        }
    }
}

pub(super) fn render_display(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
) {
    render_page(
        screen,
        assets,
        localization,
        world,
        world.pause.render_view().page,
    );
}

fn render_page(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
    page: PausePage,
) {
    match page {
        PausePage::Quests => {
            render_root(
                screen,
                assets,
                localization,
                world,
                world.pause.render_view().selection,
            );
        }
        PausePage::QuestSeriesInfo => {
            render_root(
                screen,
                assets,
                localization,
                world,
                world.pause.render_view().quest_series_selection,
            );
            let Some(group) = world
                .progress
                .quest_group_views(&world.inventory_progress(), false)
                .into_iter()
                .find(|group| group.index == world.pause.render_view().quest_group)
            else {
                return;
            };
            render_info_panel(
                screen,
                assets,
                5,
                0,
                &[(localization.text(&group.id).to_owned(), 0xFFFFFF, false)],
            );
            let status_key = if group.completed {
                "minicraft.displays.quests.quest_info.display.status.completed"
            } else {
                "minicraft.displays.quests.quest_info.display.status.unlocked"
            };
            let mut entries = vec![
                (
                    localization.format(
                        "minicraft.displays.quests.quest_info.display.status",
                        &[localization.text(status_key)],
                    ),
                    if group.completed {
                        quest_tree::GREEN
                    } else {
                        0xFFFFFF
                    },
                    false,
                ),
                (
                    localization.format(
                        "minicraft.displays.quests.quest_info.display.quests_completed_count",
                        &[&group.completed_quests.to_string()],
                    ),
                    0xFFFFFF,
                    false,
                ),
            ];
            let description = localization.format(
                "minicraft.displays.quests.quest_info.display.description",
                &[localization.text(&group.description)],
            );
            entries.extend(
                quest_description_lines(&description)
                    .into_iter()
                    .map(|line| (line, 0xFFFFFF, false)),
            );
            entries.push((
                localization.format(
                    "minicraft.displays.quests.quest_info.display.ongoing_quests",
                    &[&group.ongoing_quests.to_string()],
                ),
                0xFFFFFF,
                false,
            ));
            entries.push((String::new(), 0xFFFFFF, false));
            entries.push((
                localization
                    .text("minicraft.displays.quests.quest_info.view_quests")
                    .to_owned(),
                0xFFFFFF,
                true,
            ));
            render_info_panel(screen, assets, 30, 2, &entries);
        }
        PausePage::QuestSeriesViewer | PausePage::QuestDetail => {
            let quests = world.progress.quest_views(
                world.pause.render_view().quest_group,
                &world.inventory_progress(),
            );
            let Some(group) = world
                .progress
                .quest_group_views(&world.inventory_progress(), false)
                .into_iter()
                .find(|group| group.index == world.pause.render_view().quest_group)
            else {
                return;
            };
            if page == PausePage::QuestSeriesViewer {
                render_page(
                    screen,
                    assets,
                    localization,
                    world,
                    PausePage::QuestSeriesInfo,
                );
                render_info_panel(
                    screen,
                    assets,
                    6,
                    0,
                    &[
                        (
                            localization.text("minicraft.displays.quests").to_owned(),
                            0xFFFFFF,
                            false,
                        ),
                        (localization.text(&group.id).to_owned(), 0xFFFFFF, false),
                    ],
                );
                render_menu_frame(
                    screen,
                    assets.hud(),
                    8,
                    40,
                    WIDTH as i32 - 16,
                    HEIGHT as i32 - 60,
                );
                let tree = quest_tree::Tree::new(&quests, |id| localization.text(id).to_owned());
                tree.render(
                    screen,
                    assets.font(),
                    &quests,
                    world.pause.render_view().selection,
                    &world.quest_tree_state,
                    localization.text("minicraft.displays.quests.display.no_quest"),
                );
            } else {
                render_page(
                    screen,
                    assets,
                    localization,
                    world,
                    PausePage::QuestSeriesViewer,
                );
                if let Some(quest) = quests.get(world.pause.render_view().quest_item) {
                    let status_key = if quest.completed {
                        "minicraft.displays.quests.quest_info.display.status.completed"
                    } else {
                        "minicraft.displays.quests.quest_info.display.status.unlocked"
                    };
                    let progress = if quest.has_requirements {
                        localization
                            .text(
                                "minicraft.displays.quests.quest_info.display.progress_uncompleted",
                            )
                            .to_owned()
                    } else {
                        localization.format(
                            "minicraft.displays.quests.quest_info.display.progress",
                            &[
                                &quest.criteria_completed.to_string(),
                                &quest.criteria_total.to_string(),
                            ],
                        )
                    };
                    render_info_panel(
                        screen,
                        assets,
                        5,
                        1,
                        &[
                            (localization.text(&group.id).to_owned(), 0xFFFFFF, false),
                            (
                                format!(
                                    "{}: {}",
                                    localization.text(&quest.id),
                                    localization.text(status_key)
                                ),
                                if quest.completed {
                                    quest_tree::GREEN
                                } else {
                                    0xFFFFFF
                                },
                                false,
                            ),
                            (progress, 0xFFFFFF, false),
                        ],
                    );
                    let entries: Vec<_> =
                        quest_description_lines(localization.text(&quest.description))
                            .into_iter()
                            .map(|line| (line, 0xFFFFFF, false))
                            .collect();
                    render_info_panel(screen, assets, 52, 2, &entries);
                }
            }
        }
        PausePage::Main | PausePage::Options | PausePage::Achievements => unreachable!(),
    }
}
