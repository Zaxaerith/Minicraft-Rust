//! Java `FishingData` — loot-table load points for rod category rolls.
//!
//! Java reads `/resources/data/fishing/{fish,junk,tool,rare}_loot.txt` at
//! class-init. The Rust client loads those tables through `assets` when a
//! fishing catch is resolved; these names stay 1:1 with the Java files.
use super::fishing_rod::LEVEL_CHANCES;

/// Java `FishingData` category keys, ordered like `LEVEL_CHANCES` columns.
pub const CATEGORIES: [&str; 4] = ["fish", "junk", "tool", "rare"];

/// Java resource path stem.
pub const RESOURCE_STEM: &str = "/resources/data/fishing";

/// Path for one category loot file (Java `Load.loadFile` argument).
pub fn loot_path(category: &str) -> String {
    format!("{RESOURCE_STEM}/{category}_loot.txt")
}

/// Cumulative threshold table shared with `FishingRodItem.LEVEL_CHANCES`.
pub const fn level_chances() -> [[u8; 4]; 4] {
    LEVEL_CHANCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_paths_match_java_resources() {
        assert_eq!(loot_path("fish"), "/resources/data/fishing/fish_loot.txt");
        assert_eq!(loot_path("rare"), "/resources/data/fishing/rare_loot.txt");
        assert_eq!(CATEGORIES.len(), 4);
        assert_eq!(level_chances()[0][0], 44);
    }
}
