//! Java `AchievementsDisplay.java` — achievement list and score panel.
use std::collections::HashSet;

use crate::gfx::{HEIGHT, WIDTH};
use crate::progression::Achievement;
use crate::resource::localization::Localization;

use super::entry::{COL_GREEN, COL_UNSLCT, Entry};
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java: list menu `Builder(false, 2, CENTER, …).setSize(48, 48)`.
pub fn build_list_menu(
    localization: &Localization,
    achievements: &[Achievement],
    unlocked: &HashSet<String>,
    selection: usize,
) -> Menu {
    // Java SelectEntry(id) with unlocked → GREEN else WHITE (COL_SLCT);
    // locked still uses selectable white/gray path via select_colored.
    let entries: Vec<Entry> = achievements
        .iter()
        .map(|achievement| {
            let color = if unlocked.contains(&achievement.id) {
                COL_GREEN
            } else {
                COL_UNSLCT
            };
            Entry::select_colored(localization.text(&achievement.id), color)
        })
        .collect();
    MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_size(48, 48)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java description panel: `Builder(false, 2, BOTTOM).setSize(200, 32)
/// .setPositioning(w/2, h/2+32, BOTTOM)`.
pub fn build_description_menu(description_lines: Vec<String>) -> Menu {
    let entries: Vec<Entry> = description_lines.into_iter().map(Entry::string).collect();
    MenuBuilder::new(false, 2, RelPos::BOTTOM, entries)
        .set_size(200, 32)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 / 2 + 32), RelPos::BOTTOM)
        .create_menu()
}

/// Title / score / status / help baselines from Java render().
pub const TITLE_Y: i32 = 8;
pub const SCORE_Y: i32 = 32;
pub const STATUS_Y: i32 = 48;

pub fn help_y() -> i32 {
    HEIGHT as i32 - 8
}

/// Java unlock color for a list row: GREEN when unlocked, otherwise WHITE
/// when selected / GRAY when not (handled by Entry::select_colored).
pub fn achievement_entry_color(unlocked: bool) -> u32 {
    if unlocked { COL_GREEN } else { COL_UNSLCT }
}

/// Java setAchievement: creative mode blocks unlocks unless allowCreative.
pub fn can_unlock(allow_creative: bool, creative_mode: bool) -> bool {
    allow_creative || !creative_mode
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achievements_list_is_48x48_centered_with_green_unlocked() {
        let localization = Localization::load("en-us", &[]);
        let achievements = crate::progression::achievements().unwrap_or_default();
        let mut unlocked = HashSet::new();
        if let Some(first) = achievements.first() {
            unlocked.insert(first.id.clone());
        }
        let menu = build_list_menu(&localization, &achievements, &unlocked, 0);
        assert_eq!(menu.bounds.2, 48);
        assert_eq!(menu.bounds.3, 48);
        assert!(!menu.entries.is_empty() || achievements.is_empty());
        if let Some(first) = achievements.first() {
            let expected = if unlocked.contains(&first.id) {
                COL_GREEN
            } else {
                COL_UNSLCT
            };
            assert_eq!(menu.entries[0].color(true), expected);
        }
    }

    #[test]
    fn description_panel_is_200x32_below_score() {
        let menu = build_description_menu(vec!["line one".into(), "line two".into()]);
        assert_eq!(menu.bounds.2, 200);
        assert_eq!(menu.bounds.3, 32);
        // Java RelPos.BOTTOM hangs the menu below the anchor (top-left on anchor).
        assert_eq!(menu.bounds.1, HEIGHT as i32 / 2 + 32);
    }

    #[test]
    fn creative_mode_blocks_unlock_unless_allowed() {
        assert!(!can_unlock(false, true));
        assert!(can_unlock(true, true));
        assert!(can_unlock(false, false));
    }
}
