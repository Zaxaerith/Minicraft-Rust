//! Java `minicraft.saveload.Save` — serialize world slices into `.miniplussave`.
//!
//! Field order matches `Save.writeGame` / `writePlayer` / `writeInventory` /
//! `writeEntity` so a Rust-exported folder is readable by Minicraft+ 2.2.4.
//! Native persistence still goes through `storage::atomic_write` + `state.json`;
//! these helpers exist for Java-format export and round-trip tests.

use serde_json::{Value, json};

use crate::item::{ItemId, PotionKind};
use crate::world::{ActiveItem, GameMode, World, spawn};

use super::version::milestones;

/// Java `Save.extension`.
pub const EXTENSION: &str = ".miniplussave";

/// Current export stamp (Java `Game.VERSION`).
pub const EXPORT_VERSION: &str = "2.2.4";

/// Java `Save.writeGame` field list (including the terminal empty slot Java's
/// trailing comma produces).
pub fn game_fields(world: &World) -> Vec<String> {
    vec![
        EXPORT_VERSION.to_owned(),
        world.seed.to_string(),
        mode_field(
            world.mode,
            world.score_ticks,
            default_score_minutes(world.mode),
        ),
        // Java Updater.tickCount.
        world.clock.get().day_tick.to_string(),
        // Java Updater.gameTime.
        world.clock.get().game_time.to_string(),
        world.difficulty.to_string(),
        world.air_wizard_defeated.to_string(),
        world.quests_enabled.to_string(),
        world.tutorials_enabled.to_string(),
        world.obsidian_knight_defeated.to_string(),
        String::new(),
    ]
}

fn default_score_minutes(mode: GameMode) -> usize {
    // World does not retain PlayOptions.score_minutes; Score mode default.
    let _ = mode;
    20
}

fn mode_field(mode: GameMode, score_ticks: i32, score_minutes: usize) -> String {
    if mode == GameMode::Score {
        format!("{};{};{}", mode.index(), score_ticks, score_minutes)
    } else {
        mode.index().to_string()
    }
}

/// Java `Save.writePlayer` field list.
pub fn player_fields(world: &World) -> Vec<String> {
    let player = &world.player;
    vec![
        player.x.to_string(),
        player.y.to_string(),
        // Rust has no separate spawn point; Java still expects the slots.
        player.x.to_string(),
        player.y.to_string(),
        player.health.to_string(),
        player.max_health.saturating_sub(10).to_string(),
        player.hunger.to_string(),
        player.armor.to_string(),
        player.armor_damage_buffer.to_string(),
        player
            .armor_kind
            .map(|armor| armor.display_name().to_owned())
            .unwrap_or_else(|| "NULL".to_owned()),
        world.score.to_string(),
        world.current_level.to_string(),
        potion_effects_field(&player.potion_effects),
        // Shirt/clothing slot — Java stores an int color; export 0.
        "0".to_owned(),
        "{}".to_owned(),
        String::new(),
    ]
}

fn potion_effects_field(effects: &[u16; PotionKind::ALL.len()]) -> String {
    let mut body = String::new();
    for kind in PotionKind::ALL {
        let remaining = effects[kind.id()];
        if remaining > 0 {
            if !body.is_empty() {
                body.push(':');
            }
            body.push_str(kind.display_name());
            body.push(';');
            body.push_str(&remaining.to_string());
        }
    }
    format!("PotionEffects[{body}]")
}

/// Java `Save.writeInventory` — active item first, then inventory stacks/tools.
pub fn inventory_fields(world: &World) -> Vec<String> {
    let mut fields = Vec::new();
    let active_stack = match world.player.active_item {
        Some(ActiveItem::Stack(item)) => world
            .player
            .inventory
            .slots()
            .iter()
            .position(|stack| stack.item == item),
        _ => None,
    };
    if let Some(active) = world.player.active_item
        && let Some(encoded) = encode_active_item(world, active)
    {
        fields.push(encoded);
    }
    for (index, stack) in world.player.inventory.slots().iter().enumerate() {
        if Some(index) == active_stack {
            continue;
        }
        fields.push(encode_item(stack.item, stack.count));
    }
    for (index, tool) in world.player.inventory.tools().iter().enumerate() {
        if world.player.active_item == Some(ActiveItem::Tool(index)) {
            continue;
        }
        fields.push(format!("{}_{}", tool.display_name(), tool.durability));
    }
    fields.push(String::new());
    fields
}

fn encode_active_item(world: &World, active: ActiveItem) -> Option<String> {
    match active {
        ActiveItem::Stack(item) => world
            .player
            .inventory
            .slots()
            .iter()
            .find(|stack| stack.item == item)
            .map(|stack| encode_item(item, stack.count)),
        ActiveItem::Tool(index) => world
            .player
            .inventory
            .tools()
            .get(index)
            .map(|tool| format!("{}_{}", tool.display_name(), tool.durability)),
    }
}

fn encode_item(item: ItemId, count: u16) -> String {
    if item == ItemId::WateringCan {
        format!("{}_1", item.display_name())
    } else {
        format!("{}_{}", item.display_name(), count)
    }
}

/// Java `Save.writeEntity` for a local save (no eid / transient particles).
pub fn mob_entity_line(
    species: spawn::NaturalMob,
    x: i32,
    y: i32,
    health: u16,
    level: usize,
) -> String {
    format!(
        "{}[{}:{}:{}:{}]",
        mob_class_name(species),
        x,
        y,
        health,
        level
    )
}

