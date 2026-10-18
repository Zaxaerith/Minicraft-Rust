//! Host progression orchestrator — tutorials, quest series, achievements.
//!
//! Combines Java `AdvancementElement` + `Quest` + `TutorialElement` update
//! loops into the single `ProgressState` used by the Rust client world.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::advancement::{
    AdvancementElement, Criterion, ProgressEvent, parse_element, record_owner_hits,
};
use super::quest::{Quest, QuestGroup, QuestGroupView, QuestView, quests};
use super::tutorial::{TutorialStep, tutorials};
use serde_json::{Value, json};

/// Preserve 2.2.4 owners and append namespaced 2.3 definitions so old saved
/// completion cannot suppress new dye-cost unlock rules with the same Java ID.
fn recipe_elements() -> Result<Vec<AdvancementElement>, String> {
    let mut elements = Vec::new();
    for (source, prefix) in [
        (
            include_str!("../../../../assets/resources/recipes.json"),
            "",
        ),
        (
            include_str!("../../../../assets/expansion230/recipe_unlocks.json"),
            "java230:",
        ),
    ] {
        let value: Value = serde_json::from_str(source)
            .map_err(|error| format!("invalid bundled recipes: {error}"))?;
        for (id, element) in value
            .as_object()
            .ok_or("bundled recipes must be an object")?
        {
            elements.push(parse_element(&format!("{prefix}{id}"), element, false)?);
        }
    }
    Ok(elements)
}

fn recipe_name(name: &str) -> String {
    if name.eq_ignore_ascii_case("White Wool") {
        "WOOL".to_owned()
    } else {
        name.to_ascii_uppercase()
    }
}

fn recipe_product(name: &str) -> String {
    name.rsplit_once('_').map_or_else(
        || recipe_name(name),
        |(name, count)| format!("{}_{count}", recipe_name(name)),
    )
}

fn recipe_costs_match(
    recipe: &crate::item::RecipeView,
    costs: impl Iterator<Item = impl AsRef<str>>,
) -> bool {
    let mut expected = HashMap::<String, u32>::new();
    for cost in recipe.costs {
        *expected
            .entry(recipe_name(cost.item.display_name()))
            .or_default() += u32::from(cost.count);
    }
    if let Some((kind, tier)) = recipe.tool_cost {
        *expected
            .entry(recipe_name(
                &crate::item::ToolItem::new(kind, tier).display_name(),
            ))
            .or_default() += 1;
    }
    let mut actual = HashMap::<String, u32>::new();
    for cost in costs {
        let Some((name, amount)) = cost.as_ref().rsplit_once('_') else {
            return false;
        };
        let Ok(amount) = amount.parse::<u32>() else {
            return false;
        };
        let count = actual.entry(recipe_name(name)).or_default();
        *count = count.saturating_add(amount);
    }
    expected == actual
}

/// ISO local-date-time accepted by Java. New events use UTC; imported local
/// timestamps are kept verbatim. Gregorian conversion from Unix epoch days.
pub(crate) fn completion_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86400) as i64 + 719468;
    let era = days / 146097;
    let day_of_era = days - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}

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
    #[serde(default)]
    java_advancements: serde_json::Map<String, Value>,
    #[serde(default)]
    imported_tutorial_cursor: bool,
    #[serde(default)]
    tutorial_cursor: Option<String>,
    #[serde(default)]
    unlocking_hits: HashSet<String>,
    #[serde(default)]
    recipe_elements: Vec<AdvancementElement>,
    #[serde(default)]
    recipe_completed: HashSet<String>,
    #[serde(default)]
    unlocked_recipes: serde_json::Map<String, Value>,
    #[serde(default)]
    criterion_times: HashMap<String, String>,
    #[serde(default)]
    unlocking_times: HashMap<String, String>,
    #[serde(default)]
    imported_recipe_unlocks: bool,
    #[serde(default)]
    recipe_variants: HashMap<String, Vec<Vec<String>>>,
    #[serde(default)]
    recipe_catalog_version: u8,
}

