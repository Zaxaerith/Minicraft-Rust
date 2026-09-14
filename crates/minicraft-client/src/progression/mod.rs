//! Achievements, advancements, quests, tutorials, books, and util ports.
//!
//! Module layout mirrors Java `minicraft/util/*` 1:1:
//! - `simplex` ↔ `Simplex.java`
//! - `advancement` ↔ `AdvancementElement.java`
//! - `book_data` ↔ `BookData.java`
//! - `quest` ↔ `Quest.java`
//! - `tutorial` ↔ `TutorialElement.java`
//! - `achievement` ↔ `Achievement.java`
//! - `logging` ↔ `Logging.java`
//! - `my_utils` ↔ `MyUtils.java`
//! - `vector2` ↔ `Vector2.java`
//! - `progress` hosts the client-side `ProgressState` orchestrator.

pub mod achievement;
pub mod advancement;
pub mod book_data;
pub mod logging;
pub mod my_utils;
pub mod progress;
pub mod quest;
pub mod simplex;
pub mod tutorial;
pub mod vector2;

pub use achievement::{Achievement, achievements};
pub use advancement::{Criterion, ElementRewards, ItemRequirement, ProgressEvent, Rangeable};
pub use book_data::{Book, load_book};
pub use my_utils::{capitalize, capitalize_fully, clamp, paginate, plural};
pub use progress::{ProgressState, ProgressUpdate};
pub use quest::{Quest, QuestGroup, QuestGroupView, QuestView, quests};
pub use simplex::Simplex;
pub use tutorial::{TutorialStep, tutorials};
pub use vector2::Vector2;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Book, ProgressEvent, ProgressState, achievements, paginate};

    #[test]
    fn bundled_content_is_valid_and_paginated() {
        assert_eq!(achievements().unwrap().len(), 17);
        assert!(Book::GameGuide.pages().len() > 1);
        let pages = paginate("one two three four", 7, 2);
        assert_eq!(pages, vec![vec!["one two", "three"], vec!["four"]]);
    }

    #[test]
    fn bundled_phase_six_progression_has_java_224_counts() {
        let progress = ProgressState::load().unwrap();
        assert_eq!(progress.tutorial_count(), 5);
        assert_eq!(progress.quest_group_count(), 4);
        assert_eq!(progress.quest_count(), 14);
    }

    #[test]
    fn farming_progress_records_inventory_use_and_placement_events() {
        let mut progress = ProgressState::load().unwrap();
        let mut inventory = HashMap::from([
            ("wood".to_owned(), 10),
            ("wheat seeds".to_owned(), 5),
            ("wood hoe".to_owned(), 1),
        ]);
        let groups = progress.quest_group_views(&inventory, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "minicraft.quest.farming");
        assert_eq!(progress.quest_views(groups[0].index, &inventory).len(), 1);
        assert!(progress.quest_group_views(&inventory, true).is_empty());
        let update = progress.update(&ProgressEvent::InventoryChanged, &inventory);
        assert!(
            update
                .completed_quests
                .contains(&"minicraft.quest.farming.crafting_hoe".to_owned())
        );
        assert_eq!(progress.quest_views(groups[0].index, &inventory).len(), 2);
        let groups_after = progress.quest_group_views(&inventory, false);
        assert_eq!(groups_after[0].completed_quests, 1);
        assert_eq!(groups_after[0].ongoing_quests, 1);
        let views = progress.quest_views(groups[0].index, &inventory);
        assert_eq!(
            views[1].parent.as_deref(),
            Some("minicraft.quest.farming.crafting_hoe")
        );
        assert!(!views[0].has_requirements);

        let update = progress.update(
            &ProgressEvent::ItemUsedOnTile {
                item: "wood hoe".to_owned(),
                tile: "dirt".to_owned(),
            },
            &inventory,
        );
        assert!(
            update
                .completed_quests
                .contains(&"minicraft.quest.farming.making_farmland".to_owned())
        );

        inventory.insert("wheat seeds".to_owned(), 4);
        let update = progress.update(
            &ProgressEvent::PlacedTile {
                tile: "wheat".to_owned(),
            },
            &inventory,
        );
        assert!(
            update
                .completed_quests
                .contains(&"minicraft.quest.farming.planting_wheat".to_owned())
        );
        inventory.insert("wheat".to_owned(), 1);
        let update = progress.update(&ProgressEvent::InventoryChanged, &inventory);
        assert!(update.rewards.contains(&"wheat seeds_8".to_owned()));
    }

    #[test]
    fn tutorial_and_quest_triggers_align_with_java_events() {
        let mut progress = ProgressState::load().unwrap();
        // First tutorial needs 1 wood.
        assert!(progress
            .update(&ProgressEvent::InventoryChanged, &HashMap::new())
            .completed_tutorials
            .is_empty());
        let inventory = HashMap::from([("wood".to_owned(), 1)]);
        let update = progress.update(&ProgressEvent::InventoryChanged, &inventory);
        assert!(
            update
                .completed_tutorials
                .contains(&"minicraft.tutorial.start_getting_wood".to_owned())
        );
        // Second tutorial needs 10 wood.
        let inventory = HashMap::from([("wood".to_owned(), 10)]);
        let update = progress.update(&ProgressEvent::InventoryChanged, &inventory);
        assert!(
            update
                .completed_tutorials
                .contains(&"minicraft.tutorial.getting_wood".to_owned())
        );
    }
}
