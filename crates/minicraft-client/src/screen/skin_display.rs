//! Java `SkinDisplay.java` — skin list and preview animation.
use crate::gfx::{HEIGHT, WIDTH};
#[cfg(test)]
use crate::resource::localization::Localization;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java SkinDisplay: `Builder(false, 2, CENTER).setDisplayLength(8)
/// .setSelectable(true).setPositioning(w/2, h*3/5, CENTER)`.
pub const DISPLAY_LENGTH: i32 = 8;

/// Java default skin keys, in order.
pub const DEFAULT_SKINS: [&str; 4] = [
    "minicraft.skin.paul",
    "minicraft.skin.paul_cape",
    "minicraft.skin.minecraft_steve",
    "minicraft.skin.minecraft_alex",
];

/// Preview animation frame period (Java `step / 40`).
pub const PREVIEW_FRAME_TICKS: u64 = 40;

pub fn build_menu(skins: &[String], selection: usize) -> Menu {
    let entries: Vec<Entry> = skins
        .iter()
        .map(|skin| Entry::select(skin.clone(), false))
        .collect();
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_display_length(DISPLAY_LENGTH)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java compileMobSpriteAnimations + SkinDisplay:
/// spriteIndex = step/40 % 8 → [dir][frame].
/// set1 = sheetX 0,2,4,6 (down1, up1, right1, right2); set2 = same with mirror=1.
/// sprites[0]=down set2/set1, [1]=up set2/set1, [2]=left set2, [3]=right set1.
pub fn skin_preview_frame(ticks: u64) -> (usize, bool) {
    let sprite_index = (ticks / PREVIEW_FRAME_TICKS % 8) as usize;
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

/// Title y=16; preview at (w/2-8, 40); help at h-17 and h-9.
pub const TITLE_Y: i32 = 16;
pub const PREVIEW_Y: i32 = 40;

pub fn preview_x() -> i32 {
    WIDTH as i32 / 2 - 8
}

pub fn help_move_y() -> i32 {
    HEIGHT as i32 - 17
}

pub fn help_select_y() -> i32 {
    HEIGHT as i32 - 9
}

/// Java confirmExit: selected skin at menu selection index.
pub fn selected_skin(skins: &[String], selection: usize) -> Option<&str> {
    skins.get(selection).map(String::as_str)
}

/// Fashion Show achievement id (Java AchievementsDisplay.setAchievement).
pub const FASHION_SHOW_ACHIEVEMENT: &str = "minicraft.achievement.skin";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skins_menu_matches_java_display_length_and_anchor() {
        let localization = Localization::load("en-us", &[]);
        let skins: Vec<String> = DEFAULT_SKINS
            .iter()
            .map(|key| localization.text(key).to_owned())
            .collect();
        let menu = build_menu(&skins, 0);
        assert_eq!(menu.display_length, DISPLAY_LENGTH);
        assert_eq!(menu.spacing, 2);
        assert_eq!(menu.entries.len(), DEFAULT_SKINS.len());
        assert_eq!(menu.bounds.0 + menu.bounds.2 / 2, WIDTH as i32 / 2);
        let center_y = menu.bounds.1 + menu.bounds.3 / 2;
        assert!((center_y - HEIGHT as i32 * 3 / 5).abs() <= 8);
    }

    #[test]
    fn skin_preview_cycles_java_direction_and_walk_frames() {
        assert_eq!(skin_preview_frame(0), (0, false));
        assert_eq!(skin_preview_frame(40), (0, true));
        assert_eq!(skin_preview_frame(80), (16, false));
        assert_eq!(skin_preview_frame(120), (16, true));
        assert_eq!(skin_preview_frame(160), (32, true));
        assert_eq!(skin_preview_frame(200), (48, true));
        assert_eq!(skin_preview_frame(240), (32, false));
        assert_eq!(skin_preview_frame(280), (48, false));
        assert_eq!(skin_preview_frame(320), (0, false));
    }

    #[test]
    fn selected_skin_follows_menu_index() {
        let skins = vec!["a".into(), "b".into()];
        assert_eq!(selected_skin(&skins, 1), Some("b"));
        assert_eq!(selected_skin(&skins, 2), None);
    }
}
