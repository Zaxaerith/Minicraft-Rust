//! Java `minicraft.saveload.LegacyLoad` / pre-name-migration helpers.
//!
//! `sub_old_name` is a 1:1 port of `Load.subOldName` plus the LegacyLoad
//! prefix replacements. Callers apply it before resolving items against the
//! modern registry.

use super::version::{Version, milestones};

/// Java `Load.subOldName(String, Version)`.
pub fn sub_old_name(name: &str, world_ver: &Version) -> String {
    let mut name = name.to_owned();

    if world_ver.less_than(&milestones::v_1_9_4_dev4()) {
        name = name
            .replace("Hatchet", "Axe")
            .replace("Pick", "Pickaxe")
            .replace("Pickaxeaxe", "Pickaxe")
            .replace("Spade", "Shovel")
            .replace("Pow glove", "Power Glove")
            .replace("II", "")
            .replace("W.Bucket", "Water Bucket")
            .replace("L.Bucket", "Lava Bucket")
            .replace("G.Apple", "Gold Apple")
            .replace("St.", "Stone")
            .replace("Ob.", "Obsidian")
            .replace("I.Lantern", "Iron Lantern")
            .replace("G.Lantern", "Gold Lantern")
            .replace("BrickWall", "Wall")
            .replace("Brick", " Brick")
            .replace("Wall", " Wall")
            .replace("  ", " ");
        if name == "Bucket" {
            name = "Empty Bucket".to_owned();
        }
    }

    if world_ver.less_than(&Version::parse("1.9.4")) {
        name = name
            .replace("I.Armor", "Iron Armor")
            .replace("S.Armor", "Snake Armor")
            .replace("L.Armor", "Leather Armor")
            .replace("G.Armor", "Gold Armor")
            .replace("BrickWall", "Wall");
    }

    if world_ver.less_than(&milestones::v_2_0_6_dev3()) && name == "Fishing Rod" {
        name = "Wood Fishing Rod".to_owned();
    }

    if world_ver.less_than(&Version::parse("2.0.6")) && name.starts_with("Pork Chop") {
        name = name.replace("Pork Chop", "Cooked Pork");
    }

    if world_ver.less_than(&milestones::v_2_0_7_dev1()) && name.starts_with("Seeds") {
        name = name.replace("Seeds", "Wheat Seeds");
    }

    if world_ver.less_than(&milestones::v_2_1_0_dev2()) && name.starts_with("Shear") {
        name = name.replace("Shear", "Shears");
    }

    if world_ver.less_than(&milestones::v_2_2_0_dev4()) && name.starts_with("Potion") {
        name = name.replace("Potion", "Awkward Potion");
    }

    if world_ver.less_than(&Version::parse("2.3.0-dev1")) {
        if name.starts_with("Wool") {
            name = name.replace("Wool", "White Wool");
        }
        if name.starts_with("Flower") {
            name = name.replace("Flower", "Oxeye Daisy");
        }
    }

    name
}

/// Java `LegacyLoad.subOldName` — prefix replacements applied before `Load.subOldName`.
pub fn legacy_prefixed_name(old_name: &str, world_ver: &Version) -> String {
    let new_name = old_name
        .replace("P.", "Potion")
        .replace("Fish Rod", "Fishing Rod")
        .replace("bed", "Bed");
    sub_old_name(&new_name, world_ver)
}

/// Java difficulty index adjustment: versions before `1.9.3-dev3` used a
/// 1-based difficulty ordinal.
pub fn adjust_difficulty_index(diff_idx: i32, world_ver: &Version) -> i32 {
    if world_ver.less_than(&milestones::v_1_9_3_dev3()) {
        diff_idx - 1
    } else {
        diff_idx
    }
}

/// Java `loadMode` mode-index adjustment: versions at or before `2.0.3`
/// used a 1-based mode ordinal.
pub fn adjust_mode_index(mode: i32, world_ver: &Version) -> i32 {
    if world_ver.compare(&milestones::v_2_0_3()) != std::cmp::Ordering::Greater {
        mode - 1
    } else {
        mode
    }
}

/// Whether the Game file carries a world seed (`2.2.0-dev1`+).
pub fn has_world_seed(world_ver: &Version) -> bool {
    world_ver.at_least(&milestones::v_2_2_0_dev1())
}

/// Whether Level headers include a per-level seed (`2.0.7-dev2`+).
pub fn level_has_seed(world_ver: &Version) -> bool {
    world_ver.at_least(&milestones::v_2_0_7_dev2())
}

/// Whether quests/tutorials flags are present in Game (`2.2.0-dev1`+).
pub fn has_quest_flags(world_ver: &Version) -> bool {
    world_ver.at_least(&milestones::v_2_2_0_dev1())
}

/// Whether player extra health is present (`2.2.0-dev3`+).
pub fn has_extra_health(world_ver: &Version) -> bool {
    world_ver.at_least(&milestones::v_2_2_0_dev3())
}

/// Whether player hunger is present (`2.0.4-dev7`+).
pub fn has_hunger(world_ver: &Version) -> bool {
    world_ver.at_least(&milestones::v_2_0_4_dev7())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renames_pre_1_9_4_tools() {
        let ver = Version::parse("1.9.3");
        assert_eq!(sub_old_name("Hatchet", &ver), "Axe");
        assert_eq!(sub_old_name("G.Apple", &ver), "Gold Apple");
        assert_eq!(sub_old_name("I.Armor", &ver), "Iron Armor");
    }

    #[test]
    fn modern_versions_keep_names() {
        let ver = milestones::v_2_2_4();
        assert_eq!(sub_old_name("Iron Pickaxe", &ver), "Iron Pickaxe");
        assert_eq!(sub_old_name("Wool", &ver), "White Wool");
    }

    #[test]
    fn legacy_prefix_replacements() {
        let ver = Version::parse("1.8");
        // Java LegacyLoad.subOldName replaces "P." with "Potion", then
        // Load.subOldName's <2.2.0-dev4 rule replaces "Potion" prefix with
        // "Awkward Potion" — same as Java.
        assert_eq!(legacy_prefixed_name("P.Speed", &ver), "Awkward PotionSpeed");
        assert_eq!(legacy_prefixed_name("Fish Rod", &ver), "Wood Fishing Rod");
    }

    #[test]
    fn difficulty_and_mode_shift_for_old_ordinals() {
        let old = Version::parse("1.9.2");
        assert_eq!(adjust_difficulty_index(2, &old), 1);
        assert_eq!(adjust_mode_index(1, &old), 0);
        let modern = milestones::v_2_2_4();
        assert_eq!(adjust_difficulty_index(2, &modern), 2);
        assert_eq!(adjust_mode_index(0, &modern), 0);
    }
}
