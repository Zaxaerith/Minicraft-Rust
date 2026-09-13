//! Top-level menu / parity preview drawing for the compatibility State host.
use super::menu_build::TITLE_KEYS;
use super::*;
use crate::io::Input;
use crate::{
    progression::{Achievement, Book},
    resource::pack::ResourcePack,
};

pub(crate) fn render_resource_pack_parity(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    scene: minicraft_common::parity::RenderScene,
) {
    use minicraft_common::parity::RenderScene;
    let mut packs = vec![
        ResourcePack::parity_fixture("alpha", "Alpha", Some(assets.default_pack_logo.clone())),
        ResourcePack::parity_fixture("beta", "Beta", None),
        ResourcePack::parity_fixture("gamma", "Gamma", None),
    ];
    let mut enabled = vec!["alpha".to_owned(), "gamma".to_owned()];
    order_packs_for_menu(&mut packs, &enabled);
    let mut selection = 2;
    if matches!(
        scene,
        RenderScene::ResourcePacksReordered | RenderScene::ResourcePacksToggled
    ) {
        tick_resource_packs(
            &mut selection,
            &Input {
                left_pressed: true,
                ..Input::default()
            },
            &mut packs,
        );
        sync_enabled_pack_order(&mut enabled, &packs);
    }
    if scene == RenderScene::ResourcePacksToggled
        && let Transition::TogglePack(index) = tick_resource_packs(
            &mut selection,
            &Input {
                select: true,
                ..Input::default()
            },
            &mut packs,
        )
    {
        enabled.push(packs[index].id.clone());
        sync_enabled_pack_order(&mut enabled, &packs);
    }
    if scene == RenderScene::ResourcePacksDefault {
        packs.clear();
        enabled.clear();
        selection = 0;
    }
    if scene == RenderScene::ResourcePacksLogo {
        selection = 1;
    }
    if scene == RenderScene::ResourcePacksScrolled {
        packs = (0..15)
            .map(|index| {
                ResourcePack::parity_fixture(
                    &format!("pack{index:02}"),
                    &format!("Pack {index:02}"),
                    None,
                )
            })
            .collect();
        enabled = vec!["pack00".into(), "pack14".into()];
        selection = 0;
        for _ in 0..14 {
            tick_resource_packs(
                &mut selection,
                &Input {
                    down_pressed: true,
                    ..Input::default()
                },
                &mut packs,
            );
        }
    }
    render_resource_packs(
        screen,
        assets,
        localization,
        &packs,
        &enabled,
        selection,
        &KeyBindings::default(),
    );
    if scene == RenderScene::ResourcePacksInfo {
        crate::resource_pack_ui::render_info(
            screen,
            assets,
            localization,
            packs.get(selection),
        );
    }
}

pub(super) fn render_title_menu(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    menu: &crate::screen::menu::Menu,
    ticks: u64,
) {
    screen.blit(
        assets.title(),
        (WIDTH as i32 - assets.title().width as i32) / 2,
        26 - assets.title().height as i32 / 2,
    );
    let phase = ticks % 50;
    let count = if phase < 25 { phase + 1 } else { 49 - phase };
    let brightness = 5_u64.saturating_sub(count / 5);
    let splash_color = ((brightness * 51) << 16) | ((brightness * 51) << 8) | (brightness * 25);
    crate::gfx::font::draw_centered(
        screen,
        assets.font(),
        "Now with skins!",
        HEIGHT as i32 / 2 - 44,
        splash_color as u32,
    );
    let version = localization.format("minicraft.displays.title.display.version", &["2.2.4"]);
    screen.text_colored(assets.font(), &version, 1, 1, 0x333333);
    let latest = localization.text("minicraft.displays.title.display.latest_already");
    crate::gfx::font::draw_centered(screen, assets.font(), latest, 76, 0x333333);
    menu.render(screen, assets.hud(), assets.font());
    for (message, y) in [
        (
            localization.format("minicraft.displays.title.display.help.0", &["UP", "DOWN"]),
            HEIGHT as i32 - 30,
        ),
        (
            localization.format("minicraft.displays.title.display.help.1", &["ENTER"]),
            HEIGHT as i32 - 20,
        ),
        (
            localization.format("minicraft.displays.title.display.help.2", &["ESCAPE"]),
            HEIGHT as i32 - 10,
        ),
    ] {
        crate::gfx::font::draw_centered(screen, assets.font(), &message, y, 0x333333);
    }
}