fn mob_class_name(species: spawn::NaturalMob) -> &'static str {
    match species {
        spawn::NaturalMob::Slime => "Slime",
        spawn::NaturalMob::Zombie => "Zombie",
        spawn::NaturalMob::Creeper => "Creeper",
        spawn::NaturalMob::Skeleton => "Skeleton",
        spawn::NaturalMob::Snake => "Snake",
        spawn::NaturalMob::Knight => "Knight",
        spawn::NaturalMob::Cow => "Cow",
        spawn::NaturalMob::Pig => "Pig",
        spawn::NaturalMob::Sheep => "Sheep",
        spawn::NaturalMob::AirWizard => "AirWizard",
        spawn::NaturalMob::ObsidianKnight => "ObsidianKnight",
    }
}

/// Java `SignDisplay` signs.json body.
pub fn signs_json(world: &World) -> Value {
    let mut signs = Vec::new();
    let width = world.width.max(1);
    for (level, map) in world.signs.iter().enumerate() {
        for (index, text) in map {
            let x = index % width;
            let y = index / width;
            signs.push(json!({
                "level": level,
                "x": x,
                "y": y,
                "lines": text.split(' ').collect::<Vec<_>>(),
            }));
        }
    }
    json!({
        "Version": EXPORT_VERSION,
        "signs": signs,
    })
}

/// Java `Save.writeToFile(..., isWorldSave=true)` — comma-joined with trailing commas.
pub fn join_world_save_fields(fields: &[String]) -> String {
    let mut out = String::new();
    for field in fields {
        out.push_str(field);
        out.push(',');
    }
    out
}

/// Reference layout version used for round-trip checks.
pub fn export_version() -> super::version::Version {
    milestones::v_2_2_4()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemId;
    use crate::world::{GameMode, PlayOptions, WorldSpec};

    fn tiny_score_world() -> World {
        World::new_with_play_options(
            42,
            WorldSpec::new(128, 0, 0),
            PlayOptions {
                difficulty: 1,
                mode: GameMode::Score,
                score_minutes: 20,
                tutorials: true,
                quests: true,
                show_quests: true,
                custom_skin: false,
            },
        )
    }

    #[test]
    fn game_fields_match_java_write_game_order() {
        let mut world = tiny_score_world();
        world.score_ticks = 600;
        world.air_wizard_defeated = true;
        world.quests_enabled = true;
        world.tutorials_enabled = true;
        world.obsidian_knight_defeated = false;
        let fields = game_fields(&world);
        assert_eq!(fields[0], "2.2.4");
        assert_eq!(fields[1], "42");
        assert_eq!(fields[2], "3;600;20");
        assert_eq!(fields[5], "1");
        assert_eq!(fields[6], "true");
        assert_eq!(fields[7], "true");
        assert_eq!(fields[8], "true");
        assert_eq!(fields[9], "false");
        let joined = join_world_save_fields(&fields);
        assert!(joined.ends_with(','));
        assert!(joined.contains("3;600;20"));
    }

    #[test]
    fn inventory_fields_encode_stacks_and_tools() {
        let mut world = tiny_score_world();
        world.player.inventory.add(ItemId::Wood, 3);
        let mut tool =
            crate::item::ToolItem::new(crate::item::ToolKind::Pickaxe, crate::item::ToolTier::Iron);
        tool.durability = 70;
        world.player.inventory.add_tool(tool).unwrap();
        let fields = inventory_fields(&world);
        assert!(fields.iter().any(|field| field == "Wood_3"));
        assert!(
            fields.iter().any(|field| field == "Iron Pickaxe_70"),
            "{fields:?}"
        );
    }

    #[test]
    fn signs_json_round_trips_java_shape() {
        let mut world = tiny_score_world();
        world.signs = vec![Default::default(); 6];
        world.signs[1].insert(world.width, "HELLO RUST".to_owned());
        let value = signs_json(&world);
        assert_eq!(value["Version"], "2.2.4");
        let signs = value["signs"].as_array().unwrap();
        assert_eq!(signs.len(), 1);
        assert_eq!(signs[0]["level"], 1);
        assert_eq!(signs[0]["x"], 0);
        assert_eq!(signs[0]["y"], 1);
        assert_eq!(signs[0]["lines"][0], "HELLO");
        assert_eq!(signs[0]["lines"][1], "RUST");
    }

    #[test]
    fn java_export_counts_equipped_items_once_and_uses_calendar_tick() {
        let mut world = tiny_score_world();
        world.player.inventory.add(ItemId::Wood, 1000);
        world.player.active_item = Some(ActiveItem::Stack(ItemId::Wood));
        let fields = inventory_fields(&world);
        let exported: u16 = fields
            .iter()
            .filter_map(|field| field.strip_prefix("Wood_"))
            .map(|count| count.parse::<u16>().unwrap())
            .sum();
        assert_eq!(exported, 1000);
        let index = world
            .player
            .inventory
            .add_tool(crate::item::ToolItem::new(
                crate::item::ToolKind::Pickaxe,
                crate::item::ToolTier::Iron,
            ))
            .unwrap();
        world.player.active_item = Some(ActiveItem::Tool(index));
        assert_eq!(
            inventory_fields(&world)
                .iter()
                .filter(|field| field.starts_with("Iron Pickaxe_"))
                .count(),
            1
        );
        world.tick = 999;
        world.clock.set_day_tick(123);
        assert_eq!(game_fields(&world)[3], "123");
    }

    #[test]
    fn mob_entity_line_matches_java_bracket_form() {
        assert_eq!(
            mob_entity_line(spawn::NaturalMob::Cow, 8, 8, 4, 1),
            "Cow[8:8:4:1]"
        );
    }

    #[test]
    fn potion_field_uses_java_bracket_syntax() {
        let mut effects = [0u16; PotionKind::ALL.len()];
        effects[PotionKind::Speed.id()] = 120;
        assert_eq!(potion_effects_field(&effects), "PotionEffects[Speed;120]");
    }
}
