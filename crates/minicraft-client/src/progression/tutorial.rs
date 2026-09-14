//! Java `util/TutorialElement.java` — tutorial steps + bundled `tutorials.json`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::advancement::{Criterion, object, parse_element, required_string};

/// Java `TutorialElement` (extends `AdvancementElement`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorialStep {
    pub id: String,
    pub description: String,
    pub index: usize,
    pub criteria: Vec<Criterion>,
    #[serde(default)]
    pub requirements: Vec<Vec<String>>,
}

/// Load bundled Java `resources/tutorials.json`, ordered by `index`.
pub fn tutorials() -> Result<Vec<TutorialStep>, String> {
    let value: Value = serde_json::from_str(include_str!(
        "../../../../assets/resources/tutorials.json"
    ))
    .map_err(|error| format!("cannot parse bundled tutorials: {error}"))?;
    let mut steps = object(&value, "tutorial root")?
        .iter()
        .map(|(id, entry)| {
            let element = parse_element(id, entry, true)?;
            Ok(TutorialStep {
                id: element.key,
                description: required_string(entry, "description")?,
                index: entry["index"].as_u64().unwrap_or(0) as usize,
                criteria: element.criteria,
                requirements: element.requirements,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    steps.sort_by_key(|step| step.index);
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::tutorials;

    #[test]
    fn bundled_tutorials_match_java_224_counts() {
        let steps = tutorials().unwrap();
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0].id, "minicraft.tutorial.start_getting_wood");
        assert_eq!(steps[0].index, 0);
    }
}
