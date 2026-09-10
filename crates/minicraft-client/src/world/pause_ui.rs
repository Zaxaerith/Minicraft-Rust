//! Pause menu / exit-confirm rendering (Java PauseDisplay host).
use super::*;
use crate::screen::pause::{PauseAction, PausePage, pause_actions};

pub(crate) fn render_pause(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world: &World,
) {
    if matches!(
        world.pause.render_view().page,
        PausePage::Quests
            | PausePage::QuestSeriesInfo
            | PausePage::QuestSeriesViewer
            | PausePage::QuestDetail
    ) {
        quest_ui::render_display(screen, assets, localization, world);
        return;
    }
    // Java Menu.Builder derives the frame from nine 8px entries with 4px
    // spacing. The non-selectable help line is the widest entry, but the
    // selectable cursor inset removes 32px from its width contribution.
    let entry_count = if world.quests_enabled { 10 } else { 9 };
    let width = 184;
    let height = entry_count * 12 + 12;
    let x = (WIDTH as i32 - width) / 2;
    let y = (HEIGHT as i32 - height) / 2;
    render_menu_frame(screen, assets.hud(), x, y, width, height);
    let (title, labels): (&str, Vec<String>) = match world.pause.render_view().page {
        PausePage::Main => (
            "Paused",
            pause_actions(world.quests_enabled)
                .into_iter()
                .map(|action| match action {
                    PauseAction::Return => "Return to Game",
                    PauseAction::Options => "Options",
                    PauseAction::Achievements => "Achievements",
                    PauseAction::Quests => "Quests",
                    PauseAction::Save => "Save Game",
                    PauseAction::MainMenu => "Main Menu",
                })
                .map(str::to_owned)
                .collect(),
        ),
        PausePage::Options => (
            "Options",
            vec![
                format!(
                    "Difficulty: {}",
                    ["Easy", "Normal", "Hard"][world.difficulty.min(2)]
                ),
                format!(
                    "Show Quest Panel: {}",
                    if world.show_quests { "On" } else { "Off" }
                ),
                format!(
                    "Tutorials: {}",
                    if world.tutorials_enabled { "On" } else { "Off" }
                ),
                format!(
                    "Quests: {}",
                    if world.quests_enabled { "On" } else { "Off" }
                ),
                "Back".to_owned(),
            ],
        ),
        PausePage::Achievements => (
            "Achievements",
            vec![
                format!("COMPLETED: {}", world.progress.achievement_count()),
                "ENTER OR ESC TO RETURN".to_owned(),
            ],
        ),
        PausePage::Quests
        | PausePage::QuestSeriesInfo
        | PausePage::QuestSeriesViewer
        | PausePage::QuestDetail => unreachable!(),
    };
    render_menu_title(screen, assets, title, x, y, width, 0xFFFF00);
    for (index, label) in labels.iter().enumerate() {
        // The first Java entry is BlankEntry; selectable entries begin on row 1.
        let draw_y = y + 20 + index as i32 * 12;
        let draw_x = x + (width - crate::gfx::font::text_width(&label)) / 2;
        let selected = if world.pause.render_view().confirm
            && world.pause.render_view().page == PausePage::Main
        {
            labels.len().saturating_sub(1)
        } else {
            world.pause.render_view().selection
        };
        if index == selected
            && matches!(
                world.pause.render_view().page,
                PausePage::Main | PausePage::Options
            )
        {
            screen.text(assets.font(), "> ", draw_x - 16, draw_y);
            screen.text(
                assets.font(),
                " <",
                draw_x + crate::gfx::font::text_width(&label),
                draw_y,
            );
        }
        let selectable = matches!(
            world.pause.render_view().page,
            PausePage::Main | PausePage::Options
        );
        screen.text_colored(
            assets.font(),
            label,
            draw_x,
            draw_y,
            if selectable && index != selected {
                0x999999
            } else {
                0xFFFFFF
            },
        );
    }
    if world.pause.render_view().page == PausePage::Main {
        let help_y = y + 20 + labels.len() as i32 * 12 + 12;
        screen.text_colored(
            assets.font(),
            "UP and DOWN to Scroll",
            (WIDTH as i32 - 21 * 8) / 2,
            help_y,
            0x999999,
        );
        screen.text_colored(
            assets.font(),
            "ENTER: Choose",
            (WIDTH as i32 - 13 * 8) / 2,
            help_y + 12,
            0x999999,
        );
    }
    if world.pause.render_view().confirm {
        // PopupDisplay ignores PopupConfig.entrySpacing and uses Menu's default
        // zero spacing. Six entries therefore produce this centered 280x64 box.
        let (popup_x, popup_y, popup_width, popup_height) = (4, 64, 280, 64);
        render_menu_frame(
            screen,
            assets.hud(),
            popup_x,
            popup_y,
            popup_width,
            popup_height,
        );
        screen.centered_text(
            assets.font(),
            "Are you sure you want to exit the",
            popup_y + 8,
        );
        screen.centered_text(assets.font(), "game?", popup_y + 16);
        let warning = "All unsaved progress will be lost";
        screen.text_colored(
            assets.font(),
            warning,
            (WIDTH as i32 - crate::gfx::font::text_width(&warning)) / 2,
            popup_y + 24,
            0xC62C2C,
        );
        for (index, label) in ["Cancel", "Quit without saving"].iter().enumerate() {
            let draw_y = popup_y + 40 + index as i32 * 8;
            let draw_x = (WIDTH as i32 - label.len() as i32 * 8) / 2;
            if index == world.pause.render_view().selection {
                screen.text(assets.font(), "> ", draw_x - 16, draw_y);
                screen.text(assets.font(), " <", draw_x + label.len() as i32 * 8, draw_y);
            }
            screen.text_colored(
                assets.font(),
                label,
                draw_x,
                draw_y,
                if index == world.pause.render_view().selection {
                    0xFFFFFF
                } else {
                    0x999999
                },
            );
        }
    }
}


