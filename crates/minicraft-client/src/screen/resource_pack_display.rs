//! Java `ResourcePackDisplay.java` — pack list, reorder, toggle, and info.
use crate::resource::pack::ResourcePack;

use super::entry::Entry;
use super::menu::{Menu, MenuBuilder};
use super::rel_pos::RelPos;

/// Java ResourcePackDisplay pack list: indicators at (20,60) BOTTOM_RIGHT,
/// packs at (20,60) BOTTOM_LEFT.
pub const ANCHOR: (i32, i32) = (20, 60);

/// Java default pack cannot be toggled or reordered.
pub fn is_default(pack: &ResourcePack) -> bool {
    pack.id.eq_ignore_ascii_case("default") || pack.name.eq_ignore_ascii_case("Default")
}

pub fn build_menu(packs: &[ResourcePack], _enabled: &[String], selection: usize) -> Menu {
    let entries: Vec<Entry> = packs
        .iter()
        .map(|pack| Entry::select(pack.name.clone(), false))
        .collect();
    MenuBuilder::new(false, 2, RelPos::LEFT, entries)
        .set_positioning(ANCHOR, RelPos::BOTTOM_LEFT)
        .set_selection(selection as i32)
        .create_menu()
}

/// Java toggle indicator column (✓ / empty) at BOTTOM_RIGHT.
pub fn build_indicator_menu(packs: &[ResourcePack], enabled: &[String], selection: usize) -> Menu {
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
        .set_positioning(ANCHOR, RelPos::BOTTOM_RIGHT)
        .set_selection(selection as i32)
        .create_menu()
}

/// Reorder request from cursor-left/right. Does not wrap past default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reorder {
    Moved(usize),
    Blocked,
}

/// Java: can move onto any pack except the default (cannot reorder default).
pub fn try_reorder(packs: &mut [ResourcePack], selection: usize, right: bool) -> Reorder {
    if right {
        let next = selection + 1;
        if next < packs.len() && !is_default(&packs[next]) {
            packs.swap(selection, next);
            return Reorder::Moved(next);
        }
        return Reorder::Blocked;
    }
    if selection > 0 {
        packs.swap(selection, selection - 1);
        return Reorder::Moved(selection - 1);
    }
    Reorder::Blocked
}

/// Enabled ids ordered by pack menu order (enabled packs keep relative priority).
pub fn order_packs_for_menu(packs: &mut [ResourcePack], enabled: &[String]) {
    packs.sort_by_key(|pack| {
        enabled
            .iter()
            .rev()
            .position(|id| id == &pack.id)
            .map_or((1, usize::MAX), |priority| (0, priority))
    });
}

/// Rewrite enabled list to match the reordered pack list (enabled only).
pub fn sync_enabled_pack_order(enabled: &mut Vec<String>, packs: &[ResourcePack]) {
    let was_enabled = enabled
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    enabled.clear();
    enabled.extend(
        packs
            .iter()
            .rev()
            .filter(|pack| was_enabled.contains(&pack.id))
            .map(|pack| pack.id.clone()),
    );
}

/// Drop enabled ids that no longer resolve. Returns true when the list changed.
pub fn prune_missing_enabled_packs(enabled: &mut Vec<String>, packs: &[ResourcePack]) -> bool {
    let before = enabled.len();
    enabled.retain(|id| packs.iter().any(|pack| &pack.id == id));
    enabled.len() != before
}

/// Java pack list spacing is 2, LEFT-aligned names.
pub const SPACING: i32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_menu_uses_bottom_left_anchor_and_spacing_two() {
        let packs = vec![
            ResourcePack::parity_fixture("a", "Alpha", None),
            ResourcePack::parity_fixture("b", "Beta", None),
        ];
        let menu = build_menu(&packs, &["a".to_owned()], 0);
        assert_eq!(menu.spacing, SPACING);
        assert_eq!(menu.entry_pos, RelPos::LEFT);
        assert_eq!(menu.selection, 0);
    }

    #[test]
    fn cannot_move_onto_default_but_default_itself_may_shift() {
        let mut packs = vec![
            ResourcePack::parity_fixture("default", "Default", None),
            ResourcePack::parity_fixture("a", "Alpha", None),
            ResourcePack::parity_fixture("b", "Beta", None),
        ];
        // Left from index 0 is blocked (would wrap).
        assert_eq!(try_reorder(&mut packs, 0, false), Reorder::Blocked);
        // Right from default onto non-default is allowed (Java only blocks the target being default).
        assert_eq!(try_reorder(&mut packs, 0, true), Reorder::Moved(1));
        assert_eq!(packs[0].id, "a");
        assert_eq!(packs[1].id, "default");
        // Now default is at 1; moving right onto "b" is allowed.
        assert_eq!(try_reorder(&mut packs, 1, true), Reorder::Moved(2));
        assert_eq!(packs[2].id, "default");
    }

    #[test]
    fn cannot_reorder_a_pack_onto_default_target() {
        let mut packs = vec![
            ResourcePack::parity_fixture("a", "Alpha", None),
            ResourcePack::parity_fixture("default", "Default", None),
        ];
        // Right from "a" would land on default → blocked.
        assert_eq!(try_reorder(&mut packs, 0, true), Reorder::Blocked);
        assert_eq!(packs[1].id, "default");
    }

    #[test]
    fn reorder_moves_selected_pack() {
        let mut packs: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .map(|id| ResourcePack::parity_fixture(id, id, None))
            .collect();
        assert_eq!(try_reorder(&mut packs, 0, true), Reorder::Moved(1));
        assert_eq!(packs[1].id, "a");
        assert_eq!(try_reorder(&mut packs, 1, false), Reorder::Moved(0));
        assert_eq!(packs[0].id, "a");
    }

    #[test]
    fn order_and_sync_match_java_priority() {
        let mut packs: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .map(|id| ResourcePack::parity_fixture(id, id, None))
            .collect();
        let mut enabled = vec!["a".to_owned(), "c".to_owned()];
        order_packs_for_menu(&mut packs, &enabled);
        assert_eq!(
            packs.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["c", "a", "b"]
        );
        try_reorder(&mut packs, 2, false);
        try_reorder(&mut packs, 1, false);
        sync_enabled_pack_order(&mut enabled, &packs);
        assert_eq!(enabled, ["a", "c"]);
    }

    #[test]
    fn prune_drops_missing_ids() {
        let packs = vec![ResourcePack::parity_fixture("keep", "Keep", None)];
        let mut enabled = vec!["keep".to_owned(), "ghost".to_owned()];
        assert!(prune_missing_enabled_packs(&mut enabled, &packs));
        assert_eq!(enabled, vec!["keep".to_owned()]);
        assert!(!prune_missing_enabled_packs(&mut enabled, &packs));
    }
}
