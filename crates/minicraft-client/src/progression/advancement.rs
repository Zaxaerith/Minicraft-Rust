//! Java `util/AdvancementElement.java` — advancement criteria, triggers, rewards.
//!
//! The Rust host evaluates criteria against `ProgressEvent` + inventory rather
//! than a global trigger registry. Matching semantics (inventory item lists,
//! count ranges, location/tile filters, placed-tile / used-on-tile) follow the
//! Java `AdvancementTrigger` condition handlers 1:1 for the bundled JSON.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Rust-side event that drives criterion evaluation (Java trigger payloads).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent {
    InventoryChanged,
    ItemUsedOnTile { item: String, tile: String },
    PlacedTile { tile: String },
}

/// Java `Rangeable<T>` — optional min/max inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rangeable<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T> Rangeable<T> {
    pub fn is_absent(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }
}

impl Rangeable<i64> {
    pub fn in_range(&self, value: i64) -> bool {
        if self.min.is_none() && self.max.is_none() {
            return true;
        }
        if let Some(min) = self.min
            && value < min
        {
            return false;
        }
        if let Some(max) = self.max
            && value > max
        {
            return false;
        }
        true
    }
}

/// Java `ItemConditions` — item name alternatives with optional count range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemRequirement {
    pub alternatives: Vec<String>,
    pub minimum: u16,
    /// Optional upper bound (`count.max`); `None` when only min is present.
    #[serde(default)]
    pub maximum: Option<u16>,
    /// Java durability range (unused by bundled quest/tutorial JSON).
    #[serde(default)]
    pub durability: Option<Rangeable<u16>>,
}

impl ItemRequirement {
    fn matches_count(&self, count: u16) -> bool {
        if count < self.minimum {
            return false;
        }
        if let Some(maximum) = self.maximum
            && count > maximum
        {
            return false;
        }
        true
    }
}

/// Java `ElementCriterion` conditions for the three bundled triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Criterion {
    pub name: String,
    pub trigger: String,
    pub items: Vec<ItemRequirement>,
    pub used_item: Vec<String>,
    pub location_tiles: Vec<String>,
    pub placed_tile: Option<String>,
    /// Java inventory slots.empty / slots.full / slots.occupied ranges.
    /// Host inventory has no max-slots model; stored for JSON fidelity.
    #[serde(default)]
    pub slots_empty: Option<Rangeable<i64>>,
    #[serde(default)]
    pub slots_full: Option<Rangeable<i64>>,
    #[serde(default)]
    pub slots_occupied: Option<Rangeable<i64>>,
}

/// Java `ElementRewards` — item stack tokens (`"wheat seeds_8"`) and recipes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementRewards {
    pub items: Vec<String>,
    pub recipes: Vec<RecipeReward>,
}

/// Java `Recipe` reward entry loaded from `rewards.recipes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeReward {
    pub product: String,
    pub costs: Vec<String>,
}

/// Java `AdvancementElement` — key, description, criteria, rewards, requirement sets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancementElement {
    pub key: String,
    pub description: Option<String>,
    pub criteria: Vec<Criterion>,
    /// Java `requirements`: AND across sublists, OR within a sublist.
    /// Empty means "all criteria must complete".
    pub requirements: Vec<Vec<String>>,
    pub rewards: Option<ElementRewards>,
    pub unlocking_criteria: Vec<Criterion>,
    /// Java `unlocking_requirements` — same AND/OR shape as `requirements`.
    pub unlocking_requirements: Vec<Vec<String>>,
}

impl Criterion {
    /// Java trigger condition matching for the bundled trigger set.
    pub fn matches(&self, event: &ProgressEvent, inventory: &HashMap<String, u16>) -> bool {
        match self.trigger.as_str() {
            "inventory_changed" => {
                // Host inventory is a name→count map (no max-slot ceiling),
                // so slots.empty / slots.full cannot be evaluated 1:1.
                if let Some(range) = self.slots_occupied
                    && !range.is_absent()
                    && !range.in_range(inventory.len() as i64)
                {
                    return false;
                }
                self.items.iter().all(|requirement| {
                    requirement.alternatives.iter().any(|item| {
                        let count = inventory.get(item).copied().unwrap_or(0);
                        requirement.matches_count(count)
                    })
                })
            }
            "item_used_on_tile" => {
                let ProgressEvent::ItemUsedOnTile { item, tile } = event else {
                    return false;
                };
                (self.used_item.is_empty()
                    || self
                        .used_item
                        .iter()
                        .any(|expected| expected.eq_ignore_ascii_case(item)))
                    && (self.location_tiles.is_empty()
                        || self
                            .location_tiles
                            .iter()
                            .any(|expected| expected.eq_ignore_ascii_case(tile)))
            }
            "placed_tile" => {
                let ProgressEvent::PlacedTile { tile } = event else {
                    return false;
                };
                self.placed_tile
                    .as_ref()
                    .is_none_or(|placed| placed.eq_ignore_ascii_case(tile))
            }
            _ => false,
        }
    }
}