pub(crate) fn render_title(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    selection: usize,
    ticks: u64,
) {
    screen.blit(
        assets.title(),
        (WIDTH as i32 - assets.title().width as i32) / 2,
        26 - assets.title().height as i32 / 2,
    );
    let phase = ticks % 50;
    let count = if phase < 25 { phase + 1 } else { 49 - phase };
    let brightness = 5_u64.saturating_sub(count / 5);
    let splash_color = ((brightness * 51) << 16) | ((brightness * 51) << 8) | (brightness * 25);
    crate::gfx::font::draw_centered(
        screen,
        assets.font(),
        "Now with skins!",
        HEIGHT as i32 / 2 - 44,
        splash_color as u32,
    );
    let version = localization.format("minicraft.displays.title.display.version", &["2.2.4"]);
    screen.text_colored(assets.font(), &version, 1, 1, 0x333333);
    let latest = localization.text("minicraft.displays.title.display.latest_already");
    crate::gfx::font::draw_centered(screen, assets.font(), latest, 76, 0x333333);
    for (index, key) in TITLE_KEYS.iter().enumerate() {
        let item = localization.text(key);
        let y = 96 + index as i32 * 10;
        if index == selection {
            let width = crate::gfx::font::text_width(&item);
            let x = (WIDTH as i32 - width) / 2;
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(assets.font(), "<", x + width, y);
        }
        let x = (WIDTH as i32 - crate::gfx::font::text_width(&item)) / 2;
        screen.text_colored(
            assets.font(),
            item,
            x,
            y,
            if index == selection {
                0xFFFFFF
            } else {
                0x999999
            },
        );
    }
    for (message, y) in [
        (
            localization.format("minicraft.displays.title.display.help.0", &["UP", "DOWN"]),
            HEIGHT as i32 - 30,
        ),
        (
            localization.format("minicraft.displays.title.display.help.1", &["ENTER"]),
            HEIGHT as i32 - 20,
        ),
        (
            localization.format("minicraft.displays.title.display.help.2", &["ESCAPE"]),
            HEIGHT as i32 - 10,
        ),
    ] {
        crate::gfx::font::draw_centered(screen, assets.font(), &message, y, 0x333333);
    }
}

pub(super) fn render_controls_menu(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    menu: &crate::screen::menu::Menu,
    keyboard: bool,
    capture: Option<(usize, bool)>,
) {
    menu.render(screen, assets.hud(), assets.font());
    crate::gfx::font::draw_centered(
        screen,
        assets.font(),
        localization.text("minicraft.displays.controls"),
        0,
        0xFF_FF_FF,
    );
    let mode_key = if keyboard {
        "minicraft.displays.controls.display.keyboard"
    } else {
        "minicraft.displays.controls.display.controller"
    };
    crate::gfx::font::draw_centered(
        screen,
        assets.font(),
        localization.text(mode_key),
        10,
        0xFF_FF_FF,
    );
    if keyboard {
        crate::gfx::font::draw_centered(
            screen,
            assets.font(),
            localization.text("minicraft.displays.controls.display.keyboard.desc"),
            HEIGHT as i32 - 16,
            0x99_99_99,
        );
    } else {
        crate::gfx::font::draw_centered(
            screen,
            assets.font(),
            localization.text("minicraft.displays.controls.display.controller.desc.0"),
            HEIGHT as i32 - 24,
            0x99_99_99,
        );
        crate::gfx::font::draw_centered(
            screen,
            assets.font(),
            localization.text("minicraft.displays.controls.display.controller.desc.1"),
            HEIGHT as i32 - 16,
            0x99_99_99,
        );
    }
    crate::gfx::font::draw_centered(
        screen,
        assets.font(),
        &localization.format(
            "minicraft.displays.controls.display.help.0",
            &["LEFT", "RIGHT"],
        ),
        HEIGHT as i32 - 8,
        0x99_99_99,
    );
    if let Some((index, append)) = capture {
        let prompt = format!(
            "PRESS KEY{} (ESC cancel)",
            if append { " TO ADD" } else { "" }
        );
        let _ = index;
        screen.centered_text(assets.font(), &prompt, HEIGHT as i32 / 2);
    }
}

