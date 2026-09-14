//! Java `WorldGenDisplay.java` — world creation form, seed hash, name rules.
use crate::io::Settings;
use crate::resource::localization::Localization;
use crate::world::GameMode;

use super::entry::{COL_CYAN, Entry};
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java WorldGenDisplay world name regex (Windows / POSIX).
pub const WORLD_NAME_REGEX_WINDOWS: &str = "[^<>:\"/\\\\|?*\\x00-\\x1F]+";
pub const WORLD_NAME_REGEX_MAC: &str = "[^/:]+";
pub const WORLD_NAME_REGEX_POSIX: &str = "[^/\0]+";

/// Java worldSeed InputEntry max length.
pub const SEED_MAX_LEN: usize = 20;
/// Java world name InputEntry max length.
pub const NAME_MAX_LEN: usize = 36;

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
const WORLD_SIZES: [usize; 3] = [128, 256, 512];

/// Entry indices inside the WorldGen form (nameHelp is hidden in Rust host).
pub mod indices {
    pub const NAME: usize = 0;
    pub const MODE: usize = 1;
    pub const SCORETIME: usize = 2;
    pub const CREATE: usize = 3;
    pub const SIZE: usize = 4;
    pub const THEME: usize = 5;
    pub const TYPE: usize = 6;
    pub const QUESTS: usize = 7;
    pub const TUTORIALS: usize = 8;
    pub const SEED: usize = 9;
}