impl AdvancementElement {
    /// Java `checkIsCompleted` — empty requirements → all criteria; else AND/OR sets.
    pub fn criteria_completed(&self, completed: &HashSet<String>) -> bool {
        if self.requirements.is_empty() {
            self.criteria
                .iter()
                .all(|criterion| completed.contains(&criterion.name))
        } else {
            self.requirements.iter().all(|sublist| {
                sublist.iter().any(|name| {
                    completed.contains(name)
                        || self
                            .criterion_named(name)
                            .is_some_and(|c| completed.contains(&c.name))
                })
            })
        }
    }

    /// Java `isUnlockable` — empty unlocking_requirements → all unlocking criteria.
    pub fn unlocking_met(&self, completed: &HashSet<String>) -> bool {
        if self.unlocking_requirements.is_empty() {
            self.unlocking_criteria
                .iter()
                .all(|criterion| completed.contains(&criterion.name))
        } else {
            self.unlocking_requirements
                .iter()
                .all(|sublist| sublist.iter().any(|name| completed.contains(name)))
        }
    }

    fn criterion_named(&self, name: &str) -> Option<&Criterion> {
        self.criteria.iter().find(|c| c.name == name)
    }
}

/// Parse an advancement element object (Java `loadAdvancementElement` body).
pub fn parse_element(
    key: &str,
    json: &Value,
    displayable: bool,
) -> Result<AdvancementElement, String> {
    let criteria = parse_criteria_map(json.get("criteria"))?;
    if criteria.is_empty() && json.get("criteria").and_then(Value::as_object).is_some() {
        return Err(format!("criteria is empty for {key}"));
    }
    let requirements = parse_requirement_sets(json.get("requirements"));
    let unlocking_criteria = parse_criteria_map(json.get("unlocking_criteria"))?;
    let unlocking_requirements = parse_requirement_sets(json.get("unlocking_requirements"));
    let rewards = parse_rewards(json.get("rewards"));
    Ok(AdvancementElement {
        key: key.to_owned(),
        description: if displayable {
            json.get("description")
                .and_then(Value::as_str)
                .map(str::to_owned)
        } else {
            None
        },
        criteria,
        requirements,
        rewards,
        unlocking_criteria,
        unlocking_requirements,
    })
}