pub(super) fn render_play_menu(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world_count: usize,
    selection: usize,
) {
    let _ = world_count;
    let labels = [
        localization
            .text("minicraft.displays.title.play.load_world")
            .to_owned(),
        localization
            .text("minicraft.displays.title.play.new_world")
            .to_owned(),
    ];
    render_centered_menu(screen, assets, &labels, selection, 87, 10);
}

pub(super) fn render_new_world_menu(
    screen: &mut Screen,
    assets: &Assets,
    menu: &crate::screen::menu::Menu,
) {
    menu.render(screen, assets.hud(), assets.font());
}

pub(super) fn render_worlds_menu(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    settings: &Settings,
    worlds: &[WorldRecord],
    menu: &crate::screen::menu::Menu,
    popup: Option<&WorldPopup>,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.world_select.select_world"),
        0,
    );
    if worlds.is_empty() {
        screen.centered_text(assets.font(), "NO WORLDS FOUND", 92);
    } else {
        menu.render(screen, assets.hud(), assets.font());
        screen.centered_text(
            assets.font(),
            &localization.format(
                "minicraft.displays.world_select.display.world_version",
                &["2.2.4"],
            ),
            28,
        );
    }
    for (row, text) in [
        localization.format(
            "minicraft.displays.world_select.display.help.0",
            &[&settings.key_bindings.select],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.1",
            &[&settings.key_bindings.exit],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.2",
            &["SHIFT-C"],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.3",
            &["SHIFT-R"],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.4",
            &["SHIFT-D"],
        ),
    ]
    .iter()
    .enumerate()
    {
        let y = [
            HEIGHT as i32 - 60,
            HEIGHT as i32 - 40,
            HEIGHT as i32 - 24,
            HEIGHT as i32 - 16,
            HEIGHT as i32 - 8,
        ][row];
        screen.centered_text(assets.font(), text, y);
    }
    if let Some(dialog) = popup {
        render_world_popup(screen, assets, localization, worlds, menu.selection, dialog);
    }
}

pub(super) fn render_world_popup(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    worlds: &[WorldRecord],
    selection: usize,
    popup: &WorldPopup,
) {
    let (x, y, width, height) = (28, 56, WIDTH as i32 - 56, 80);
    screen.rect(x, y, width, height, 0x08080C);
    screen.frame(x, y, width, height, 0xC8C8C8);
    match popup.kind {
        WorldPopupKind::Copy | WorldPopupKind::Rename => {
            screen.centered_text(
                assets.font(),
                localization.text("minicraft.displays.world_select.popups.display.change"),
                y + 12,
            );
            screen.centered_text(assets.font(), &format!("{}{}", popup.text, "_"), y + 30);
        }
        WorldPopupKind::Delete => {
            screen.centered_text(assets.font(), "ARE YOU SURE YOU WANT TO DELETE", y + 12);
            screen.centered_text(
                assets.font(),
                worlds
                    .get(selection)
                    .map(|world| world.name.as_str())
                    .unwrap_or(""),
                y + 26,
            );
            screen.centered_text(assets.font(), "THIS CAN NOT BE UNDONE!", y + 40);
        }
    }
    screen.centered_text(assets.font(), "ENTER TO CONFIRM", y + height - 22);
    screen.centered_text(assets.font(), "ESC TO CANCEL", y + height - 12);
}

pub(super) fn render_book_menu(
    screen: &mut Screen,
    assets: &Assets,
    book: Book,
    page: usize,
    menu: &crate::screen::menu::Menu,
) {
    screen.frame(6, 6, WIDTH as i32 - 12, HEIGHT as i32 - 12, 0xC8C8C8);
    screen.centered_text(assets.font(), book.title(), 13);
    menu.render(screen, assets.hud(), assets.font());
    let pages = book.pages();
    let page = page.min(pages.len().saturating_sub(1));
    screen.centered_text(
        assets.font(),
        &format!("<  PAGE {} / {}  >", page + 1, pages.len()),
        HEIGHT as i32 - 18,
    );
}