/// Java WorldGenDisplay: `Builder(false, 10, LEFT, …).setDisplayLength(5)
/// .setScrollPolicies(0.8f, false).setTitle(…)`.
pub fn build_menu(
    localization: &Localization,
    settings: &Settings,
    name: &str,
    seed: &str,
    selection: usize,
    _world_count: usize,
) -> Menu {
    let score_mode = GameMode::from_index(settings.game_mode) == GameMode::Score;
    let mut name_field = Entry::input(
        localization.text("minicraft.displays.world_gen.enter_world"),
        NAME_MAX_LEN,
    );
    // Java InputEntry worldNameRegex: empty is invalid (red) until typed.
    if let Entry::Input { pattern, .. } = &mut name_field {
        *pattern = Some(WORLD_NAME_REGEX_WINDOWS.to_owned());
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
    // Java Create World SelectEntry always draws Color.CYAN.
    entries.push(Entry::select_colored(
        localization.text("minicraft.displays.world_gen.create_world"),
        COL_CYAN,
    ));
    entries.push(Entry::array(
        localization.text("minicraft.settings.size"),
        WORLD_SIZES.iter().map(|s| s.to_string()).collect(),
        WORLD_SIZES
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
        SEED_MAX_LEN,
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
    MenuBuilder::new(false, 10, RelPos::LEFT, entries)
        .set_display_length(5)
        .set_scroll_policies(0.8, false)
        .set_title(localization.text("minicraft.displays.world_gen.title"))
        .set_selection(selection as i32)
        .create_menu()
}

/// Java WorldGenDisplay.getSeed: empty → None; pure digits (<20) parse; else
/// Java String.hashCode-style `seed = 31*seed + char` starting at 1125899906842597.
pub fn parse_world_seed(value: &str) -> Option<i64> {
    if value.is_empty() {
        return None;
    }
    if value.len() < 20
        && value
            .strip_prefix('-')
            .unwrap_or(value)
            .bytes()
            .all(|byte| byte.is_ascii_digit())
    {
        return value.parse().ok();
    }
    let mut seed = 1_125_899_906_842_597_i64;
    for character in value.encode_utf16() {
        seed = seed.wrapping_mul(31).wrapping_add(i64::from(character));
    }
    Some(seed)
}

/// Sync name/seed Input fields and Array/Boolean entries into settings.
/// Returns true when any setting changed. Layout matches `indices` above.
pub fn apply_menu_to_settings(
    menu: &mut Menu,
    settings: &mut Settings,
    name: &mut String,
    seed: &mut String,
) -> bool {
    let mut changed = false;
    if let Some(Entry::Input { user_input, .. }) = menu.entries.get(indices::NAME) {
        if user_input != name {
            *name = user_input.clone();
        }
    }
    if let Some(Entry::Input { user_input, .. }) = menu.entries.get(indices::SEED) {
        if user_input != seed {
            *seed = user_input.clone();
        }
    }
    if let Some(Entry::Array { selection, .. }) = menu.entries.get(indices::MODE)
        && *selection != settings.game_mode
    {
        settings.game_mode = *selection;
        changed = true;
    }
    // Java always keeps the scoretime ArrayEntry in the list (hidden unless score mode).
    let score_mode_now = matches!(GameMode::from_index(settings.game_mode), GameMode::Score);
    if let Some(Entry::Array {
        selection,
        options,
        hidden,
        ..
    }) = menu.entries.get_mut(indices::SCORETIME)
    {
        *hidden = !score_mode_now;
        if score_mode_now
            && let Some(value) = options
                .get(*selection)
                .and_then(|s| s.parse::<usize>().ok())
            && value != settings.score_minutes
        {
            settings.score_minutes = value;
            changed = true;
        }
    }
    if let Some(Entry::Array {
        selection, options, ..
    }) = menu.entries.get(indices::SIZE)
    {
        if let Some(value) = options
            .get(*selection)
            .and_then(|s| s.parse::<usize>().ok())
            && value != settings.world_size
        {
            settings.world_size = value;
            changed = true;
        }
    }
    if let Some(Entry::Array { selection, .. }) = menu.entries.get(indices::THEME)
        && *selection != settings.theme
    {
        settings.theme = *selection;
        changed = true;
    }
    if let Some(Entry::Array { selection, .. }) = menu.entries.get(indices::TYPE)
        && *selection != settings.terrain_type
    {
        settings.terrain_type = *selection;
        changed = true;
    }
    if let Some(Entry::Boolean { value, .. }) = menu.entries.get(indices::QUESTS)
        && *value != settings.quests
    {
        settings.quests = *value;
        changed = true;
    }
    if let Some(Entry::Boolean { value, .. }) = menu.entries.get(indices::TUTORIALS)
        && *value != settings.tutorials
    {
        settings.tutorials = *value;
        changed = true;
    }
    changed
}

/// Java Create World is the only SelectEntry; activation with a non-empty name
/// requests world creation.
pub fn create_requested(menu: &Menu, name: &str) -> Option<String> {
    let create_index = menu
        .entries
        .iter()
        .position(|entry| matches!(entry, Entry::Select { .. }))?;
    if menu.selection == create_index && !name.trim().is_empty() {
        Some(name.trim().to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_seed_input_matches_java_numeric_and_string_rules() {
        assert_eq!(parse_world_seed(""), None);
        assert_eq!(parse_world_seed("-42"), Some(-42));
        assert_eq!(parse_world_seed("abc"), Some(-3_351_804_022_671_199_651));
    }

    #[test]
    fn world_gen_menu_layout_matches_java_scroll_and_title() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, &Settings::default(), "", "", 0, 0);
        assert_eq!(menu.display_length, 5);
        assert_eq!(menu.spacing, 10);
        assert_eq!(menu.entry_pos, RelPos::LEFT);
        assert!(!menu.wrap);
        // padding = floor(0.8 * 5 / 2) = 2
        assert_eq!(menu.padding, 2);
        let (x, y, w, h) = menu.bounds;
        assert!(w > 0 && h > 0);
        assert!(
            x >= 0
                && y >= 0
                && x + w <= crate::gfx::WIDTH as i32
                && y + h <= crate::gfx::HEIGHT as i32
        );
        assert_eq!(menu.entries.len(), indices::SEED + 1);
        // Create World is CYAN.
        assert_eq!(menu.entries[indices::CREATE].color(true), COL_CYAN);
        // Scoretime starts hidden in non-score mode but still occupies a row.
        assert!(!menu.entries[indices::SCORETIME].is_selectable());
    }

    #[test]
    fn create_requested_requires_select_entry_and_non_empty_name() {
        let localization = Localization::load("en-us", &[]);
        let mut menu = build_menu(&localization, &Settings::default(), "World", "", indices::CREATE, 0);
        assert_eq!(
            create_requested(&menu, "World"),
            Some("World".to_owned())
        );
        menu.selection = indices::NAME;
        assert_eq!(create_requested(&menu, "World"), None);
        menu.selection = indices::CREATE;
        assert_eq!(create_requested(&menu, "  "), None);
    }

    #[test]
    fn apply_menu_syncs_name_seed_and_mode() {
        let localization = Localization::load("en-us", &[]);
        let mut settings = Settings::default();
        let mut menu = build_menu(&localization, &settings, "abc", "seed", 0, 0);
        let mut name = String::new();
        let mut seed = String::new();
        let changed = apply_menu_to_settings(&mut menu, &mut settings, &mut name, &mut seed);
        assert!(!changed);
        assert_eq!(name, "abc");
        assert_eq!(seed, "seed");
    }
}
