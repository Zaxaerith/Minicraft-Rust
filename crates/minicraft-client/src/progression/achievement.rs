//! Java `util/Achievement.java` + bundled `achievements.json`.

use serde_json::Value;

/// Java `Achievement`.
#[derive(Debug, Clone)]
pub struct Achievement {
    pub id: String,
    pub description: String,
    pub score: u32,
}

/// Load bundled Java `resources/achievements.json`.
pub fn achievements() -> Result<Vec<Achievement>, String> {
    let value: Value =
        serde_json::from_str(include_str!("../../../../assets/resources/achievements.json"))
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

#[cfg(test)]
mod tests {
    use super::achievements;

    #[test]
    fn bundled_achievements_parse() {
        assert_eq!(achievements().unwrap().len(), 17);
    }
}