impl ProgressState {
    pub(crate) fn begin_new_world(&mut self) {
        self.imported_recipe_unlocks = true;
        self.imported_tutorial_cursor = true;
        self.tutorial_cursor = self.tutorials.first().map(|step| step.id.clone());
    }
    /// Legacy native saves had no recipe-lock state: keep their available
    /// recipes, while Java imports obey the saved product AND cost definitions.
    pub fn recipe_is_unlocked(&self, recipe: &crate::item::RecipeView) -> bool {
        if !self.imported_recipe_unlocks {
            return true;
        }
        use crate::item::RecipeOutput;
        let amount = match recipe.output {
            RecipeOutput::Stack(stack) => stack.count,
            RecipeOutput::Tool(_) => 1,
        };
        let key = format!("{}_{amount}", recipe_name(&recipe.product_name()));
        self.recipe_variants.get(&key).is_some_and(|variants| {
            variants
                .iter()
                .any(|costs| recipe_costs_match(recipe, costs.iter()))
        }) || self.unlocked_recipes.iter().any(|(product, costs)| {
            if recipe_product(product) != key {
                return false;
            }
            let Some(costs) = costs.as_array() else {
                return false;
            };
            if costs.iter().any(|v| !v.is_string()) {
                return false;
            }
            recipe_costs_match(recipe, costs.iter().filter_map(Value::as_str))
        })
    }
    /// Java Save.writePlayer stores full recipe definitions, not recipe IDs.
    pub fn import_unlocked_recipes(&mut self, value: Value) -> Result<(), String> {
        let map = value.as_object().ok_or("Java recipes must be an object")?;
        for (product, costs) in map {
            if !costs
                .as_array()
                .is_some_and(|costs| costs.iter().all(Value::is_string))
            {
                return Err(format!("invalid Java recipe costs for {product}"));
            }
        }
        self.unlocked_recipes = map.clone();
        self.recipe_variants.clear();
        self.remember_unlocked_recipes();
        self.imported_recipe_unlocks = true;
        Ok(())
    }

    /// Keep earned legacy definitions even when the new catalog introduces
    /// another cost list for the same output. This migrates metadata only.
    fn remember_unlocked_recipes(&mut self) {
        for (product, costs) in &self.unlocked_recipes {
            let Some(costs) = costs.as_array() else {
                continue;
            };
            let Some(costs) = costs
                .iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let variants = self
                .recipe_variants
                .entry(recipe_product(product))
                .or_default();
            if !variants.contains(&costs) {
                variants.push(costs);
            }
        }
    }

    pub fn unlocked_recipes_json(&self) -> Value {
        Value::Object(self.unlocked_recipes.clone())
    }

    /// Preserve original timestamps and unknown entries, then restore known state.
    /// Importing completion does not reissue rewards.
    pub fn import_java_advancements(&mut self, value: Value) -> Result<(), String> {
        let map = value
            .as_object()
            .ok_or("Java advancements must be an object")?;
        self.java_advancements = map.clone();
        self.imported_tutorial_cursor = true;
        self.tutorial_cursor = map
            .get("CurrentOngoingTutorial")
            .and_then(Value::as_str)
            .map(str::to_owned);
        for (id, saved) in map {
            if id == "Version" || id == "CurrentOngoingTutorial" {
                continue;
            }
            if !saved.is_object() {
                return Err(format!("invalid Java advancement {id}"));
            }
            for (field, hits, times) in [
                (
                    "criteria",
                    &mut self.criterion_hits,
                    &mut self.criterion_times,
                ),
                (
                    "unlockingCriteria",
                    &mut self.unlocking_hits,
                    &mut self.unlocking_times,
                ),
            ] {
                if let Some(criteria) = saved.get(field) {
                    let criteria = criteria
                        .as_object()
                        .ok_or_else(|| format!("invalid {field} for {id}"))?;
                    for (name, time) in criteria {
                        if !time.is_string() {
                            return Err(format!("invalid criterion timestamp {id}:{name}"));
                        }
                        let key = format!("{id}:{name}");
                        hits.insert(key.clone());
                        times.insert(key, time.as_str().expect("validated timestamp").to_owned());
                    }
                }
            }
            if saved["done"].as_bool() == Some(true) {
                if self.tutorials.iter().any(|step| &step.id == id) {
                    self.tutorial_completed.insert(id.clone());
                }
                if self
                    .quest_groups
                    .iter()
                    .any(|group| group.quests.iter().any(|quest| &quest.id == id))
                {
                    self.quest_completed.insert(id.clone());
                }
                if self
                    .recipe_elements
                    .iter()
                    .any(|element| &element.key == id)
                {
                    self.recipe_completed.insert(id.clone());
                }
            }
            if saved["unlocked"].as_bool() == Some(true) {
                if self.quest_groups.iter().any(|group| &group.id == id) {
                    self.group_unlocked.insert(id.clone());
                } else {
                    self.quest_unlocked.insert(id.clone());
                }
            }
        }
        Ok(())
    }