pub(super) fn render_achievements(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    achievements: &[Achievement],
    unlocked: &HashSet<String>,
    bindings: &KeyBindings,
    selection: usize,
) {
    screen.centered_text(assets.font(), "Achievements", 8);
    let score: u32 = achievements
        .iter()
        .filter(|achievement| unlocked.contains(&achievement.id))
        .map(|achievement| achievement.score)
        .sum();
    let score_label = localization.format(
        "minicraft.displays.achievements.display.score",
        &[&score.to_string()],
    );
    screen.text_colored(
        assets.font(),
        &score_label,
        (WIDTH as i32 - crate::gfx::font::text_width(&score_label)) / 2,
        32,
        0x999999,
    );
    if let Some(achievement) = achievements.get(selection) {
        let achieved = unlocked.contains(&achievement.id);
        let status = localization.text(if achieved {
            "minicraft.displays.achievements.display.achieved"
        } else {
            "minicraft.displays.achievements.display.not_achieved"
        });
        screen.text_colored(
            assets.font(),
            status,
            (WIDTH as i32 - crate::gfx::font::text_width(&status)) / 2,
            48,
            if achieved { 0x00FF00 } else { 0xFF3030 },
        );
    }
    let first = selection
        .saturating_sub(2)
        .min(achievements.len().saturating_sub(5));
    for (row, achievement) in achievements.iter().skip(first).take(5).enumerate() {
        let index = first + row;
        let y = 72 + row as i32 * 10;
        let name = localization.text(&achievement.id);
        let x = (WIDTH as i32 - crate::gfx::font::text_width(&name)) / 2;
        if index == selection {
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(
                assets.font(),
                "<",
                x + crate::gfx::font::text_width(&name),
                y,
            );
        }
        screen.text_colored(
            assets.font(),
            name,
            x,
            y,
            if unlocked.contains(&achievement.id) {
                0x00FF00
            } else {
                0xFFFFFF
            },
        );
    }
    if let Some(achievement) = achievements.get(selection) {
        render_app_menu_frame(screen, assets.hud(), 44, 128, 200, 32);
        let description = localization.text(&achievement.description);
        for (row, line) in crate::progression::paginate(description, 23, 2)[0]
            .iter()
            .enumerate()
        {
            screen.text(assets.font(), line, 52, 136 + row as i32 * 10);
        }
    }
    let help = localization.format(
        "minicraft.displays.achievements.display.help",
        &[&bindings.down, &bindings.up],
    );
    screen.text_colored(
        assets.font(),
        &help,
        (WIDTH as i32 - crate::gfx::font::text_width(&help)) / 2,
        184,
        0x333333,
    );
}

#[allow(dead_code)]
pub(super) fn render_key_bindings(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    bindings: &KeyBindings,
    selection: usize,
    capture: Option<(usize, bool)>,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.key_input.title"),
        8,
    );
    // Java KeyInputDisplay: menu at Screen.h - textHeight*5; each row action + pad + mapping.
    let menu_y = HEIGHT as i32 - 8 * 5;
    for (index, label) in KeyBindings::LABELS.iter().enumerate() {
        let y = menu_y + index as i32 * 8;
        if index == selection {
            screen.text(assets.font(), ">", 6, y);
            screen.text(assets.font(), "<", WIDTH as i32 - 14, y);
        }
        let value = bindings.display_value(index);
        screen.text_colored(assets.font(), label, 16, y, 0xFF_FF_FF);
        let map_x = WIDTH as i32 - 17 - crate::gfx::font::text_width(&value);
        screen.text_colored(assets.font(), &value, map_x, y, 0x99_99_99);
    }
    let help = [
        localization
            .text("minicraft.displays.key_input.display.help.0")
            .to_owned(),
        localization
            .text("minicraft.displays.key_input.display.help.1")
            .to_owned(),
        localization
            .text("minicraft.displays.key_input.display.help.2")
            .to_owned(),
        localization.format(
            "minicraft.displays.key_input.display.help.3",
            &[&bindings.exit],
        ),
    ];
    for (line, y) in help.iter().enumerate().map(|(i, line)| (line, HEIGHT as i32 - 8 * (4 - i as i32))) {
        screen.text_colored(assets.font(), line, 8, y, 0x999999);
    }
    if let Some((_, _append)) = capture {
        let message =
            localization.text("minicraft.displays.key_input.popup_display.press_key_sequence");
        let width = crate::gfx::font::text_width(&message) + 16;
        let x = (WIDTH as i32 - width) / 2;
        render_app_menu_frame(screen, assets.hud(), x, 82, width, 24);
        screen.text_colored(assets.font(), message, x + 8, 90, 0xFFFF00);
    }
}

