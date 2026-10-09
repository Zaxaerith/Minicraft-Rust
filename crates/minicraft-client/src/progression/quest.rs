//! Java `util/Quest.java` — Quest / QuestSeries + bundled `quests.json`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::advancement::{Criterion, object, parse_element, required_string};

/// Java `Quest` (extends `AdvancementElement`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub description: String,
    pub parent: Option<String>,
    pub rewards: Vec<String>,
    #[serde(default)]
    pub has_requirements: bool,
    pub criteria: Vec<Criterion>,
    pub unlocking: Vec<Criterion>,
    /// Java `requirements` AND/OR sets (empty ⇒ all criteria).
    #[serde(default)]
    pub requirements: Vec<Vec<String>>,
    #[serde(default)]
    pub unlocking_requirements: Vec<Vec<String>>,
}

/// Java `Quest.QuestSeries` (extends `AdvancementElement`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestGroup {
    pub id: String,
    pub description: String,
    pub quests: Vec<Quest>,
    pub unlocking: Vec<Criterion>,
    #[serde(default)]
    pub requirements: Vec<Vec<String>>,
    #[serde(default)]
    pub unlocking_requirements: Vec<Vec<String>>,
}

/// Host view model for the quest-group tab UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestGroupView {
    pub index: usize,
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub completed_quests: usize,
    pub ongoing_quests: usize,
    pub total_quests: usize,
}

/// Host view model for the quest list UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestView {
    pub index: usize,
    pub id: String,
    pub description: String,
    pub parent: Option<String>,
    pub has_requirements: bool,
    pub completed: bool,
    pub criteria_completed: usize,
    pub criteria_total: usize,
}

/// Load bundled Java `resources/quests.json`.
pub fn quests() -> Result<Vec<QuestGroup>, String> {
    let value: Value =
        serde_json::from_str(include_str!("../../../../assets/resources/quests.json"))
            .map_err(|error| format!("cannot parse bundled quests: {error}"))?;
    object(&value, "quest root")?
        .iter()
        .map(|(id, group)| {
            let quests = object(&group["quests"], "quest group")?
                .iter()
                .map(|(quest_id, quest)| {
                    let element = parse_element(quest_id, quest, true)?;
                    let rewards = quest
                        .get("rewards")
                        .map(|rewards| {
                            rewards["items"]
                                .as_array()
                                .map(|items| {
                                    items
                                        .iter()
                                        .filter_map(|item| item.as_str().map(str::to_owned))
                                        .collect()
                                })
                                .unwrap_or_default()
                        })
                        .unwrap_or_else(|| {
                            element
                                .rewards
                                .as_ref()
                                .map(|r| r.items.clone())
                                .unwrap_or_default()
                        });
                    Ok(Quest {
                        id: element.key,
                        description: required_string(quest, "description")?,
                        parent: quest["parent"].as_str().map(str::to_owned),
                        has_requirements: quest["requirements"]
                            .as_array()
                            .is_some_and(|requirements| !requirements.is_empty()),
                        rewards,
                        criteria: element.criteria,
                        unlocking: element.unlocking_criteria,
                        requirements: element.requirements,
                        unlocking_requirements: element.unlocking_requirements,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let group_element = parse_element(id, group, true)?;
            Ok(QuestGroup {
                id: group_element.key,
                description: required_string(group, "description")?,
                quests,
                unlocking: group_element.unlocking_criteria,
                requirements: group_element.requirements,
                unlocking_requirements: group_element.unlocking_requirements,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::quests;

    #[test]
    fn bundled_quests_match_java_224_counts() {
        let groups = quests().unwrap();
        assert_eq!(groups.len(), 4);
        let total: usize = groups.iter().map(|g| g.quests.len()).sum();
        assert_eq!(total, 14);
        assert_eq!(groups[0].id, "minicraft.quest.farming");
    }
}