    /// Java dev2 owners map to the namespaced native expansion definitions.
    /// Keep the original Java object for provenance/timestamps and native saves.
    pub fn import_java230_advancements(&mut self, value: Value) -> Result<(), String> {
        let original = value
            .as_object()
            .ok_or("Java advancements must be an object")?
            .clone();
        let expansion: Value = serde_json::from_str(include_str!(
            "../../../../assets/expansion230/recipe_unlocks.json"
        ))
        .map_err(|e| e.to_string())?;
        let mut mapped = original.clone();
        for id in expansion
            .as_object()
            .ok_or("invalid expansion recipes")?
            .keys()
        {
            if let Some(saved) = mapped.remove(id) {
                mapped.insert(format!("java230:{id}"), saved);
            }
        }
        self.import_java_advancements(Value::Object(mapped))?;
        self.java_advancements = original;
        Ok(())
    }

    /// Lossless export of imported Java progression; native completion sets are
    /// also retained in native saves even when Java has no matching definition.
    pub fn java_advancements_json(&self) -> Value {
        let mut saved = self.java_advancements.clone();
        saved.insert("Version".to_owned(), json!("2.2.4"));
        saved.remove("CurrentOngoingTutorial");
        let cursor = if self.imported_tutorial_cursor {
            self.tutorial_cursor.as_deref()
        } else {
            self.current_tutorial().map(|step| step.id.as_str())
        };
        if let Some(cursor) = cursor {
            saved.insert("CurrentOngoingTutorial".to_owned(), json!(cursor));
        }
        for (field, times) in [
            ("criteria", &self.criterion_times),
            ("unlockingCriteria", &self.unlocking_times),
        ] {
            for (key, time) in times {
                let Some((owner, name)) = key.rsplit_once(':') else {
                    continue;
                };
                let entry = saved.entry(owner.to_owned()).or_insert_with(|| json!({}));
                if let Some(entry) = entry.as_object_mut() {
                    let criteria = entry.entry(field.to_owned()).or_insert_with(|| json!({}));
                    if let Some(criteria) = criteria.as_object_mut() {
                        criteria.insert(name.to_owned(), json!(time));
                    }
                    if field == "criteria"
                        && (self.tutorials.iter().any(|step| step.id == owner)
                            || self
                                .quest_groups
                                .iter()
                                .any(|g| g.quests.iter().any(|q| q.id == owner))
                            || self.recipe_elements.iter().any(|e| e.key == owner))
                    {
                        entry.insert(
                            "done".to_owned(),
                            json!(
                                self.tutorial_completed.contains(owner)
                                    || self.quest_completed.contains(owner)
                                    || self.recipe_completed.contains(owner)
                            ),
                        );
                    } else if field == "unlockingCriteria"
                        && (self
                            .quest_groups
                            .iter()
                            .any(|g| g.id == owner || g.quests.iter().any(|q| q.id == owner)))
                    {
                        entry.insert(
                            "unlocked".to_owned(),
                            json!(
                                self.group_unlocked.contains(owner)
                                    || self.quest_unlocked.contains(owner)
                            ),
                        );
                    }
                }
            }
        }
        Value::Object(saved)
    }
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
            java_advancements: Default::default(),
            imported_tutorial_cursor: false,
            tutorial_cursor: None,
            unlocking_hits: Default::default(),
            recipe_elements: recipe_elements()?,
            recipe_completed: Default::default(),
            unlocked_recipes: Default::default(),
            criterion_times: Default::default(),
            unlocking_times: Default::default(),
            imported_recipe_unlocks: false,
            recipe_variants: HashMap::new(),
            recipe_catalog_version: 2,
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
        if self.recipe_catalog_version < 2 || self.recipe_elements.is_empty() {
            self.remember_unlocked_recipes();
            // Definitions are static metadata, not earned state; preserve all
            // recorded completions/timestamps and add the new catalog once.
            if let Ok(elements) = recipe_elements() {
                self.recipe_elements = elements;
                self.recipe_catalog_version = 2;
            }
        }
        for element in &self.recipe_elements {
            if self.recipe_completed.contains(&element.key) {
                continue;
            }
            record_owner_hits(
                &mut self.criterion_hits,
                &mut self.criterion_times,
                &element.key,
                &element.criteria,
                event,
                inventory,
            );
        }
        for element in &self.recipe_elements {
            if !self.recipe_completed.contains(&element.key)
                && self.owner_criteria_met(
                    &element.key,
                    &element.criteria,
                    &element.requirements,
                    event,
                    inventory,
                )
            {
                self.recipe_completed.insert(element.key.clone());
                if let Some(rewards) = &element.rewards {
                    for recipe in &rewards.recipes {
                        // Java's JSON object holds one cost list per product.
                        // Native progress also retains alternatives (e.g. three
                        // flowers yielding Light Gray Dye) without overwriting.
                        let variants = self
                            .recipe_variants
                            .entry(recipe_product(&recipe.product))
                            .or_default();
                        if !variants.contains(&recipe.costs) {
                            variants.push(recipe.costs.clone());
                        }
                        self.unlocked_recipes
                            .insert(recipe.product.clone(), json!(recipe.costs));
                    }
                }
            }
        }
        self.record_event_hits(event, inventory, tutorials_enabled, quests_enabled);
        let mut update = ProgressUpdate::default();
        if tutorials_enabled {
            for tutorial in &self.tutorials {
                if self.tutorial_completed.contains(&tutorial.id)
                    || (self.imported_tutorial_cursor
                        && self.tutorial_cursor.as_deref() != Some(&tutorial.id))
                {
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

        if tutorials_enabled
            && self.imported_tutorial_cursor
            && let Some(cursor) = &self.tutorial_cursor
            && self.tutorial_completed.contains(cursor)
        {
            self.tutorial_cursor = self
                .tutorials
                .iter()
                .position(|step| &step.id == cursor)
                .and_then(|i| self.tutorials.get(i + 1))
                .map(|step| step.id.clone());
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
        if self.imported_tutorial_cursor {
            return self
                .tutorial_cursor
                .as_ref()
                .and_then(|id| self.tutorials.iter().find(|step| &step.id == id));
        }
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
                    &mut self.criterion_times,
                    &tutorial.id,
                    &tutorial.criteria,
                    event,
                    inventory,
                );
            }
        }
        if quests_enabled {
            for group in &self.quest_groups {
                record_owner_hits(
                    &mut self.unlocking_hits,
                    &mut self.unlocking_times,
                    &group.id,
                    &group.unlocking,
                    event,
                    inventory,
                );
                for quest in &group.quests {
                    record_owner_hits(
                        &mut self.unlocking_hits,
                        &mut self.unlocking_times,
                        &quest.id,
                        &quest.unlocking,
                        event,
                        inventory,
                    );
                    record_owner_hits(
                        &mut self.criterion_hits,
                        &mut self.criterion_times,
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
            // New/empty inventories can fail hundreds of recipe criteria each
            // tick. An empty history cannot match; avoid allocating every key.
            || (!self.criterion_hits.is_empty()
                && self
                    .criterion_hits
                    .contains(&format!("{owner}:{}", criterion.name)))
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
        criteria.iter().all(|criterion| {
            criterion.matches(event, inventory)
                || self
                    .unlocking_hits
                    .contains(&format!("{owner}:{}", criterion.name))
        })
    }
}