/// Java `loadRewards`.
pub fn parse_rewards(json: Option<&Value>) -> Option<ElementRewards> {
    let json = json?;
    let items = json
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let recipes = json
        .get("recipes")
        .and_then(Value::as_object)
        .map(|recipes| {
            recipes
                .iter()
                .map(|(product, costs)| RecipeReward {
                    product: product.clone(),
                    costs: costs
                        .as_array()
                        .map(|costs| {
                            costs
                                .iter()
                                .filter_map(|c| c.as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(ElementRewards { items, recipes })
}

fn parse_criteria_map(value: Option<&Value>) -> Result<Vec<Criterion>, String> {
    let Some(criteria) = value.and_then(Value::as_object) else {
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
                            let (minimum, maximum) = parse_count_range(requirement);
                            ItemRequirement {
                                alternatives,
                                minimum,
                                maximum,
                                durability: parse_range_u16(requirement.get("durability")),
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
                trigger: criterion["trigger"]
                    .as_str()
                    .ok_or_else(|| format!("bundled content is missing trigger for {name}"))?
                    .to_owned(),
                items,
                used_item,
                location_tiles,
                placed_tile: conditions["tile"].as_str().map(normalize),
                slots_empty: parse_range_i64(conditions["slots"].get("empty")),
                slots_full: parse_range_i64(conditions["slots"].get("full")),
                slots_occupied: parse_range_i64(conditions["slots"].get("occupied")),
            })
        })
        .collect()
}

fn parse_count_range(requirement: &Value) -> (u16, Option<u16>) {
    if let Some(count) = requirement.get("count") {
        if let Some(exact) = count.as_u64() {
            return (exact as u16, Some(exact as u16));
        }
        let min = count["min"].as_u64().unwrap_or(1) as u16;
        let max = count["max"].as_u64().map(|v| v as u16);
        return (min, max);
    }
    // Java JSON quirk: a named field other than `items` may carry the range
    // (e.g. `"iron": { "min": 30 }` in quests.json).
    if let Some(fields) = requirement.as_object() {
        for (key, range) in fields {
            if key.as_str() == "items" {
                continue;
            }
            let min = range["min"].as_u64();
            let max = range["max"].as_u64();
            if min.is_some() || max.is_some() {
                return (min.unwrap_or(1) as u16, max.map(|v| v as u16));
            }
        }
    }
    (1, None)
}

fn parse_range_i64(value: Option<&Value>) -> Option<Rangeable<i64>> {
    let value = value?;
    if let Some(exact) = value.as_i64() {
        return Some(Rangeable {
            min: Some(exact),
            max: Some(exact),
        });
    }
    if !value.is_object() {
        return None;
    }
    Some(Rangeable {
        min: value["min"].as_i64(),
        max: value["max"].as_i64(),
    })
}

fn parse_range_u16(value: Option<&Value>) -> Option<Rangeable<u16>> {
    let value = value?;
    if let Some(exact) = value.as_u64() {
        return Some(Rangeable {
            min: Some(exact as u16),
            max: Some(exact as u16),
        });
    }
    if !value.is_object() {
        return None;
    }
    Some(Rangeable {
        min: value["min"].as_u64().map(|v| v as u16),
        max: value["max"].as_u64().map(|v| v as u16),
    })
}

fn parse_requirement_sets(value: Option<&Value>) -> Vec<Vec<String>> {
    let Some(sets) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    sets.iter()
        .filter_map(|set| set.as_array())
        .map(|set| {
            set.iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .collect()
}

pub(crate) fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn object<'a>(
    value: &'a Value,
    label: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("bundled {label} must be an object"))
}

pub(crate) fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("bundled content is missing {key}"))
}

/// Record criteria whose trigger matches the current event (Java pending set).
pub(crate) fn record_owner_hits(
    hits: &mut HashSet<String>,
    times: &mut HashMap<String, String>,
    owner: &str,
    criteria: &[Criterion],
    event: &ProgressEvent,
    inventory: &HashMap<String, u16>,
) {
    for criterion in criteria {
        if criterion.matches(event, inventory) {
            let key = format!("{owner}:{}", criterion.name);
            if hits.insert(key.clone()) {
                times.insert(key, super::progress::completion_timestamp());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Criterion, ProgressEvent, parse_count_range};
    use serde_json::json;
    use std::collections::HashMap;

    fn inv(entries: &[(&str, u16)]) -> HashMap<String, u16> {
        entries.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
    }

    #[test]
    fn inventory_changed_matches_alternatives_and_min_count() {
        let criterion = Criterion {
            name: "has_hoe".into(),
            trigger: "inventory_changed".into(),
            items: vec![super::ItemRequirement {
                alternatives: vec!["wood hoe".into(), "iron hoe".into()],
                minimum: 1,
                maximum: None,
                durability: None,
            }],
            used_item: Vec::new(),
            location_tiles: Vec::new(),
            placed_tile: None,
            slots_empty: None,
            slots_full: None,
            slots_occupied: None,
        };
        assert!(criterion.matches(&ProgressEvent::InventoryChanged, &inv(&[("wood hoe", 1)])));
        assert!(!criterion.matches(&ProgressEvent::InventoryChanged, &inv(&[])));
    }

    #[test]
    fn item_used_on_tile_matches_item_and_tile() {
        let criterion = Criterion {
            name: "has_hoe".into(),
            trigger: "item_used_on_tile".into(),
            items: Vec::new(),
            used_item: vec!["wood hoe".into()],
            location_tiles: vec!["dirt".into()],
            placed_tile: None,
            slots_empty: None,
            slots_full: None,
            slots_occupied: None,
        };
        assert!(criterion.matches(
            &ProgressEvent::ItemUsedOnTile {
                item: "wood hoe".into(),
                tile: "dirt".into()
            },
            &HashMap::new()
        ));
        assert!(!criterion.matches(
            &ProgressEvent::ItemUsedOnTile {
                item: "wood hoe".into(),
                tile: "grass".into()
            },
            &HashMap::new()
        ));
    }

    #[test]
    fn count_range_accepts_named_java_quirk() {
        let (min, max) = parse_count_range(&json!({
            "items": ["iron"],
            "iron": { "min": 30 }
        }));
        assert_eq!(min, 30);
        assert_eq!(max, None);
    }
}
