//! Compatibility re-exports of Java-matching `screen::*` menu builders.
//!
//! Concrete layout/scroll logic lives in `crate::screen::{title_display,
//! world_gen_display, …}`. This module keeps the historical `menu_build`
//! names used by the State host and previews.
use std::collections::HashSet;

use crate::{
    io::{Input, Settings},
    progression::{Achievement, Book},
    resource::localization::Localization,
    resource::pack::ResourcePack,
    worlds::WorldRecord,
};

pub(super) use crate::screen::title_display::TITLE_KEYS;

/// Java TitleDisplay menu: checking line + blank + 6 selects, spacing 2, CENTER @ h*3/5.
#[cfg(test)]
pub(super) fn build_title_menu(
    localization: &Localization,
    world_count: usize,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::title_display::build_menu(localization, world_count, selection)
}

/// Java Play menu when worlds exist: Load / New.
pub(super) fn build_play_menu(
    localization: &Localization,
    has_worlds: bool,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::play_display::build_menu(localization, has_worlds, selection)
}

/// Java ControlGuideDisplay: selectable help lines, left/right flips device.
pub(super) fn build_control_guide_menu(
    localization: &Localization,
    controller: bool,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::controls_display::build_menu(localization, !controller, selection)
}

/// Java BookDisplay: one Menu of StringEntry lines for the current page.
pub(super) fn build_book_menu(book: Book, page: usize) -> crate::screen::menu::Menu {
    let pages = book.pages();
    let page = page.min(pages.len().saturating_sub(1));
    let lines = pages.get(page).cloned().unwrap_or_default();
    crate::screen::book::build_page_menu(&lines)
}

/// Java Help submenu: BlankEntry + 4 Selects, spacing 1, CENTER, title.
pub(super) fn build_help_menu(
    localization: &Localization,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::help_display::build_menu(localization, selection)
}

/// Java AchievementsDisplay list menu (48×48 CENTER box).
pub(super) fn build_achievements_menu(
    localization: &Localization,
    achievements: &[Achievement],
    unlocked: &HashSet<String>,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::achievements_display::build_list_menu(
        localization,
        achievements,
        unlocked,
        selection,
    )
}

/// Java SkinDisplay: Select list, displayLength 8, CENTER @ h*3/5.
pub(super) fn build_skins_menu(skins: &[String], selection: usize) -> crate::screen::menu::Menu {
    crate::screen::skin_display::build_menu(skins, selection)
}

/// Java ResourcePackDisplay: indicators at (20,60) BOTTOM_RIGHT, packs at (20,60) BOTTOM_LEFT.
pub(super) fn build_resource_packs_menu(
    packs: &[ResourcePack],
    enabled: &[String],
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::resource_pack_display::build_menu(packs, enabled, selection)
}

/// Java ResourcePackDisplay toggle indicators column (✓ / empty).
#[allow(dead_code)]
pub(super) fn build_resource_pack_indicators(
    packs: &[ResourcePack],
    enabled: &[String],
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::resource_pack_display::build_indicator_menu(packs, enabled, selection)
}

/// Java OptionsMainMenuDisplay: Menu.Builder(false, 6, LEFT, …).setTitle(...).
pub(super) fn build_options_menu(
    localization: &Localization,
    settings: &Settings,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::options_main_menu_display::build_menu(localization, settings, selection)
}

/// Java LanguageSettingsDisplay: sorted by "Name (region)", BOTTOM @ y=10.
pub(super) fn build_languages_menu(
    localization: &Localization,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::language_settings_display::build_menu(localization, selection)
}

/// Java ControlsDisplay: selectable string help lines, displayLength 17.
pub(super) fn build_controls_menu(
    localization: &Localization,
    keyboard: bool,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::controls_display::build_menu(localization, keyboard, selection)
}

/// Java WorldSelectDisplay: Menu.Builder(false, 0, CENTER).setDisplayLength(5).setScrollPolicies(1, true).
pub(super) fn build_worlds_menu(
    worlds: &[WorldRecord],
    localization: &Localization,
    selection: usize,
) -> crate::screen::menu::Menu {
    crate::screen::world_select_display::build_menu(worlds, localization, selection)
}

/// Java WorldGenDisplay form as Entry list (spacing 10, LEFT-aligned entries).
pub(super) fn build_new_world_menu(
    localization: &Localization,
    settings: &Settings,
    name: &str,
    seed: &str,
    selection: usize,
    world_count: usize,
) -> crate::screen::menu::Menu {
    crate::screen::world_gen_display::build_menu(
        localization,
        settings,
        name,
        seed,
        selection,
        world_count,
    )
}

pub(super) fn menu_input_from(input: &Input) -> crate::screen::menu::MenuInput {
    // Java menus use cursor-up/down edges only (WASD is move, not menu).
    crate::screen::menu::MenuInput {
        up: input.cursor_up_pressed,
        down: input.cursor_down_pressed,
        left: input.cursor_left_pressed,
        right: input.cursor_right_pressed,
        select: input.select,
        exit: input.exit,
        shift_up: input.shift && input.cursor_up_pressed,
        shift_down: input.shift && input.cursor_down_pressed,
        page_up: input.page_up,
        page_down: input.page_down,
        search: input.search,
        alt: input.alt,
        backspace: input.backspace,
        typed_char: input.text.first().copied(),
        typed_text: input.text.iter().collect(),
    }
}
