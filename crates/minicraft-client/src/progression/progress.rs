//! Host progression orchestrator — tutorials, quest series, achievements.
//!
//! Combines Java `AdvancementElement` + `Quest` + `TutorialElement` update
//! loops into the single `ProgressState` used by the Rust client world.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::advancement::{Criterion, ProgressEvent, record_owner_hits};
use super::quest::{Quest, QuestGroup, QuestGroupView, QuestView, quests};
use super::tutorial::{TutorialStep, tutorials};

/// Aggregate completion result of one `update` call.
#[derive(Debug, Default, Clone)]
pub struct ProgressUpdate {
    pub completed_tutorials: Vec<String>,
    pub completed_quests: Vec<String>,
    pub rewards: Vec<String>,
}

/// Serializable progression snapshot used by world save/import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressState {
    tutorials: Vec<TutorialStep>,
    quest_groups: Vec<QuestGroup>,
    tutorial_completed: HashSet<String>,
    quest_completed: HashSet<String>,
    group_unlocked: HashSet<String>,
    quest_unlocked: HashSet<String>,
    criterion_hits: HashSet<String>,
    achievements: HashSet<String>,
}

impl ProgressState {
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            tutorials: tutorials()?,
            quest_groups: quests()?,
            tutorial_completed: HashSet::new(),
            quest_completed: HashSet::new(),
            group_unlocked: HashSet::new(),
            quest_unlocked: HashSet::new(),
            criterion_hits: HashSet::new(),
            achievements: HashSet::new(),
        })
    }

    pub fn update(
        &mut self,
        event: &ProgressEvent,
        inventory: &HashMap<String, u16>,
    ) -> ProgressUpdate {
        self.update_filtered(event, inventory, true, true)
    }

    pub fn update_filtered(
        &mut self,
        event: &ProgressEvent,
        inventory: &HashMap<String, u16>,
        tutorials_enabled: bool,
        quests_enabled: bool,
    ) -> ProgressUpdate {
        self.record_event_hits(event, inventory, tutorials_enabled, quests_enabled);
        let mut update = ProgressUpdate::default();
        if tutorials_enabled {
            for tutorial in &self.tutorials {
                if self.tutorial_completed.contains(&tutorial.id) {
                    continue;
                }
                if self.owner_criteria_met(
                    &tutorial.id,
                    &tutorial.criteria,
                    &tutorial.requirements,
                    event,
                    inventory,
                ) {
                    self.tutorial_completed.insert(tutorial.id.clone());
                    update.completed_tutorials.push(tutorial.id.clone());
                }
            }
        }

        if quests_enabled {
            loop {
                let mut changed = false;
                for group in &self.quest_groups {
                    if self.criteria_list_met(&group.id, &group.unlocking, event, inventory) {
                        self.group_unlocked.insert(group.id.clone());
                    }
                    if !self.group_unlocked.contains(&group.id) {
                        continue;
                    }
                    for quest in &group.quests {
                        if self.quest_completed.contains(&quest.id)
                            || quest
                                .parent
                                .as_ref()
                                .is_some_and(|parent| !self.quest_completed.contains(parent))
                        {
                            continue;
                        }
                        if self.criteria_list_met(&quest.id, &quest.unlocking, event, inventory) {
                            self.quest_unlocked.insert(quest.id.clone());
                        }
                        if !self.quest_unlocked.contains(&quest.id) {
                            continue;
                        }
                        if self.owner_criteria_met(
                            &quest.id,
                            &quest.criteria,
                            &quest.requirements,
                            event,
                            inventory,
                        ) {
                            self.quest_completed.insert(quest.id.clone());
                            update.completed_quests.push(quest.id.clone());
                            update.rewards.extend(quest.rewards.iter().cloned());
                            changed = true;
                        }
                    }
                }
                if !changed {
                    break;
                }
            }
        }
        update
    }

    pub fn unlock_achievement(&mut self, id: &str) -> bool {
        self.achievements.insert(id.to_owned())
    }

    pub fn achievement_unlocked(&self, id: &str) -> bool {
        self.achievements.contains(id)
    }

    pub fn achievement_count(&self) -> usize {
        self.achievements.len()
    }

    pub fn tutorial_completed_count(&self) -> usize {
        self.tutorial_completed.len()
    }

    pub fn quest_completed_count(&self) -> usize {
        self.quest_completed.len()
    }

    pub fn current_tutorial(&self) -> Option<&TutorialStep> {
        self.tutorials
            .iter()
            .find(|step| !self.tutorial_completed.contains(&step.id))
    }

    pub fn current_quest(&self, inventory: &HashMap<String, u16>) -> Option<&Quest> {
        let event = ProgressEvent::InventoryChanged;
        self.quest_groups.iter().find_map(|group| {
            (self.group_unlocked.contains(&group.id)
                || self.criteria_list_met(&group.id, &group.unlocking, &event, inventory))
            .then(|| {
                group.quests.iter().find(|quest| {
                    !self.quest_completed.contains(&quest.id)
                        && quest
                            .parent
                            .as_ref()
                            .is_none_or(|parent| self.quest_completed.contains(parent))
                        && (self.quest_unlocked.contains(&quest.id)
                            || self.criteria_list_met(
                                &quest.id,
                                &quest.unlocking,
                                &event,
                                inventory,
                            ))
                })
            })
            .flatten()
        })
    }

    pub fn tutorial_count(&self) -> usize {
        self.tutorials.len()
    }

    pub fn quest_group_count(&self) -> usize {
        self.quest_groups.len()
    }

    pub fn quest_count(&self) -> usize {
        self.quest_groups
            .iter()
            .map(|group| group.quests.len())
            .sum()
    }

    pub fn quest_group_views(
        &self,
        inventory: &HashMap<String, u16>,
        completed_tab: bool,
    ) -> Vec<QuestGroupView> {
        self.quest_groups
            .iter()
            .enumerate()
            .filter_map(|(index, group)| {
                let unlocked = self.group_is_unlocked(group, inventory);
                let completed_quests = group
                    .quests
                    .iter()
                    .filter(|quest| self.quest_completed.contains(&quest.id))
                    .count();
                let completed = !group.quests.is_empty() && completed_quests == group.quests.len();
                (unlocked && (!completed_tab || completed)).then(|| QuestGroupView {
                    index,
                    id: group.id.clone(),
                    description: group.description.clone(),
                    completed,
                    completed_quests,
                    ongoing_quests: group
                        .quests
                        .iter()
                        .filter(|quest| {
                            self.quest_is_unlocked(group, quest, inventory)
                                && !self.quest_completed.contains(&quest.id)
                        })
                        .count(),
                    total_quests: group.quests.len(),
                })
            })
            .collect()
    }

    pub fn quest_views(
        &self,
        group_index: usize,
        inventory: &HashMap<String, u16>,
    ) -> Vec<QuestView> {
        let Some(group) = self.quest_groups.get(group_index) else {
            return Vec::new();
        };
        group
            .quests
            .iter()
            .enumerate()
            .filter(|(_, quest)| self.quest_is_unlocked(group, quest, inventory))
            .map(|(index, quest)| QuestView {
                index,
                id: quest.id.clone(),
                description: quest.description.clone(),
                parent: quest.parent.clone(),
                has_requirements: quest.has_requirements,
                completed: self.quest_completed.contains(&quest.id),
                criteria_completed: quest
                    .criteria
                    .iter()
                    .filter(|criterion| {
                        criterion.matches(&ProgressEvent::InventoryChanged, inventory)
                            || self
                                .criterion_hits
                                .contains(&format!("{}:{}", quest.id, criterion.name))
                    })
                    .count(),
                criteria_total: quest.criteria.len(),
            })
            .collect()
    }

    fn group_is_unlocked(&self, group: &QuestGroup, inventory: &HashMap<String, u16>) -> bool {
        self.group_unlocked.contains(&group.id)
            || self.criteria_list_met(
                &group.id,
                &group.unlocking,
                &ProgressEvent::InventoryChanged,
                inventory,
            )
    }

    fn quest_is_unlocked(
        &self,
        group: &QuestGroup,
        quest: &Quest,
        inventory: &HashMap<String, u16>,
    ) -> bool {
        self.group_is_unlocked(group, inventory)
            && quest
                .parent
                .as_ref()
                .is_none_or(|parent| self.quest_completed.contains(parent))
            && (self.quest_unlocked.contains(&quest.id)
                || self.criteria_list_met(
                    &quest.id,
                    &quest.unlocking,
                    &ProgressEvent::InventoryChanged,
                    inventory,
                ))
    }

    fn record_event_hits(
        &mut self,
        event: &ProgressEvent,
        inventory: &HashMap<String, u16>,
        tutorials_enabled: bool,
        quests_enabled: bool,
    ) {
        if tutorials_enabled {
            for tutorial in &self.tutorials {
                record_owner_hits(
                    &mut self.criterion_hits,
                    &tutorial.id,
                    &tutorial.criteria,
                    event,
                    inventory,
                );
            }
        }
        if quests_enabled {
            for group in &self.quest_groups {
                for quest in &group.quests {
                    record_owner_hits(
                        &mut self.criterion_hits,
                        &quest.id,
                        &quest.criteria,
                        event,
                        inventory,
                    );
                }
            }
        }
    }

    fn criterion_met(
        &self,
        owner: &str,
        criterion: &Criterion,
        event: &ProgressEvent,
        inventory: &HashMap<String, u16>,
    ) -> bool {
        criterion.matches(event, inventory)
            || self
                .criterion_hits
                .contains(&format!("{owner}:{}", criterion.name))
    }

    /// Java `AdvancementElement.checkIsCompleted` with optional requirement sets.
    fn owner_criteria_met(
        &self,
        owner: &str,
        criteria: &[Criterion],
        requirements: &[Vec<String>],
        event: &ProgressEvent,
        inventory: &HashMap<String, u16>,
    ) -> bool {
        if requirements.is_empty() {
            criteria
                .iter()
                .all(|criterion| self.criterion_met(owner, criterion, event, inventory))
        } else {
            requirements.iter().all(|sublist| {
                sublist.iter().any(|name| {
                    criteria.iter().any(|criterion| {
                        &criterion.name == name
                            && self.criterion_met(owner, criterion, event, inventory)
                    })
                })
            })
        }
    }

    /// Java unlocking check: empty list → all unlocking criteria; else AND/OR sets.
    fn criteria_list_met(
        &self,
        owner: &str,
        criteria: &[Criterion],
        event: &ProgressEvent,
        inventory: &HashMap<String, u16>,
    ) -> bool {
        let _ = owner;
        criteria
            .iter()
            .all(|criterion| criterion.matches(event, inventory))
    }
}
