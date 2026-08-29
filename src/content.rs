use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Book {
    Instructions,
    GameGuide,
    About,
    Credits,
    Antidious,
}

impl Book {
    pub const ALL: [Self; 4] = [
        Self::Instructions,
        Self::GameGuide,
        Self::About,
        Self::Credits,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Instructions => "INSTRUCTIONS",
            Self::GameGuide => "GAME GUIDE",
            Self::About => "ABOUT",
            Self::Credits => "CREDITS",
            Self::Antidious => "ANTIDIOUS VENOMI",
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::Instructions => include_str!("../assets/assets/books/instructions.txt"),
            Self::GameGuide => include_str!("../assets/assets/books/game_guide.txt"),
            Self::About => include_str!("../assets/assets/books/about.txt"),
            Self::Credits => include_str!("../assets/assets/books/credits.txt"),
            Self::Antidious => include_str!("../assets/assets/books/antidous.txt"),
        }
    }

    pub fn pages(self) -> Vec<Vec<String>> {
        paginate(self.text(), 32, 14)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent {
    InventoryChanged,
    ItemUsedOnTile { item: String, tile: String },
    PlacedTile { tile: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ItemRequirement {
    alternatives: Vec<String>,
    minimum: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Criterion {
    name: String,
    trigger: String,
    items: Vec<ItemRequirement>,
    used_item: Vec<String>,
    location_tiles: Vec<String>,
    placed_tile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TutorialStep {
    pub id: String,
    pub description: String,
    pub index: usize,
    criteria: Vec<Criterion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Quest {
    pub id: String,
    pub description: String,
    pub parent: Option<String>,
    pub rewards: Vec<String>,
    criteria: Vec<Criterion>,
    unlocking: Vec<Criterion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct QuestGroup {
    pub id: String,
    pub description: String,
    pub quests: Vec<Quest>,
    unlocking: Vec<Criterion>,
}

#[derive(Debug, Default, Clone)]
pub struct ProgressUpdate {
    pub completed_tutorials: Vec<String>,
    pub completed_quests: Vec<String>,
    pub rewards: Vec<String>,
}

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

#[allow(dead_code)]
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
                if tutorial
                    .criteria
                    .iter()
                    .all(|criterion| self.criterion_met(&tutorial.id, criterion, event, inventory))
                {
                    self.tutorial_completed.insert(tutorial.id.clone());
                    update.completed_tutorials.push(tutorial.id.clone());
                }
            }
        }

        if quests_enabled {
            loop {
                let mut changed = false;
                for group in &self.quest_groups {
                    if group
                        .unlocking
                        .iter()
                        .all(|criterion| criterion.matches(event, inventory))
                    {
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
                        if quest
                            .unlocking
                            .iter()
                            .all(|criterion| criterion.matches(event, inventory))
                        {
                            self.quest_unlocked.insert(quest.id.clone());
                        }
                        if !self.quest_unlocked.contains(&quest.id) {
                            continue;
                        }
                        if quest.criteria.iter().all(|criterion| {
                            self.criterion_met(&quest.id, criterion, event, inventory)
                        }) {
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
                || group
                    .unlocking
                    .iter()
                    .all(|criterion| criterion.matches(&event, inventory)))
            .then(|| {
                group.quests.iter().find(|quest| {
                    !self.quest_completed.contains(&quest.id)
                        && quest
                            .parent
                            .as_ref()
                            .is_none_or(|parent| self.quest_completed.contains(parent))
                        && (self.quest_unlocked.contains(&quest.id)
                            || quest
                                .unlocking
                                .iter()
                                .all(|criterion| criterion.matches(&event, inventory)))
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
}

impl Criterion {
    fn matches(&self, event: &ProgressEvent, inventory: &HashMap<String, u16>) -> bool {
        match self.trigger.as_str() {
            "inventory_changed" => self.items.iter().all(|requirement| {
                requirement
                    .alternatives
                    .iter()
                    .any(|item| inventory.get(item).copied().unwrap_or(0) >= requirement.minimum)
            }),
            "item_used_on_tile" => {
                let ProgressEvent::ItemUsedOnTile { item, tile } = event else {
                    return false;
                };
                (self.used_item.is_empty() || self.used_item.contains(item))
                    && (self.location_tiles.is_empty() || self.location_tiles.contains(tile))
            }
            "placed_tile" => {
                let ProgressEvent::PlacedTile { tile } = event else {
                    return false;
                };
                self.placed_tile
                    .as_ref()
                    .is_none_or(|placed| placed == tile)
            }
            _ => false,
        }
    }
}

fn record_owner_hits(
    hits: &mut HashSet<String>,
    owner: &str,
    criteria: &[Criterion],
    event: &ProgressEvent,
    inventory: &HashMap<String, u16>,
) {
    for criterion in criteria {
        if criterion.matches(event, inventory) {
            hits.insert(format!("{owner}:{}", criterion.name));
        }
    }
}

pub fn tutorials() -> Result<Vec<TutorialStep>, String> {
    let value: Value = serde_json::from_str(include_str!("../assets/resources/tutorials.json"))
        .map_err(|error| format!("cannot parse bundled tutorials: {error}"))?;
    let mut steps = object(&value, "tutorial root")?
        .iter()
        .map(|(id, entry)| {
            Ok(TutorialStep {
                id: id.clone(),
                description: required_string(entry, "description")?,
                index: entry["index"].as_u64().unwrap_or(0) as usize,
                criteria: parse_criteria(&entry["criteria"])?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    steps.sort_by_key(|step| step.index);
    Ok(steps)
}

pub fn quests() -> Result<Vec<QuestGroup>, String> {
    let value: Value = serde_json::from_str(include_str!("../assets/resources/quests.json"))
        .map_err(|error| format!("cannot parse bundled quests: {error}"))?;
    object(&value, "quest root")?
        .iter()
        .map(|(id, group)| {
            let quests = object(&group["quests"], "quest group")?
                .iter()
                .map(|(quest_id, quest)| {
                    Ok(Quest {
                        id: quest_id.clone(),
                        description: required_string(quest, "description")?,
                        parent: quest["parent"].as_str().map(str::to_owned),
                        rewards: quest["rewards"]["items"]
                            .as_array()
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| item.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        criteria: parse_criteria(&quest["criteria"])?,
                        unlocking: parse_criteria(&quest["unlocking_criteria"])?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(QuestGroup {
                id: id.clone(),
                description: required_string(group, "description")?,
                quests,
                unlocking: parse_criteria(&group["unlocking_criteria"])?,
            })
        })
        .collect()
}

fn parse_criteria(value: &Value) -> Result<Vec<Criterion>, String> {
    let Some(criteria) = value.as_object() else {
        return Ok(Vec::new());
    };
    criteria
        .iter()
        .map(|(name, criterion)| {
            let conditions = &criterion["conditions"];
            let items = conditions["items"]
                .as_array()
                .map(|requirements| {
                    requirements
                        .iter()
                        .map(|requirement| {
                            let alternatives = requirement["items"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|item| item.as_str().map(normalize))
                                .collect();
                            let minimum = requirement["count"]["min"]
                                .as_u64()
                                .or_else(|| {
                                    requirement.as_object().and_then(|fields| {
                                        fields
                                            .iter()
                                            .find(|(key, _)| key.as_str() != "items")
                                            .and_then(|(_, range)| range["min"].as_u64())
                                    })
                                })
                                .unwrap_or(1) as u16;
                            ItemRequirement {
                                alternatives,
                                minimum,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            let used_item = conditions["item"]["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item.as_str().map(normalize))
                .collect();
            let location_tiles = conditions["location"]["tile"]["tiles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|tile| tile.as_str().map(normalize))
                .collect();
            Ok(Criterion {
                name: name.clone(),
                trigger: required_string(criterion, "trigger")?,
                items,
                used_item,
                location_tiles,
                placed_tile: conditions["tile"].as_str().map(normalize),
            })
        })
        .collect()
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("bundled {label} must be an object"))
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("bundled content is missing {key}"))
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[derive(Debug, Clone)]
pub struct Achievement {
    pub id: String,
    pub description: String,
    pub score: u32,
}

pub fn achievements() -> Result<Vec<Achievement>, String> {
    let value: Value = serde_json::from_str(include_str!("../assets/resources/achievements.json"))
        .map_err(|error| format!("cannot parse bundled achievements: {error}"))?;
    value
        .as_array()
        .ok_or_else(|| "bundled achievements must be an array".to_owned())?
        .iter()
        .map(|entry| {
            Ok(Achievement {
                id: entry["id"]
                    .as_str()
                    .ok_or_else(|| "achievement id is missing".to_owned())?
                    .to_owned(),
                description: entry["desc"]
                    .as_str()
                    .ok_or_else(|| "achievement description is missing".to_owned())?
                    .to_owned(),
                score: entry["score"].as_u64().unwrap_or(0) as u32,
            })
        })
        .collect()
}

pub fn paginate(text: &str, columns: usize, rows: usize) -> Vec<Vec<String>> {
    let mut lines = Vec::new();
    for paragraph in text.replace('\r', "").lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if word.chars().count() > columns {
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                let characters: Vec<char> = word.chars().collect();
                for chunk in characters.chunks(columns) {
                    lines.push(chunk.iter().collect());
                }
            } else if line.is_empty() {
                line.push_str(word);
            } else if line.chars().count() + 1 + word.chars().count() <= columns {
                line.push(' ');
                line.push_str(word);
            } else {
                lines.push(std::mem::take(&mut line));
                line.push_str(word);
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines.chunks(rows).map(|chunk| chunk.to_vec()).collect()
}

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
        let update = progress.update(&ProgressEvent::InventoryChanged, &inventory);
        assert!(
            update
                .completed_quests
                .contains(&"minicraft.quest.farming.crafting_hoe".to_owned())
        );

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
}
