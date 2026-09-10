//! Top-level menu builders shared by the compatibility State host and previews.
use std::collections::HashSet;

use crate::{
    io::Settings,
    progression::{Achievement, Book},
    gfx::{HEIGHT, WIDTH},
    io::Input,
    resource::localization::{LOCALES, Localization},
    resource::pack::ResourcePack,
    world::GameMode,
    worlds::WorldRecord,
};

pub(super) const TITLE_KEYS: [&str; 6] = [
    "minicraft.displays.title.play",
    "minicraft.display.options_display",
    "minicraft.displays.skin",
    "minicraft.displays.achievements",
    "minicraft.displays.title.help",
    "minicraft.displays.title.quit",
];

/// Java TitleDisplay menu: checking line + blank + 6 selects, spacing 2, CENTER @ h*3/5.
pub(super) fn build_title_menu(
    localization: &Localization,
    world_count: usize,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let mut entries = vec![
        Entry::string_colored(
            localization.text("minicraft.displays.title.display.latest_already"),
            0x33_33_33,
        ),
        Entry::blank(),
    ];
    for key in TITLE_KEYS {
        entries.push(Entry::select(localization.text(key), false));
    }
    let _ = world_count;
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java Play menu when worlds exist: Load / New.
pub(super) fn build_play_menu(
    localization: &Localization,
    has_worlds: bool,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let entries = if has_worlds {
        vec![
            Entry::select(
                localization.text("minicraft.displays.title.play.load_world"),
                false,
            ),
            Entry::select(
                localization.text("minicraft.displays.title.play.new_world"),
                false,
            ),
        ]
    } else {
        vec![Entry::select(
            localization.text("minicraft.displays.title.play.new_world"),
            false,
        )]
    };
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java ControlGuideDisplay: selectable help lines, left/right flips device.
pub(super) fn build_control_guide_menu(
    localization: &Localization,
    controller: bool,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let count = if controller { 16 } else { 23 };
    let mode = if controller { "controller" } else { "keyboard" };
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let key = format!("minicraft.displays.controls.display.{mode}.{i:02}");
        entries.push(Entry::select(localization.text(&key), false));
    }
    MenuBuilder::new(false, 0, RelPos::CENTER, entries)
        .set_display_length(17)
        .set_positioning((WIDTH as i32 / 2, 20), RelPos::BOTTOM)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java BookDisplay: one Menu of StringEntry lines for the current page.
pub(super) fn build_book_menu(book: Book, page: usize) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let pages = book.pages();
    let page = page.min(pages.len().saturating_sub(1));
    let entries: Vec<Entry> = pages[page]
        .iter()
        .map(|line| Entry::string(line.clone()))
        .collect();
    MenuBuilder::new(true, 3, RelPos::CENTER, entries)
        .set_size(WIDTH as i32 - 30, HEIGHT as i32 - 80)
        .set_positioning((WIDTH as i32 / 2, 40), RelPos::CENTER)
        .create_menu()
}

/// Java Help submenu: BlankEntry + 4 Selects, spacing 1, CENTER, title.
pub(super) fn build_help_menu(
    localization: &Localization,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let keys = [
        "minicraft.displays.title.help.instructions",
        "minicraft.displays.title.help.storyline_guide",
        "minicraft.displays.title.help.about",
        "minicraft.displays.title.help.credits",
    ];
    let mut entries = vec![Entry::blank()];
    entries.extend(
        keys.iter()
            .map(|key| Entry::select(localization.text(key), false)),
    );
    MenuBuilder::new(false, 1, RelPos::CENTER, entries)
        .set_title(localization.text("minicraft.displays.title.help"))
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 / 2), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java AchievementsDisplay list menu (48×48 CENTER box).
pub(super) fn build_achievements_menu(
    localization: &Localization,
    achievements: &[Achievement],
    unlocked: &HashSet<String>,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    // Java SelectEntry(id) with unlocked → GREEN else GRAY; selectable.
    let entries: Vec<Entry> = achievements
        .iter()
        .map(|achievement| {
            let color = if unlocked.contains(&achievement.id) {
                crate::screen::entry::COL_GREEN
            } else {
                crate::screen::entry::COL_UNSLCT
            };
            Entry::select_colored(localization.text(&achievement.id), color)
        })
        .collect();
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_size(48, 48)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java SkinDisplay: Select list, displayLength 8, CENTER @ h*3/5.
pub(super) fn build_skins_menu(skins: &[String], selection: usize) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let entries: Vec<Entry> = skins
        .iter()
        .map(|skin| Entry::select(skin.clone(), false))
        .collect();
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_display_length(8)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java ResourcePackDisplay: indicators at (20,60) BOTTOM_RIGHT, packs at (20,60) BOTTOM_LEFT.
pub(super) fn build_resource_packs_menu(
    packs: &[ResourcePack],
    enabled: &[String],
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let entries: Vec<Entry> = packs
        .iter()
        .map(|pack| Entry::select(pack.name.clone(), false))
        .collect();
    let _ = enabled;
    MenuBuilder::new(false, 2, RelPos::LEFT, entries)
        .set_positioning((20, 60), RelPos::BOTTOM_LEFT)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java ResourcePackDisplay toggle indicators column (✓ / empty).
#[allow(dead_code)]
pub(super) fn build_resource_pack_indicators(
    packs: &[ResourcePack],
    enabled: &[String],
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let entries: Vec<Entry> = packs
        .iter()
        .map(|pack| {
            let mark = if enabled.iter().any(|id| id == &pack.id) {
                "✓"
            } else {
                ""
            };
            Entry::select(mark, false)
        })
        .collect();
    MenuBuilder::new(false, 2, RelPos::LEFT, entries)
        .set_positioning((20, 60), RelPos::BOTTOM_RIGHT)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java OptionsMainMenuDisplay: Menu.Builder(false, 6, LEFT, …).setTitle(...).
pub(super) fn build_options_menu(
    localization: &Localization,
    settings: &Settings,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let entries = vec![
        Entry::range(
            localization.text("minicraft.settings.fps"),
            10,
            300,
            settings.fps as i32,
        ),
        Entry::boolean(
            localization.text("minicraft.settings.sound"),
            settings.sound,
        ),
        Entry::boolean(
            localization.text("minicraft.settings.show_quests"),
            settings.show_quests,
        ),
        Entry::boolean(
            localization.text("minicraft.settings.opengl_hwa"),
            settings.hwa,
        ),
        Entry::select(
            localization.text("minicraft.display.options_display.change_key_bindings"),
            false,
        ),
        Entry::select(localization.text("minicraft.displays.controls"), false),
        Entry::select(
            localization.text("minicraft.display.options_display.language"),
            false,
        ),
        Entry::select(
            localization.text("minicraft.display.options_display.resource_packs"),
            false,
        ),
    ];
    MenuBuilder::new(false, 6, RelPos::LEFT, entries)
        .set_title(localization.text("minicraft.displays.options_main_menu"))
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 / 2), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java LanguageSettingsDisplay: sorted by "Name (region)", BOTTOM @ y=10.
pub(super) fn build_languages_menu(
    localization: &Localization,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let order = {
        let mut indices: Vec<usize> = (0..LOCALES.len()).collect();
        indices.sort_by(|left, right| {
            let a = format!("{} ({})", LOCALES[*left].1, LOCALES[*left].0);
            let b = format!("{} ({})", LOCALES[*right].1, LOCALES[*right].0);
            a.to_lowercase().cmp(&b.to_lowercase())
        });
        indices
    };
    let entries: Vec<Entry> = order
        .iter()
        .map(|index| {
            let (code, name) = LOCALES[*index];
            Entry::select(format!("{name} ({code})"), false)
        })
        .collect();
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_title(localization.text("minicraft.displays.language_settings.title"))
        .set_positioning((WIDTH as i32 / 2, 10), RelPos::BOTTOM)
        .set_size(WIDTH as i32, HEIGHT as i32 - 30)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java ControlsDisplay: selectable string help lines, displayLength 17.
pub(super) fn build_controls_menu(
    localization: &Localization,
    keyboard: bool,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let prefix = if keyboard {
        "minicraft.displays.controls.display.keyboard"
    } else {
        "minicraft.displays.controls.display.controller"
    };
    let count = if keyboard { 23 } else { 16 };
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let key = format!("{prefix}.{i:02}");
        entries.push(Entry::select(localization.text(&key), false));
    }
    MenuBuilder::new(false, 0, RelPos::CENTER, entries)
        .set_display_length(17)
        .set_positioning((WIDTH as i32 / 2, 20), RelPos::BOTTOM)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java WorldSelectDisplay: Menu.Builder(false, 0, CENTER).setDisplayLength(5).setScrollPolicies(1, true).
pub(super) fn build_worlds_menu(
    worlds: &[WorldRecord],
    localization: &Localization,
    selection: usize,
) -> crate::screen::menu::Menu {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let _ = localization;
    let entries: Vec<Entry> = if worlds.is_empty() {
        vec![Entry::string("NO WORLDS FOUND")]
    } else {
        worlds
            .iter()
            .map(|world| Entry::select(world.name.clone(), false))
            .collect()
    };
    let mut builder = MenuBuilder::new(false, 0, RelPos::CENTER, entries)
        .set_display_length(5)
        .set_scroll_policies(1.0, true)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 / 2), RelPos::CENTER);
    if !worlds.is_empty() {
        builder = builder.set_selection(selection.min(worlds.len().saturating_sub(1)) as i32);
    }
    builder.create_menu()
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
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    const MODE_KEYS: [&str; 4] = [
        "minicraft.settings.mode.survival",
        "minicraft.settings.mode.creative",
        "minicraft.settings.mode.hardcore",
        "minicraft.settings.mode.score",
    ];
    const THEME_KEYS: [&str; 5] = [
        "minicraft.settings.theme.normal",
        "minicraft.settings.theme.forest",
        "minicraft.settings.theme.desert",
        "minicraft.settings.theme.plain",
        "minicraft.settings.theme.hell",
    ];
    const TYPE_KEYS: [&str; 4] = [
        "minicraft.settings.type.island",
        "minicraft.settings.type.box",
        "minicraft.settings.type.mountain",
        "minicraft.settings.type.irregular",
    ];
    let score_mode = GameMode::from_index(settings.game_mode) == GameMode::Score;
    let mut name_field = Entry::input(
        localization.text("minicraft.displays.world_gen.enter_world"),
        36,
    );
    // Java InputEntry worldNameRegex: empty is invalid (red) until typed.
    if let Entry::Input { pattern, .. } = &mut name_field {
        *pattern = Some("[^<>:\"/\\\\|?*\\x00-\\x1F]+".to_owned());
    }
    let mut entries = vec![
        name_field,
        Entry::array(
            localization.text("minicraft.settings.mode"),
            MODE_KEYS
                .iter()
                .map(|key| localization.text(key).to_owned())
                .collect(),
            settings.game_mode.min(3),
            true,
        ),
    ];
    // Java always includes scoretime and hides it via ListEntry.setVisible when mode != score.
    let mut scoretime = Entry::range(
        localization.text("minicraft.settings.scoretime"),
        10,
        120,
        settings.score_minutes as i32,
    );
    scoretime.set_hidden(!score_mode);
    entries.push(scoretime);
    entries.push(Entry::select_colored(
        localization.text("minicraft.displays.world_gen.create_world"),
        crate::screen::entry::COL_CYAN,
    ));
    entries.push(Entry::array(
        localization.text("minicraft.settings.size"),
        ["128", "256", "512"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        [128, 256, 512]
            .iter()
            .position(|size| *size == settings.world_size)
            .unwrap_or(0),
        true,
    ));
    entries.push(Entry::array(
        localization.text("minicraft.settings.theme"),
        THEME_KEYS
            .iter()
            .map(|key| localization.text(key).to_owned())
            .collect(),
        settings.theme.min(4),
        true,
    ));
    entries.push(Entry::array(
        localization.text("minicraft.settings.type"),
        TYPE_KEYS
            .iter()
            .map(|key| localization.text(key).to_owned())
            .collect(),
        settings.terrain_type.min(3),
        true,
    ));
    entries.push(Entry::boolean(
        localization.text("minicraft.settings.quests"),
        settings.quests,
    ));
    entries.push(Entry::boolean(
        localization.text("minicraft.settings.tutorials"),
        settings.tutorials,
    ));
    // Java seed InputEntry.isValid always returns true (green when selected).
    let mut seed_field = Entry::input(
        localization.text("minicraft.displays.world_gen.world_seed"),
        20,
    );
    if let Entry::Input { pattern, .. } = &mut seed_field {
        *pattern = Some(String::new());
    }
    entries.push(seed_field);
    if let Some(Entry::Input { user_input, .. }) = entries.first_mut() {
        *user_input = name.to_owned();
    }
    if let Some(Entry::Input { user_input, .. }) = entries.last_mut() {
        *user_input = seed.to_owned();
    }
    let _ = world_count;
    // Java WorldGenDisplay: Builder(false, 10, LEFT, …).setDisplayLength(5)
    // .setScrollPolicies(0.8f, false).setTitle(...); default CENTER anchor.
    MenuBuilder::new(false, 10, RelPos::LEFT, entries)
        .set_display_length(5)
        .set_scroll_policies(0.8, false)
        .set_title(localization.text("minicraft.displays.world_gen.title"))
        .set_selection(selection as i32)
        .create_menu()
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
        page_up: false,
        page_down: false,
        search: false,
        alt: input.alt,
        backspace: input.backspace,
        typed_char: input.text.first().copied(),
        typed_text: input.text.iter().collect(),
    }
}