#[allow(dead_code)]
pub(super) fn render_control_guide(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    selection: usize,
    controller: bool,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.controls"),
        0,
    );
    screen.centered_text(
        assets.font(),
        localization.text(if controller {
            "minicraft.displays.controls.display.controller"
        } else {
            "minicraft.displays.controls.display.keyboard"
        }),
        10,
    );
    let count: usize = if controller { 16 } else { 23 };
    let first = selection.saturating_sub(8).min(count.saturating_sub(17));
    for (row, index) in (first..count).take(17).enumerate() {
        let key = format!(
            "minicraft.displays.controls.display.{}.{index:02}",
            if controller { "controller" } else { "keyboard" }
        );
        let label = localization.text(&key);
        let y = 20 + row as i32 * 8;
        if index == selection {
            let width = crate::gfx::font::text_width(&label);
            let x = (WIDTH as i32 - width) / 2;
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(assets.font(), "<", x + width, y);
        }
        screen.centered_text(assets.font(), label, y);
    }
    screen.centered_text(
        assets.font(),
        if controller {
            "CONTROLLER INPUTS"
        } else {
            "KEYBOARD INPUTS"
        },
        HEIGHT as i32 - 16,
    );
    screen.centered_text(assets.font(), "LEFT RIGHT TO SWITCH", HEIGHT as i32 - 8);
}

pub(super) fn render_centered_menu(
    screen: &mut Screen,
    assets: &Assets,
    labels: &[String],
    selection: usize,
    start_y: i32,
    spacing: i32,
) {
    for (index, label) in labels.iter().enumerate() {
        let y = start_y + index as i32 * spacing;
        if index == selection {
            let width = crate::gfx::font::text_width(&label);
            let x = (WIDTH as i32 - width) / 2;
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(assets.font(), "<", x + width, y);
        }
        let x = (WIDTH as i32 - crate::gfx::font::text_width(&label)) / 2;
        screen.text_colored(
            assets.font(),
            label,
            x,
            y,
            if index == selection {
                0xFFFFFF
            } else {
                0x999999
            },
        );
    }
}

pub(crate) fn render_focus_nagger(screen: &mut Screen, assets: &Assets, ticks: u64) {
    let message = "Click to focus!";
    let width = crate::gfx::font::text_width(&message) + 16;
    let height = 24;
    let x = (WIDTH as i32 - width) / 2;
    let y = (HEIGHT as i32 - height) / 2;
    render_app_menu_frame(screen, assets.hud(), x, y, width, height);
    screen.text_colored(
        assets.font(),
        message,
        x + 8,
        y + 8,
        if (ticks / 20) & 1 == 0 {
            0x999999
        } else {
            0xFFFFFF
        },
    );
}

pub(super) fn render_app_menu_frame(
    screen: &mut Screen,
    hud: &crate::gfx::Image,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let right = x + width - 8;
    let bottom = y + height - 8;
    for draw_y in (y..=bottom).step_by(8) {
        for draw_x in (x..=right).step_by(8) {
            let edge_x = draw_x == x || draw_x == right;
            let edge_y = draw_y == y || draw_y == bottom;
            let source_x = if edge_x && edge_y {
                0
            } else if edge_y {
                8
            } else if edge_x {
                16
            } else {
                24
            };
            screen.blit_region_transformed(
                hud,
                source_x,
                48,
                8,
                8,
                draw_x,
                draw_y,
                draw_x == right,
                draw_y == bottom,
            );
        }
    }
}

