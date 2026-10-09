//! Java `TitleDisplay.java` — title menu layout, splash, and selection mapping.
//!
//! Menu structure matches `Menu.Builder(false, 2, RelPos.CENTER, …)` with the
//! checking line + blank + six SelectEntry rows at `(w/2, h*3/5)`.
use crate::gfx::{HEIGHT, WIDTH};
use crate::resource::localization::Localization;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java TitleDisplay selectable localization keys (Play…Quit).
pub const TITLE_KEYS: [&str; 6] = [
    "minicraft.displays.title.play",
    "minicraft.display.options_display",
    "minicraft.displays.skin",
    "minicraft.displays.achievements",
    "minicraft.displays.title.help",
    "minicraft.displays.title.quit",
];

/// Checking StringEntry + BlankEntry precede the selectable rows.
pub const LEADING_ROWS: usize = 2;

// Java TitleDisplay chooses a new splash when the title screen is created.
// These entries are from its ordinary (non-secret/non-birthday) pool.
const SPLASHES: &[&str] = &[
    "Now with skins!",
    "Now with better fishing!",
    "Now with better tools!",
    "Now with better chests!",
    "Now with better dungeons!",
    "Only on PlayMinicraft.com!",
    "The Wiki is weak! Help it!",
    "Notch is Awesome!",
    "Dillyg10 is cool as Ice!",
    "Shylor is the man!",
    "Chris J is great with portals!",
    "AntVenom loves cows! Honest!",
    "You should read Antidious Venomi!",
    "Keep calm!",
    "Get him, Paul!",
    "Forty-Two!",
    "Kill Creeper, get Gunpowder!",
    "Kill Cow, get Beef!",
    "Kill Zombie, get Cloth!",
    "Kill Slime, get Slime!",
    "Kill Skeleton, get Bones!",
    "Kill Sheep, get Wool!",
    "Kill Pig, get Porkchop!",
    "Gold > Iron",
    "Gem > Gold",
    "Story? Uhh...",
    "Infinite terrain? What's that?",
    "Windows? I prefer Doors!",
    "2.5D FTW!",
    "3rd dimension not included!",
];

pub fn random_splash() -> &'static str {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    SPLASHES[(seed % SPLASHES.len() as u128) as usize]
}

/// Compatibility index is TITLE_KEYS order (0=Play); menu has 2 leading rows.
pub fn menu_selection_for_action(action: usize) -> usize {
    action + LEADING_ROWS
}

/// Map a full menu selection index onto TITLE_KEYS (0=Play).
pub fn action_for_menu_selection(selection: usize) -> usize {
    selection
        .saturating_sub(LEADING_ROWS)
        .min(TITLE_KEYS.len() - 1)
}

/// Java TitleDisplay: checking line (DARK_GRAY / Color.get(1,51)) + blank + selects.
pub fn build_menu(localization: &Localization, _world_count: usize, selection: usize) -> Menu {
    let mut entries = vec![
        Entry::string_colored(
            localization.text("minicraft.displays.title.display.latest_already"),
            crate::gfx::color::DARK_GRAY_RGB,
        ),
        Entry::blank(),
    ];
    for key in TITLE_KEYS {
        entries.push(Entry::select(localization.text(key), false));
    }
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java TitleDisplay splash fade: `count` oscillates 1..=25 over 50 ticks;
/// `bcol = 5 - count/5` then RGB `(bcol*51, bcol*51, bcol*25)`.
pub fn splash_count(ticks: u64) -> u32 {
    let phase = ticks % 50;
    if phase < 25 {
        (phase + 1) as u32
    } else {
        (49 - phase) as u32
    }
}

/// 24-bit splash RGB for the current tick phase.
pub fn splash_color(ticks: u64) -> u32 {
    let count = splash_count(ticks);
    let brightness = 5_u32.saturating_sub(count / 5);
    ((brightness * 51) << 16) | ((brightness * 51) << 8) | (brightness * 25)
}

/// Java TitleDisplay logo anchor: `(w - sheet.width)/2`, `26 - sheet.height/2`.
pub fn logo_origin(sheet_w: i32, sheet_h: i32) -> (i32, i32) {
    ((WIDTH as i32 - sheet_w) / 2, 26 - sheet_h / 2)
}

/// Splash text baseline: `Screen.h/2 - 44`.
pub fn splash_text_y() -> i32 {
    HEIGHT as i32 / 2 - 44
}

/// Footer help lines: h-30, h-20, h-10 (DARK_GRAY).
pub const HELP_LINE_Y: [i32; 3] = [HEIGHT as i32 - 30, HEIGHT as i32 - 20, HEIGHT as i32 - 10];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_menu_matches_java_center_anchor_and_leading_rows() {
        let localization = Localization::load("en-us", &[]);
        let menu = build_menu(&localization, 0, LEADING_ROWS);
        assert_eq!(menu.entries.len(), TITLE_KEYS.len() + LEADING_ROWS);
        assert_eq!(menu.selection, LEADING_ROWS);
        assert_eq!(menu.spacing, 2);
        assert!(!menu.has_frame);
        assert!(menu.selectable);
        assert_eq!(menu.bounds.0 + menu.bounds.2 / 2, WIDTH as i32 / 2);
        // Anchor is (w/2, h*3/5) with RelPos::CENTER.
        let center_y = menu.bounds.1 + menu.bounds.3 / 2;
        assert!((center_y - HEIGHT as i32 * 3 / 5).abs() <= 8);
        // Checking line is non-selectable DARK_GRAY.
        assert!(!menu.entries[0].is_selectable());
        assert_eq!(
            menu.entries[0].color(false),
            crate::gfx::color::DARK_GRAY_RGB
        );
    }

    #[test]
    fn action_mapping_offset_by_leading_rows() {
        assert_eq!(menu_selection_for_action(0), 2);
        assert_eq!(action_for_menu_selection(2), 0);
        assert_eq!(action_for_menu_selection(7), 5);
        assert_eq!(action_for_menu_selection(0), 0);
    }

    #[test]
    fn splash_color_oscillates_like_java_count_division() {
        assert_eq!(splash_count(0), 1);
        assert_eq!(splash_count(24), 25);
        assert_eq!(splash_count(25), 24);
        assert_eq!(splash_count(49), 0);
        // count=1 → bcol=5 → (255,255,125)
        assert_eq!(splash_color(0), (255 << 16) | (255 << 8) | 125);
        // count=25 → bcol=5 → same; count=0 phase edge → bcol=5
        assert_eq!(splash_color(49), (255 << 16) | (255 << 8) | 125);
        // count=5 → bcol=4 → (204,204,100)
        assert_eq!(splash_count(4), 5);
        assert_eq!(splash_color(4), (204 << 16) | (204 << 8) | 100);
    }

    #[test]
    fn logo_origin_matches_java_sheet_centering() {
        assert_eq!(logo_origin(160, 80), ((WIDTH as i32 - 160) / 2, 26 - 40));
    }
}
