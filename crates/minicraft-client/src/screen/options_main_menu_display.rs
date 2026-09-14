//! Java `OptionsMainMenuDisplay.java` — main-menu settings list.
use crate::gfx::{HEIGHT, WIDTH};
use crate::io::Settings;
use crate::resource::localization::Localization;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Entry indices matching Java's fixed Settings.getEntry + SelectEntry order.
pub mod indices {
    pub const FPS: usize = 0;
    pub const SOUND: usize = 1;
    pub const SHOW_QUESTS: usize = 2;
    pub const HWA: usize = 3;
    pub const KEY_BINDINGS: usize = 4;
    pub const CONTROLS: usize = 5;
    pub const LANGUAGE: usize = 6;
    pub const RESOURCE_PACKS: usize = 7;
}

/// Java FPS ArrayEntry range (Settings "fps").
pub const FPS_MIN: i32 = 10;
pub const FPS_MAX: i32 = 300;

/// Java OptionsMainMenuDisplay: `Builder(false, 6, LEFT, …).setTitle(options_main_menu)`.
pub fn build_menu(
    localization: &Localization,
    settings: &Settings,
    selection: usize,
) -> Menu {
    let entries = vec![
        Entry::range(
            localization.text("minicraft.settings.fps"),
            FPS_MIN,
            FPS_MAX,
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

/// Sync Array/Boolean options back into settings. Returns true when changed.
pub fn apply_menu_to_settings(menu: &Menu, settings: &mut Settings) -> bool {
    use super::entry::Entry;
    let mut changed = false;
    if let Some(Entry::Boolean { value, .. }) = menu.entries.get(indices::SOUND)
        && *value != settings.sound
    {
        settings.sound = *value;
        changed = true;
    }
    if let Some(Entry::Boolean { value, .. }) = menu.entries.get(indices::SHOW_QUESTS)
        && *value != settings.show_quests
    {
        settings.show_quests = *value;
        changed = true;
    }
    if let Some(Entry::Boolean { value, .. }) = menu.entries.get(indices::HWA)
        && *value != settings.hwa
    {
        settings.hwa = *value;
        changed = true;
    }
    if let Some(Entry::Array {
        selection, options, ..
    }) = menu.entries.get(indices::FPS)
        && let Some(fps) = options
            .get(*selection)
            .and_then(|s| s.parse::<usize>().ok())
        && fps != settings.fps
    {
        settings.fps = fps;
        changed = true;
    }
    changed
}

/// Java SelectEntry destinations for the bottom four rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsAction {
    KeyBindings,
    Controls,
    Language,
    ResourcePacks,
}

pub fn action_for_selection(selection: usize) -> Option<OptionsAction> {
    match selection {
        indices::KEY_BINDINGS => Some(OptionsAction::KeyBindings),
        indices::CONTROLS => Some(OptionsAction::Controls),
        indices::LANGUAGE => Some(OptionsAction::Language),
        indices::RESOURCE_PACKS => Some(OptionsAction::ResourcePacks),
        _ => None,
    }
}

/// Force FPS Array selection by ±10 (Java RangeEntry / host step).
pub fn step_fps(menu: &mut Menu, left: bool) -> bool {
    use super::entry::Entry;
    let Some(Entry::Array {
        options, selection, ..
    }) = menu.entries.get_mut(indices::FPS)
    else {
        return false;
    };
    let step = 10;
    let len = options.len() as i32;
    let mut next = *selection as i32;
    next += if left { -step } else { step };
    next = next.clamp(0, len - 1);
    if next as usize != *selection {
        *selection = next as usize;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_menu_matches_java_spacing_left_and_title() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, &Settings::default(), 0);
        assert_eq!(menu.spacing, 6);
        assert_eq!(menu.entry_pos, RelPos::LEFT);
        assert_eq!(menu.entries.len(), 8);
        assert!(!menu.title.is_empty());
        assert_eq!(menu.selection, 0);
    }

    #[test]
    fn option_array_entries_ignore_select_like_java() {
        let mut settings = Settings::default();
        let sound = settings.sound;
        let show_quests = settings.show_quests;
        let hwa = settings.hwa;
        for selection in 1..=3 {
            let mut menu = build_menu(
                &Localization::load("en-us", &[]),
                &settings,
                selection,
            );
            // Select on boolean/array rows must not change values by itself.
            let _ = menu.tick(&super::super::menu::MenuInput {
                select: true,
                ..Default::default()
            });
            let _ = apply_menu_to_settings(&menu, &mut settings);
        }
        assert_eq!(
            (settings.sound, settings.show_quests, settings.hwa),
            (sound, show_quests, hwa)
        );
    }

    #[test]
    fn select_actions_map_to_java_destinations() {
        assert_eq!(
            action_for_selection(indices::KEY_BINDINGS),
            Some(OptionsAction::KeyBindings)
        );
        assert_eq!(
            action_for_selection(indices::CONTROLS),
            Some(OptionsAction::Controls)
        );
        assert_eq!(
            action_for_selection(indices::LANGUAGE),
            Some(OptionsAction::Language)
        );
        assert_eq!(
            action_for_selection(indices::RESOURCE_PACKS),
            Some(OptionsAction::ResourcePacks)
        );
        assert_eq!(action_for_selection(indices::FPS), None);
    }

    #[test]
    fn fps_step_is_ten_like_java_range() {
        let localization = Localization::load("en-us", &[]);
        let mut menu = build_menu(&localization, &Settings::default(), indices::FPS);
        // default fps is typically 60 → option index 50; stepping right goes +10
        let before = match &menu.entries[indices::FPS] {
            Entry::Array { selection, .. } => *selection,
            _ => panic!("fps is array"),
        };
        assert!(step_fps(&mut menu, false));
        let after = match &menu.entries[indices::FPS] {
            Entry::Array { selection, .. } => *selection,
            _ => panic!("fps is array"),
        };
        assert_eq!(after, before + 10);
    }
}