#[allow(dead_code)]
pub(super) fn render_languages_menu(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    current_locale: &str,
    selection: usize,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.language_settings.title"),
        8,
    );
    let order = sorted_locale_indices();
    let first = selection
        .saturating_sub(7)
        .min(order.len().saturating_sub(15));
    for (row, locale_index) in order.iter().skip(first).take(15).enumerate() {
        let index = first + row;
        let (code, name) = LOCALES[*locale_index];
        // Java LocaleInformation.toString() = "Name (Region)"; do not append the code.
        let label = name;
        let y = 30 + row as i32 * 10;
        if index == selection {
            screen.text(assets.font(), ">", 41, y);
            screen.text(assets.font(), "<", 239, y);
        }
        let color = if code.eq_ignore_ascii_case(current_locale) {
            if index == selection {
                crate::screen::entry::COL_GREEN
            } else {
                0x66_99_66
            }
        } else if index == selection {
            0xFF_FF_FF
        } else {
            0x99_99_99
        };
        screen.text_colored(assets.font(), label, 53, y, color);
    }
}

pub(super) fn sorted_locale_indices() -> Vec<usize> {
    crate::resource::localization::sorted_locale_indices()
}

pub(super) fn render_skins_menu(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    skins: &[String],
    selection: usize,
    ticks: u64,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.skin"),
        16,
    );
    // Java SkinDisplay: step/40 % 8 walk cycle; select_skin already set skin_row.
    let (source_x, flip_x) = skin_preview_frame(ticks);
    screen.blit_region(
        assets.skin(),
        WIDTH as i32 / 2 - 8,
        40,
        source_x,
        assets.skin_row,
        16,
        16,
        flip_x,
    );
    let menu = crate::screen::menu::MenuBuilder::new(false, 2, crate::screen::rel_pos::RelPos::CENTER, {
        use crate::screen::entry::Entry;
        skins
            .iter()
            .map(|skin| {
                let name = if skin.starts_with("minicraft.skin.") {
                    localization.text(skin).to_owned()
                } else {
                    skin.clone()
                };
                Entry::select(name, false)
            })
            .collect()
    })
    .set_display_length(8)
    .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), crate::screen::rel_pos::RelPos::CENTER)
    .set_selection(selection as i32)
    .create_menu();
    menu.render(screen, assets.hud(), assets.font());
    screen.centered_text(
        assets.font(),
        &localization.format(
            "minicraft.displays.skin.display.help.move",
            &["DOWN", "UP"],
        ),
        HEIGHT as i32 - 17,
    );
    screen.centered_text(
        assets.font(),
        &localization.format(
            "minicraft.displays.skin.display.help.select",
            &["SELECT", "EXIT"],
        ),
        HEIGHT as i32 - 9,
    );
}

/// Java compileMobSpriteAnimations + SkinDisplay:
/// spriteIndex = step/40 % 8 → [dir][frame].
/// set1 = sheetX 0,2,4,6 (down1, up1, right1, right2); set2 = same with mirror=1.
/// sprites[0]=down set2/set1, [1]=up set2/set1, [2]=left set2, [3]=right set1.
pub(super) fn skin_preview_frame(ticks: u64) -> (usize, bool) {
    let sprite_index = (ticks / 40 % 8) as usize;
    let direction = sprite_index / 2;
    let frame = sprite_index % 2;
    match direction {
        // Down: set1[0]=x0, set2[0]=x0 mirror
        0 => (0, frame == 1),
        // Up: set1[1]=x16, set2[1]=x16 mirror
        1 => (16, frame == 1),
        // Left: set2[2]=x32 mirror, set2[3]=x48 mirror
        2 => (32 + frame * 16, true),
        // Right: set1[2]=x32, set1[3]=x48
        _ => (32 + frame * 16, false),
    }
}

