//! Java `minicraft.saveload.Load` — import Minicraft+ `.miniplussave` worlds.
//!
//! Maps Java Game/Level/Player/Inventory/Entities/signs into the native Rust
//! `World`. Version gates follow Java `Load.loadGame` / `loadPlayer` /
//! `loadInventory`; pre-2.2 name migrations live in `super::legacy_load`.

use std::path::Path;

use serde_json::Value;

use crate::entity::EntityArena;
use crate::item::{ArmorKind, Inventory, ItemId, PotionKind, ToolItem, ToolKind, ToolTier};

use crate::world::{
    ActiveItem, Direction, FurnitureKind, GameMode, Level, PlayOptions, Tile, World, WorldSpec,
    spawn,
};

use super::legacy_load::{self, sub_old_name};
use super::storage::read_text_limited;
use super::version::{Version, milestones};

/// Reference layout stamp (Java `Game.VERSION` for this port).
#[allow(dead_code)]
const JAVA_VERSION: &str = "2.2.4";
const LEVEL_COUNT: usize = 6;
const MAX_JAVA_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_HEALTH: u8 = crate::world::MAX_HEALTH;
const MAX_STAT: u8 = crate::world::MAX_STAT;
const DAY_LENGTH: u32 = crate::world::DAY_LENGTH;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaSaveInfo {
    pub seed: i64,
    pub size: usize,
    pub mode: GameMode,
    pub score_minutes: usize,
}

pub fn probe_java_save(directory: &Path) -> Result<Option<JavaSaveInfo>, String> {
    let game_path = directory.join("Game.miniplussave");
    if !game_path.exists() {
        return Ok(None);
    }
    let game = read_save(&game_path)?;
    let world_ver = require_version(&game)?;
    let level = read_save(&directory.join("Level0.miniplussave"))?;
    if level.len() < 4 {
        return Err("Java Level0 save has no level header".to_owned());
    }
    let width = parse::<usize>(&level[0], "level width")?;
    let height = parse::<usize>(&level[1], "level height")?;
    if width != height || !matches!(width, 128 | 256 | 512) {
        return Err(format!(
            "unsupported Java world dimensions {width}x{height}"
        ));
    }
    let (mode, _, score_minutes) = parse_mode(field(&game, 2, "mode")?, &world_ver)?;
    Ok(Some(JavaSaveInfo {
        seed: parse_seed(&game, &world_ver)?,
        size: width,
        mode,
        score_minutes,
    }))
}

pub fn import_java_save(directory: &Path) -> Result<World, String> {
    let game = read_save(&directory.join("Game.miniplussave"))?;
    let world_ver = require_version(&game)?;
    // This stage ports gameplay into native worlds, not Java 2.3 world layout
    // migration. In particular Torch base IDs and Flower data changed upstream;
    // accepting them as 2.2.4 would silently reinterpret stored terrain/state.
    if world_ver.at_least(&Version::parse("2.3.0-dev1")) {
        return Err("Java 2.3 world import is not supported yet; colored items are supported in native Rust worlds. The Java world was not modified.".to_owned());
    }

    let seed = parse_seed(&game, &world_ver)?;
    let (mode, score_ticks, score_minutes) = parse_mode(field(&game, 2, "mode")?, &world_ver)?;
    let raw_difficulty = parse::<i32>(field(&game, 5, "difficulty")?, "difficulty")?;
    let difficulty =
        (legacy_load::adjust_difficulty_index(raw_difficulty, &world_ver)).clamp(0, 2) as usize;

    let first_level = read_save(&directory.join("Level0.miniplussave"))?;
    let width = parse::<usize>(field(&first_level, 0, "level width")?, "level width")?;
    let height = parse::<usize>(field(&first_level, 1, "level height")?, "level height")?;
    if width != height || width == 0 || width > 512 {
        return Err(format!(
            "unsupported Java world dimensions {width}x{height}"
        ));
    }

    let (quests, tutorials) = if legacy_load::has_quest_flags(&world_ver) {
        (
            parse_bool(field(&game, 7, "quests")?),
            parse_bool(field(&game, 8, "tutorials")?),
        )
    } else {
        (true, true)
    };

    let mut world = World::new_with_play_options(
        seed,
        WorldSpec::new(width, 0, 0),
        PlayOptions {
            difficulty,
            mode,
            score_minutes,
            tutorials,
            quests,
            show_quests: true,
            custom_skin: false,
        },
    );
    world.width = width;
    world.height = height;
    world.seed = seed;
    let day_tick = parse::<u64>(field(&game, 3, "tick count")?, "tick count")?;
    let mut game_time = parse::<i32>(field(&game, 4, "game time")?, "game time")?;
    // Java: versions before 1.9.3-dev2 force gameTime = 65000 (time-cheat guard).
    if world_ver.less_than(&milestones::v_1_9_3_dev2()) {
        game_time = 65_000;
    }
    world.clock.set_game_time(game_time);
    world
        .clock
        .set_day_tick((day_tick % DAY_LENGTH as u64) as u32);
    // Compatibility day estimate; Java gameTime can wrap negative. Do not
    // seed simulation cadence with this focused (including menu) counter.
    world.days = game_time.max(0) as u32 / DAY_LENGTH + 1;
    world.difficulty = difficulty;
    world.mode = mode;
    world.score_ticks = score_ticks;
    world.score_minutes = score_minutes;
    world.air_wizard_defeated = parse_bool(field(&game, 6, "Air Wizard state")?);
    // Java writeGame always emits ObsidianKnight.beaten as field 9; older
    // readers ignored it. Import when present.
    world.obsidian_knight_defeated = game.get(9).is_some_and(|value| parse_bool(value));
    // Java loadGame grants the AirWizard achievement when importing pre-2.1.0
    // saves whose Unlocks.json may lack it. Grant whenever the flag is set so
    // the achievement state always tracks the boss flag on import.
    if world.air_wizard_defeated {
        world
            .progress
            .unlock_achievement("minicraft.achievement.airwizard");
    }
    if world.obsidian_knight_defeated {
        world
            .progress
            .unlock_achievement("minicraft.achievement.obsidianknight");
    }

    let expected = width
        .checked_mul(height)
        .ok_or_else(|| "Java world dimensions overflow".to_owned())?;
    let has_level_seed = legacy_load::level_has_seed(&world_ver);
    let mut levels = Vec::with_capacity(LEVEL_COUNT);
    for index in 0..LEVEL_COUNT {
        let level_path = directory.join(format!("Level{index}.miniplussave"));
        let data_path = directory.join(format!("Level{index}data.miniplussave"));
        let level = if index == 0 {
            first_level.clone()
        } else {
            read_save(&level_path)?
        };
        let data = read_save(&data_path)?;
        let header_fields = if has_level_seed { 4 } else { 3 };
        if level.len() < expected + header_fields || data.len() < expected {
            return Err(format!(
                "Java level {index} is truncated (tiles {}, data {}, expected {expected})",
                level.len().saturating_sub(header_fields),
                data.len()
            ));
        }
        let level_width = parse::<usize>(field(&level, 0, "level width")?, "level width")?;
        let level_height = parse::<usize>(field(&level, 1, "level height")?, "level height")?;
        if level_width != width || level_height != height {
            return Err(format!("Java level {index} dimensions do not match Level0"));
        }
        // Java 2.0.7-dev2+ stores seed at index 2, depth at index 3.
        let depth_index = if has_level_seed { 3 } else { 2 };
        let depth = parse::<i8>(field(&level, depth_index, "level depth")?, "level depth")?;
        let tile_offset = if has_level_seed { 4 } else { 3 };
        let mut tiles = vec![Tile::Grass; expected];
        let mut tile_data = vec![0; expected];
        // Java writes x as the outer loop but indexes its arrays row-major. Its loader applies
        // this same transpose while copying, so mirror that operation rather than treating the
        // serialized sequence as an ordinary row-major vector.
        for x in 0..width {
            for y in 0..height {
                let saved_index = x + y * width;
                let destination = y + x * width;
                let saved_data = parse::<u16>(&data[saved_index], "tile data")?;
                let (tile, adjusted_data) = tile_from_java(
                    &level[saved_index + tile_offset],
                    saved_data,
                    &world_ver,
                    index as i8,
                )?;
                tiles[destination] = tile;
                tile_data[destination] = adjusted_data;
            }
        }
        levels.push(Level {
            depth,
            tiles,
            data: tile_data,
            max_mob_count: spawn::max_mob_count(depth, difficulty),
            pending_spawns: Vec::new(),
            entities: EntityArena::default(),
        });
    }
    world.levels = levels;
    world.signs = vec![Default::default(); LEVEL_COUNT];

    import_player(directory, &mut world, &world_ver)?;
    import_inventory(directory, &mut world, &world_ver)?;
    import_entities(directory, &mut world, &world_ver)?;
    import_signs(directory, &mut world)?;
    import_unlocks(directory, &mut world)?;
    let advancements = directory.join("advancements.json");
    if advancements.exists() {
        world.progress.import_java_advancements(
            serde_json::from_str(&read_text_limited(&advancements, MAX_JAVA_FILE_BYTES)?)
                .map_err(|e| format!("invalid Java advancements: {e}"))?,
        )?;
    }

    world.pause.close();
    world.pause.begin_tick();
    world.inventory_open = false;
    world.sign_editor = None;
    world.book_open = None;
    world.crafting_station = None;
    world.notifications.clear();
    world.validate_save()?;
    Ok(world)
}

fn parse_seed(game: &[String], world_ver: &Version) -> Result<i64, String> {
    if legacy_load::has_world_seed(world_ver) {
        parse(field(game, 1, "seed")?, "world seed")
    } else {
        Ok(0)
    }
}

fn import_player(directory: &Path, world: &mut World, world_ver: &Version) -> Result<(), String> {
    let path = directory.join("Player.miniplussave");
    if !path.exists() {
        return Ok(());
    }
    let player = read_save(&path)?;
    // Minimum modern layout: x,y,spawnx,spawny,health,extra,hunger,armor,
    // armorBuffer,armorName,score,level,potions...
    let minimum = if legacy_load::has_extra_health(world_ver) {
        13
    } else if legacy_load::has_hunger(world_ver) {
        12
    } else {
        11
    };
    if player.len() < minimum {
        return Err("Java player save is truncated".to_owned());
    }
    let mut cursor = 0usize;
    macro_rules! take {
        ($name:literal) => {{
            let value = field(&player, cursor, $name)?;
            cursor += 1;
            value
        }};
    }
    world.player.x = parse(take!("player x"), "player x")?;
    world.player.y = parse(take!("player y"), "player y")?;
    let spawn_x = parse(take!("spawn x"), "spawn x")?;
    let spawn_y = parse(take!("spawn y"), "spawn y")?;
    world.respawn_tile = Some((spawn_x, spawn_y));
    world.player.direction = Direction::Down;
    world.player.health = parse::<u8>(take!("player health"), "player health")?.min(MAX_HEALTH);
    if legacy_load::has_extra_health(world_ver) {
        let extra_health = parse::<u8>(take!("extra health"), "extra health")?;
        world.player.max_health = (10_u8.saturating_add(extra_health)).min(MAX_HEALTH);
    } else {
        world.player.max_health = MAX_HEALTH;
    }
    world.player.health = world.player.health.min(world.player.max_health);
    if legacy_load::has_hunger(world_ver) {
        world.player.hunger = parse::<u8>(take!("hunger"), "hunger")?.min(MAX_STAT);
    }
    world.player.armor = parse(take!("armor durability"), "armor durability")?;
    world.player.armor_damage_buffer = parse(take!("armor damage buffer"), "armor damage buffer")?;
    let armor_name = take!("armor kind").to_owned();
    world.player.armor_kind = armor_from_java(&sub_old_name(&armor_name, world_ver));
    world.score = parse(take!("score"), "score")?;
    world.current_level = parse::<usize>(take!("current level"), "current level")?;
    if world.current_level >= world.levels.len() {
        return Err(format!(
            "invalid Java current level {}",
            world.current_level
        ));
    }
    let potions = take!("potion effects").to_owned();
    import_potions(&potions, &mut world.player.potion_effects)?;
    if world_ver.at_least(&Version::parse("2.1.0")) {
        if let Some(color) = player.get(cursor).filter(|color| !color.is_empty()) {
            let color = parse::<i32>(color, "shirt color")?;
            world.player.imported_shirt_color = Some(color);
            if let Some((item, _)) = crate::item::clothing::ALL
                .iter()
                .find(|(_, c)| *c == color as u32)
            {
                world.player.clothing = *item;
            }
        }
        if let Some(recipes) = player.get(cursor + 1).filter(|value| !value.is_empty()) {
            world.progress.import_unlocked_recipes(
                serde_json::from_str(recipes)
                    .map_err(|e| format!("invalid Java unlocked recipes: {e}"))?,
            )?;
        }
    }
    Ok(())
}

fn import_inventory(
    directory: &Path,
    world: &mut World,
    world_ver: &Version,
) -> Result<(), String> {
    let path = directory.join("Inventory.miniplussave");
    if !path.exists() {
        return Ok(());
    }
    let saved = read_save(&path)?;
    // Java Inventory.maxItem is always 27. Creative's unlimited catalogue is a
    // separate menu and does not enlarge the player's own inventory.
    let mut inventory = Inventory::new(27);
    let mut active = None;
    for (index, entry) in saved.iter().filter(|entry| !entry.is_empty()).enumerate() {
        let migrated = sub_old_name(entry, world_ver);
        if compact(&migrated).contains("powerglove") || compact(&migrated).contains("totemofwind") {
            continue;
        }
        match item_from_java(&migrated)? {
            ImportedItem::Stack(item, count) => {
                let remainder = inventory.add(item, count);
                if remainder > 0 {
                    world.levels[world.current_level].entities.spawn_item(
                        crate::item::ItemStack::new(item, remainder),
                        world.player.x,
                        world.player.y,
                    );
                }
                if index == 0 {
                    active = Some(ActiveItem::Stack(item));
                }
                if item == ItemId::WateringCan {
                    world.player.watering_content = migrated
                        .rsplit_once('_')
                        .and_then(|(_, content)| content.parse().ok())
                        .unwrap_or(0)
                        .min(1_800);
                }
            }
            ImportedItem::Tool(mut tool) => {
                tool.durability = tool.durability.min(tool.max_durability);
                if let Some(tool_index) = inventory.add_tool(tool) {
                    if index == 0 {
                        active = Some(ActiveItem::Tool(tool_index));
                    }
                } else {
                    world.levels[world.current_level].entities.spawn_tool(
                        tool,
                        world.player.x,
                        world.player.y,
                    );
                }
            }
        }
    }
    world.player.inventory = inventory;
    world.player.active_item = active;
    Ok(())
}

fn import_entities(directory: &Path, world: &mut World, world_ver: &Version) -> Result<(), String> {
    let path = directory.join("Entities.miniplussave");
    if !path.exists() {
        return Ok(());
    }
    for encoded in read_save(&path)? {
        if encoded.is_empty() || encoded.starts_with("Player") {
            continue;
        }
        let Some(open) = encoded.find('[') else {
            continue;
        };
        let Some(close) = encoded.rfind(']') else {
            continue;
        };
        let name = &encoded[..open];
        let info: Vec<&str> = encoded[open + 1..close].split(':').collect();
        if info.len() < 3 {
            continue;
        }
        let x = parse::<i32>(info[0], "entity x")?;
        let y = parse::<i32>(info[1], "entity y")?;
        let level_index = parse::<usize>(info[info.len() - 1], "entity level")?;
        let Some(level) = world.levels.get_mut(level_index) else {
            continue;
        };
        if let Some(species) = mob_from_java(name) {
            let health = info.get(2).and_then(|value| value.parse().ok());
            // Java: sheep cut flag only present from 2.0.7-dev1.
            let sheared = species == spawn::NaturalMob::Sheep
                && world_ver.at_least(&milestones::v_2_0_7_dev1())
                && info.get(3).is_some_and(|value| parse_bool(value));
            level.entities.import_mob(species, x, y, health, sheared);
            continue;
        }
        if compact(name) == "spawner" {
            if let Some(mob) = info.get(2).and_then(|value| mob_from_java(value)) {
                level
                    .entities
                    .import_furniture(spawner_for(mob), x, y, 0, &[], &[]);
            }
            continue;
        }
        if compact(name) == "deathchest" {
            if info.len() < 4 {
                return Err("truncated Java death chest".to_owned());
            }
            let remaining = parse::<u32>(info[info.len() - 2], "death chest remaining ticks")?;
            let mut stacks = Vec::new();
            let mut tools = Vec::new();
            for entry in &info[2..info.len() - 2] {
                match item_from_java(&sub_old_name(entry, world_ver))? {
                    ImportedItem::Stack(item, count) => {
                        stacks.push(crate::item::ItemStack::new(item, count))
                    }
                    ImportedItem::Tool(tool) => tools.push(tool),
                }
            }
            let id = level.entities.spawn_death_chest(x, y, stacks, tools);
            level.entities.set_death_chest_remaining(id, remaining);
            continue;
        }
        if let Some(mut kind) = furniture_from_java(name, info.get(2).copied()) {
            let mut state = 0;
            let mut contents = Vec::new();
            let mut tools = Vec::new();
            if matches!(kind, FurnitureKind::Chest | FurnitureKind::DungeonChest) {
                let last_content = if kind == FurnitureKind::DungeonChest {
                    state = info
                        .get(info.len().saturating_sub(2))
                        .is_some_and(|value| parse_bool(value)) as u16;
                    info.len().saturating_sub(2)
                } else {
                    info.len().saturating_sub(1)
                };
                for entry in info.iter().take(last_content).skip(2) {
                    let migrated = sub_old_name(entry, world_ver);
                    match item_from_java(&migrated) {
                        Ok(ImportedItem::Stack(item, count)) => contents.push((item, count)),
                        Ok(ImportedItem::Tool(tool)) => tools.push(tool),
                        Err(_) => {}
                    }
                }
            }
            if compact(name) == "lantern" {
                kind = match info.get(2).and_then(|value| value.parse::<usize>().ok()) {
                    Some(1) => FurnitureKind::IronLantern,
                    Some(2) => FurnitureKind::GoldLantern,
                    _ => FurnitureKind::Lantern,
                };
            }
            level
                .entities
                .import_furniture(kind, x, y, state, &contents, &tools);
        }
    }
    Ok(())
}

fn import_signs(directory: &Path, world: &mut World) -> Result<(), String> {
    let path = directory.join("signs.json");
    if !path.exists() {
        return Ok(());
    }
    let value: Value = serde_json::from_str(&read_text_limited(&path, MAX_JAVA_FILE_BYTES)?)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let Some(signs) = value.get("signs").and_then(Value::as_array) else {
        return Ok(());
    };
    for sign in signs {
        let level = sign
            .get("level")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX) as usize;
        let x = sign.get("x").and_then(Value::as_u64).unwrap_or(u64::MAX) as usize;
        let y = sign.get("y").and_then(Value::as_u64).unwrap_or(u64::MAX) as usize;
        let text = sign
            .get("lines")
            .and_then(Value::as_array)
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        if level < world.signs.len() && x < world.width && y < world.height {
            world.signs[level].insert(x + y * world.width, text);
        }
    }
    Ok(())
}

fn import_unlocks(directory: &Path, world: &mut World) -> Result<(), String> {
    let Some(game_dir) = directory.parent().and_then(Path::parent) else {
        return Ok(());
    };
    let path = game_dir.join("Unlocks.json");
    if !path.exists() {
        return Ok(());
    }
    let value: Value = serde_json::from_str(&read_text_limited(&path, MAX_JAVA_FILE_BYTES)?)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    if let Some(achievements) = value.get("unlockedAchievements").and_then(Value::as_array) {
        for id in achievements.iter().filter_map(Value::as_str) {
            world.progress.unlock_achievement(id);
        }
    }
    Ok(())
}

fn read_save(path: &Path) -> Result<Vec<String>, String> {
    let text = read_text_limited(path, MAX_JAVA_FILE_BYTES)?;
    split_unwrapped(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// Java `Load.splitUnwrappedCommas` — comma split that ignores commas inside
/// `{}[]()` so JSON and potion lists survive intact.
fn split_unwrapped(text: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut stack = Vec::new();
    for (index, ch) in text.char_indices() {
        match ch {
            '{' | '[' | '(' => stack.push(ch),
            '}' | ']' | ')' => {
                let expected = match ch {
                    '}' => '{',
                    ']' => '[',
                    _ => '(',
                };
                if stack.pop() != Some(expected) {
                    return Err(format!("unbalanced delimiter at byte {index}"));
                }
            }
            ',' if stack.is_empty() => {
                fields.push(text[start..index].trim().to_owned());
                start = index + 1;
            }
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err("unclosed delimiter".to_owned());
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        fields.push(tail.to_owned());
    }
    while fields.last().is_some_and(String::is_empty) {
        fields.pop();
    }
    Ok(fields)
}

/// Accept saves this importer can represent. Java itself loads far older
/// files via `LegacyLoad`; those layouts are not yet mapped into the Rust
/// `World` and still reject here with an explicit version error.
fn require_version(game: &[String]) -> Result<Version, String> {
    let raw = field(game, 0, "version")?;
    if !raw.contains('.') {
        return Err(format!(
            "legacy Java save without dotted version ({raw:?}); LegacyLoad layouts are not imported"
        ));
    }
    let version = Version::parse(raw);
    if !version.is_valid() {
        return Err(format!("invalid Java save version {raw:?}"));
    }
    // Minimum layout this importer understands (seed, mode, hunger, armor buffer).
    let minimum = milestones::v_2_0_4_dev8();
    if version.less_than(&minimum) {
        return Err(format!(
            "unsupported Java save version {version}; minimum importable is {minimum}"
        ));
    }
    // Cap at the next major: unknown future layouts must not silently import.
    let future = Version::parse("2.4.0");
    if !version.less_than(&future) {
        return Err(format!(
            "unsupported Java save version {version}; maximum importable is 2.3.x"
        ));
    }
    Ok(version)
}

fn parse_mode(text: &str, world_ver: &Version) -> Result<(GameMode, i32, usize), String> {
    let fields: Vec<&str> = text.split(';').collect();
    let raw_mode = parse::<i32>(fields[0], "game mode")?;
    let mode =
        GameMode::from_index(legacy_load::adjust_mode_index(raw_mode, world_ver).max(0) as usize);
    let score_ticks = fields
        .get(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let score_minutes = fields
        .get(2)
        .and_then(|value| value.parse().ok())
        .filter(|value| matches!(value, 10 | 20 | 40 | 60 | 120))
        .unwrap_or(20);
    Ok((mode, score_ticks, score_minutes))
}

fn tile_from_java(
    name: &str,
    data: u16,
    world_ver: &Version,
    depth: i8,
) -> Result<(Tile, u16), String> {
    if let Ok(id) = name.trim().parse::<u16>() {
        return Tile::from_legacy_id(id).ok_or_else(|| format!("unknown legacy tile id {id}"));
    }
    let mut key = compact(name);
    // Java loadWorld wool / lapis / cloud-cactus migrations.
    if key == "wool" {
        if world_ver.less_than(&milestones::v_2_0_6_dev4()) {
            key = match data {
                1 => "redwool".to_owned(),
                2 => "yellowwool".to_owned(),
                3 => "greenwool".to_owned(),
                4 => "bluewool".to_owned(),
                5 => "blackwool".to_owned(),
                _ => "whitewool".to_owned(),
            };
        } else if world_ver.less_than(&Version::parse("2.3.0-dev1")) {
            key = "whitewool".to_owned();
        }
    } else if depth == 1 && key == "lapis" && world_ver.less_than(&Version::parse("2.0.3-dev6")) {
        // Java randomly converts most surface lapis to gem ore on pre-2.0.3-dev6.
        key = "gemore".to_owned();
    }
    if key == "torch" {
        // Modern Java stores the underlying stable tile ID in the data field.
        let base = u8::try_from(data)
            .ok()
            .and_then(Tile::from_id)
            .filter(|tile| *tile != Tile::Torch)
            .ok_or_else(|| format!("invalid Java torch base {data}"))?;
        return Ok((Tile::Torch, base.id() as u16));
    }
    if let Some(underlying) = key.strip_prefix("torch") {
        let base = tile_by_key(underlying)
            .ok_or_else(|| format!("unknown Java torch base {underlying:?}"))?;
        return Ok((Tile::Torch, base.id() as u16));
    }
    tile_by_key(&key)
        .map(|tile| (tile, data))
        .ok_or_else(|| format!("unknown Java tile {name:?}"))
}

fn tile_by_key(key: &str) -> Option<Tile> {
    Some(match key {
        "grass" => Tile::Grass,
        "dirt" => Tile::Dirt,
        "flower" | "rose" => Tile::Flower,
        "hole" => Tile::Hole,
        "stairsup" => Tile::StairsUp,
        "stairsdown" => Tile::StairsDown,
        "water" => Tile::Water,
        "rock" => Tile::Rock,
        "tree" | "oak" => Tile::Tree,
        "treesapling" | "sapling" => Tile::TreeSapling,
        "sand" => Tile::Sand,
        "cactus" => Tile::Cactus,
        "cactussapling" => Tile::CactusSapling,
        "ironore" => Tile::IronOre,
        "goldore" => Tile::GoldOre,
        "gemore" => Tile::GemOre,
        "lapis" | "lapisore" => Tile::LapisOre,
        "lava" => Tile::Lava,
        "lavabrick" => Tile::LavaBrick,
        "explode" | "exploded" => Tile::Exploded,
        "farmland" => Tile::Farmland,
        "wheat" => Tile::Wheat,
        "hardrock" => Tile::HardRock,
        "infinitefall" => Tile::InfiniteFall,
        "cloud" => Tile::Cloud,
        "cloudore" | "cloudcactus" => Tile::CloudOre,
        "wooddoor" => Tile::WoodDoor,
        "stonedoor" => Tile::StoneDoor,
        "obsidiandoor" => Tile::ObsidianDoor,
        "woodfloor" | "woodplanks" | "planks" => Tile::WoodFloor,
        "stonefloor" | "stonebricks" => Tile::StoneFloor,
        "obsidian" | "obsidianfloor" | "obsidianbricks" => Tile::ObsidianFloor,
        "woodwall" => Tile::WoodWall,
        "stonewall" => Tile::StoneWall,
        "obsidianwall" => Tile::ObsidianWall,
        "wool" | "whitewool" => Tile::WhiteWool,
        "path" => Tile::Path,
        "redwool" => Tile::RedWool,
        "bluewool" => Tile::BlueWool,
        "greenwool" => Tile::GreenWool,
        "yellowwool" => Tile::YellowWool,
        "blackwool" => Tile::BlackWool,
        "potato" => Tile::Potato,
        "stone" | "rawstone" => Tile::RawStone,
        "rawobsidian" => Tile::RawObsidian,
        "ornatestone" => Tile::OrnateStone,
        "ornateobsidian" => Tile::OrnateObsidian,
        "obsidianbosswall" | "bosswall" => Tile::BossWall,
        "obsidianbossfloor" | "bossfloor" => Tile::BossFloor,
        "obsidianbossdoor" | "bossdoor" => Tile::BossDoor,
        "tomato" => Tile::Tomato,
        "carrot" => Tile::Carrot,
        "heavenlyberries" => Tile::HeavenlyBerries,
        "hellishberries" => Tile::HellishBerries,
        "woodfence" => Tile::WoodFence,
        "stonefence" => Tile::StoneFence,
        "obsidianfence" => Tile::ObsidianFence,
        "torch" => Tile::Torch,
        "sign" => Tile::Sign,
        _ => return None,
    })
}

enum ImportedItem {
    Stack(ItemId, u16),
    Tool(ToolItem),
}

fn item_from_java(encoded: &str) -> Result<ImportedItem, String> {
    let (name, value) = encoded
        .rsplit_once('_')
        .map_or((encoded, 1), |(name, value)| {
            (name, value.parse().unwrap_or(1))
        });
    let key = compact(name);
    if let Some(kind) = tool_kind(&key) {
        let tier = tool_tier(&key).unwrap_or(ToolTier::Wood);
        let mut tool = ToolItem::new(kind, tier);
        tool.durability = value;
        return Ok(ImportedItem::Tool(tool));
    }
    let item = ItemId::ALL
        .iter()
        .copied()
        .find(|item| compact(item.display_name()) == key)
        .or_else(|| item_alias(&key))
        .ok_or_else(|| format!("unknown Java item {name:?}"))?;
    let count = if item == ItemId::WateringCan {
        1
    } else {
        value.max(1)
    };
    Ok(ImportedItem::Stack(item, count))
}

fn item_alias(key: &str) -> Option<ItemId> {
    Some(match key {
        "iron" => ItemId::IronIngot,
        "gold" => ItemId::GoldIngot,
        "goldapple" => ItemId::GoldenApple,
        "regclothes" | "regularclothing" => ItemId::RegularClothes,
        "totemofwind" | "totemofair" => ItemId::AirTotem,
        "obsidian" => ItemId::RawObsidian,
        "seed" => ItemId::WheatSeeds,
        "whitewool" => ItemId::Wool,
        _ => return None,
    })
}

fn tool_kind(key: &str) -> Option<ToolKind> {
    ToolKind::ALL
        .into_iter()
        .find(|kind| key.ends_with(&compact(kind.display_name())))
}

fn tool_tier(key: &str) -> Option<ToolTier> {
    ToolTier::ALL
        .into_iter()
        .find(|tier| key.starts_with(&compact(tier.display_name())))
}

fn armor_from_java(name: &str) -> Option<ArmorKind> {
    let key = compact(name);
    if key == "null" || key.is_empty() {
        return None;
    }
    [
        ArmorKind::Leather,
        ArmorKind::Snake,
        ArmorKind::Iron,
        ArmorKind::Gold,
        ArmorKind::Gem,
    ]
    .into_iter()
    .find(|armor| key.starts_with(&compact(armor.display_name())))
}

fn import_potions(text: &str, effects: &mut [u16; PotionKind::ALL.len()]) -> Result<(), String> {
    effects.fill(0);
    let inner = text
        .strip_prefix("PotionEffects[")
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("invalid Java potion list {text:?}"))?;
    if inner.is_empty() {
        return Ok(());
    }
    for effect in inner.split(':') {
        let (name, duration) = effect
            .split_once(';')
            .ok_or_else(|| format!("invalid Java potion effect {effect:?}"))?;
        if let Some(kind) = PotionKind::ALL
            .into_iter()
            .find(|kind| compact(kind.display_name()) == compact(name))
        {
            effects[kind.id()] = parse(duration, "potion duration")?;
        }
    }
    Ok(())
}

fn mob_from_java(name: &str) -> Option<spawn::NaturalMob> {
    Some(match compact(name).as_str() {
        "slime" => spawn::NaturalMob::Slime,
        "zombie" => spawn::NaturalMob::Zombie,
        "creeper" => spawn::NaturalMob::Creeper,
        "skeleton" => spawn::NaturalMob::Skeleton,
        "snake" => spawn::NaturalMob::Snake,
        "knight" => spawn::NaturalMob::Knight,
        "cow" => spawn::NaturalMob::Cow,
        "pig" => spawn::NaturalMob::Pig,
        "sheep" => spawn::NaturalMob::Sheep,
        "airwizard" => spawn::NaturalMob::AirWizard,
        "obsidianknight" => spawn::NaturalMob::ObsidianKnight,
        _ => return None,
    })
}

fn furniture_from_java(name: &str, _extra: Option<&str>) -> Option<FurnitureKind> {
    Some(match compact(name).as_str() {
        "workbench" => FurnitureKind::Workbench,
        "oven" => FurnitureKind::Oven,
        "furnace" => FurnitureKind::Furnace,
        "anvil" => FurnitureKind::Anvil,
        "enchanter" => FurnitureKind::Enchanter,
        "loom" => FurnitureKind::Loom,
        "chest" => FurnitureKind::Chest,
        "deathchest" => FurnitureKind::Chest,
        "dungeonchest" => FurnitureKind::DungeonChest,
        "lantern" => FurnitureKind::Lantern,
        "tnt" => FurnitureKind::Tnt,
        "bed" => FurnitureKind::Bed,
        "composter" => FurnitureKind::Composter,
        "knightstatue" => FurnitureKind::KnightStatue,
        _ => return None,
    })
}

fn spawner_for(mob: spawn::NaturalMob) -> FurnitureKind {
    match mob {
        spawn::NaturalMob::Cow => FurnitureKind::CowSpawner,
        spawn::NaturalMob::Pig => FurnitureKind::PigSpawner,
        spawn::NaturalMob::Sheep => FurnitureKind::SheepSpawner,
        spawn::NaturalMob::Slime => FurnitureKind::SlimeSpawner,
        spawn::NaturalMob::Zombie => FurnitureKind::ZombieSpawner,
        spawn::NaturalMob::Creeper => FurnitureKind::CreeperSpawner,
        spawn::NaturalMob::Skeleton => FurnitureKind::SkeletonSpawner,
        spawn::NaturalMob::Snake => FurnitureKind::SnakeSpawner,
        _ => FurnitureKind::KnightSpawner,
    }
}

fn compact(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn field<'a>(fields: &'a [String], index: usize, name: &str) -> Result<&'a str, String> {
    fields
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("Java save is missing {name}"))
}

fn parse<T>(text: &str, name: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    text.parse()
        .map_err(|_| format!("invalid {name}: {text:?}"))
}

fn parse_bool(text: &str) -> bool {
    text.eq_ignore_ascii_case("true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_2_2_4_fixture_imports_complete_core_state() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-java-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let directory = root.join("saves").join("fixture");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("Game.miniplussave"),
            "2.2.4,42,3;600;20,123,65000,1,true,true,true,true,",
        )
        .unwrap();
        let tiles = "Grass,Dirt,Stairs Down,Water";
        for (index, depth) in [1, 0, -1, -2, -3, -4].into_iter().enumerate() {
            std::fs::write(
                directory.join(format!("Level{index}.miniplussave")),
                format!("2,2,42,{depth},{tiles},"),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("Level{index}data.miniplussave")),
                "0,0,0,0,",
            )
            .unwrap();
        }
        std::fs::write(
            directory.join("Player.miniplussave"),
            "8,8,8,8,7,5,6,9,2,Iron Armor,99,1,PotionEffects[Speed;120],0,{},",
        )
        .unwrap();
        std::fs::write(
            directory.join("Inventory.miniplussave"),
            "Wood_3,Iron Pickaxe_70,Gem_2,",
        )
        .unwrap();
        std::fs::write(
            directory.join("Entities.miniplussave"),
            "Cow[8:8:4:1],Workbench[24:8:0],Bed[8:24:0],",
        )
        .unwrap();
        std::fs::write(
            directory.join("signs.json"),
            r#"{"Version":"2.2.4","signs":[{"level":1,"x":0,"y":1,"lines":["HELLO","RUST"]}]}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("Unlocks.json"),
            r#"{"unlockedAchievements":["minicraft.achievement.skin"]}"#,
        )
        .unwrap();

        let probe = probe_java_save(&directory).unwrap_err();
        assert!(probe.contains("unsupported Java world dimensions 2x2"));
        let world = import_java_save(&directory).unwrap();
        assert_eq!((world.width, world.height, world.seed), (2, 2, 42));
        assert_eq!((world.mode, world.score_ticks), (GameMode::Score, 600));
        assert_eq!(
            (
                world.clock.get().game_time,
                world.tick,
                world.clock.get().day_tick
            ),
            (65_000, 0, 123)
        );
        assert_eq!((world.player.health, world.player.max_health), (7, 15));
        assert_eq!(world.player.inventory.count(ItemId::Wood), 3);
        assert_eq!(world.player.inventory.tools()[0].durability, 70);
        assert_eq!(world.player.potion_effects[PotionKind::Speed.id()], 120);
        assert_eq!(world.levels[1].entities.entities().len(), 1);
        assert_eq!(world.signs[1].get(&2).unwrap(), "HELLO RUST");
        assert!(
            world
                .progress
                .achievement_unlocked("minicraft.achievement.skin")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unwrapped_split_preserves_json_and_effect_lists() {
        assert_eq!(
            split_unwrapped("a,PotionEffects[Speed;20:Light;10],{\"x\":[1,2]},").unwrap(),
            ["a", "PotionEffects[Speed;20:Light;10]", "{\"x\":[1,2]}"]
        );
    }

    #[test]
    fn oversized_java_save_is_rejected_before_parsing() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-java-oversized-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::File::create(root.join("Game.miniplussave"))
            .unwrap()
            .set_len(MAX_JAVA_FILE_BYTES + 1)
            .unwrap();
        let error = probe_java_save(&root).unwrap_err();
        assert!(error.contains("read limit"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn version_gate_rejects_pre_2_0_4_layouts() {
        let game = vec!["1.9.2".to_owned()];
        let error = require_version(&game).unwrap_err();
        assert!(error.contains("unsupported Java save version"), "{error}");
        let modern = require_version(&[JAVA_VERSION.to_owned()]).unwrap();
        assert_eq!(modern.to_string(), JAVA_VERSION);
    }

    #[test]
    fn seed_and_quest_flags_follow_version_gates() {
        let old = Version::parse("2.0.5");
        let modern = milestones::v_2_2_4();
        let game_old = vec![
            "2.0.5".to_owned(),
            "99".to_owned(),
            "0".to_owned(),
            "1".to_owned(),
            "100".to_owned(),
            "0".to_owned(),
            "false".to_owned(),
        ];
        assert_eq!(parse_seed(&game_old, &old).unwrap(), 0);
        assert!(!legacy_load::has_quest_flags(&old));
        let game_new = vec![
            "2.2.4".to_owned(),
            "42".to_owned(),
            "0".to_owned(),
            "1".to_owned(),
            "100".to_owned(),
            "1".to_owned(),
            "true".to_owned(),
            "true".to_owned(),
            "false".to_owned(),
        ];
        assert_eq!(parse_seed(&game_new, &modern).unwrap(), 42);
        assert!(legacy_load::has_quest_flags(&modern));
    }

    #[test]
    fn save_game_fields_round_trip_through_load_gates() {
        use crate::world::{PlayOptions, WorldSpec};
        let mut world = World::new_with_play_options(
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
        );
        world.score_ticks = 600;
        world.air_wizard_defeated = true;
        let fields = crate::saveload::save::game_fields(&world);
        let joined = crate::saveload::save::join_world_save_fields(&fields);
        let parsed = split_unwrapped(&joined).unwrap();
        let version = require_version(&parsed).unwrap();
        assert_eq!(version.to_string(), JAVA_VERSION);
        assert_eq!(parse_seed(&parsed, &version).unwrap(), 42);
        let (mode, ticks, minutes) =
            parse_mode(field(&parsed, 2, "mode").unwrap(), &version).unwrap();
        assert_eq!(mode, GameMode::Score);
        assert_eq!((ticks, minutes), (600, 20));
        assert!(parse_bool(field(&parsed, 6, "Air Wizard state").unwrap()));
    }
}

#[cfg(test)]
mod java224_fixture_tests {
    use super::*;
    use crate::saveload::save;
    use crate::world::registry_tests::tiny_world;

    #[test]
    fn java224_all_registered_tiles_and_torch_base_import() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/java224/tiles.json")).unwrap();
        let version = Version::parse("2.2.4");
        for row in fixture["tiles"].as_array().unwrap() {
            let data = row["data"].as_u64().unwrap() as u16;
            let (tile, restored) =
                tile_from_java(row["name"].as_str().unwrap(), data, &version, 0).unwrap();
            assert_eq!(tile.id() as u64, row["id"].as_u64().unwrap(), "{row}");
            assert_eq!(restored, data, "{row}");
        }
        let torch = &fixture["torch"];
        assert_eq!(
            tile_from_java(
                torch["name"].as_str().unwrap(),
                torch["data"].as_u64().unwrap() as u16,
                &version,
                0
            )
            .unwrap(),
            (Tile::Torch, Tile::ObsidianFloor.id() as u16)
        );
        assert!(tile_from_java("Torch", 65535, &version, 0).is_err());
        assert_eq!(
            tile_from_java("Torch Wood Planks", 0, &version, 0).unwrap(),
            (Tile::Torch, Tile::WoodFloor.id() as u16)
        );
    }

    #[test]
    fn java224_actual_writer_fields_and_progress_survive_native_round_trip() {
        let mut world = tiny_world();
        while world.levels.len() < 6 {
            world.levels.push(crate::world::Level {
                depth: 0,
                tiles: vec![Tile::Grass; 256],
                data: vec![0; 256],
                max_mob_count: 0,
                pending_spawns: vec![],
                entities: EntityArena::default(),
            });
        }
        world.signs = vec![Default::default(); 6];
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/java224");
        import_player(&directory, &mut world, &Version::parse("2.2.4")).unwrap();
        import_inventory(&directory, &mut world, &Version::parse("2.2.4")).unwrap();
        import_entities(&directory, &mut world, &Version::parse("2.2.4")).unwrap();
        let chest = &world.levels[3].entities.entities()[0];
        assert_eq!(
            chest.state,
            crate::entity::furniture::chest::DEATH_CHEST_STATE
        );
        assert_eq!(chest.death_chest_lifetime, Some(123));
        let storage = world.levels[3]
            .entities
            .death_chest_storage(chest.id)
            .unwrap();
        assert_eq!(storage.count(ItemId::Wood), 9);
        assert_eq!(storage.tools()[0].durability, 17);
        let progress: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/java224/advancements.json"
        ))
        .unwrap();
        world
            .progress
            .import_java_advancements(progress.clone())
            .unwrap();
        assert_eq!(world.respawn_tile, Some((2, 3)));
        assert_eq!(world.player.clothing, ItemId::BlueClothes);
        assert_eq!(world.player.watering_content, 731);
        assert_eq!(world.player.inventory.count(ItemId::WateringCan), 1);
        assert_eq!(world.player.inventory.count(ItemId::Wood), 9);
        assert_eq!(world.player.inventory.tools()[0].durability, 17);
        assert_eq!(
            world.progress.current_tutorial().unwrap().id,
            "minicraft.tutorial.getting_rocks"
        );
        assert_eq!(world.progress.tutorial_completed_count(), 1);
        assert_eq!(world.progress.quest_completed_count(), 1);
        assert_eq!(world.progress.java_advancements_json(), progress);
        let original_player = save::player_fields(&world);
        assert_eq!(
            save::join_world_save_fields(&original_player),
            std::fs::read_to_string(directory.join("Player.miniplussave")).unwrap()
        );
        assert_eq!(save::player_fields(&world), original_player);
        // Rust stores stack slots and tool slots separately, so serialized order
        // can differ. Compare exact instances/counts instead of a bag ordering.
        let mut expected = read_save(&directory.join("Inventory.miniplussave")).unwrap();
        expected.sort();
        let mut actual: Vec<_> = save::inventory_fields(&world)
            .into_iter()
            .filter(|entry| !entry.is_empty())
            .collect();
        actual.sort();
        assert_eq!(actual, expected);
        world.score_minutes = 40;
        world.mode = GameMode::Score;
        world.score_ticks = 123;
        assert_eq!(save::game_fields(&world)[2], "3;123;40");
        let mut restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        assert_eq!(save::player_fields(&restored), original_player);
        assert_eq!(restored.progress.java_advancements_json(), progress);
        assert_eq!(restored.score_minutes, 40);
        let update = restored.progress.update(
            &crate::progression::ProgressEvent::InventoryChanged,
            &std::collections::HashMap::from([("coal".to_owned(), 5)]),
        );
        assert_eq!(
            update.completed_tutorials,
            ["minicraft.tutorial.getting_rocks"]
        );
        assert!(
            update.rewards.is_empty(),
            "loading completed quests must not pay rewards again"
        );
        let exported = restored.progress.java_advancements_json();
        assert_eq!(exported["minicraft.tutorial.getting_rocks"]["done"], true);
        assert_eq!(
            exported["minicraft.tutorial.getting_rocks"]["criteria"]["has_rock"],
            "2024-06-02T01:02:03"
        );
        assert!(exported["minicraft.tutorial.getting_rocks"]["criteria"]["has_coal"].is_string());
        // The skipped intervening tutorials remain incomplete.
        assert_eq!(restored.progress.tutorial_completed_count(), 2);
    }
}

#[cfg(test)]
mod java230_world_import_tests {
    use super::*;

    #[test]
    fn java230_world_import_rejects_changed_layout_without_creating_native_save() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parity/java230/rejected-java-world-test");
        std::fs::create_dir_all(&directory).unwrap();
        let game = directory.join("Game.miniplussave");
        let original = "2.3.0-dev2,42,0,0,0,1,false,true,true,false,";
        std::fs::write(&game, original).unwrap();
        let error = match import_java_save(&directory) {
            Ok(_) => panic!("new Java layout must not be interpreted as 2.2.4"),
            Err(error) => error,
        };
        assert!(error.contains("Java 2.3 world import is not supported"));
        assert_eq!(std::fs::read_to_string(&game).unwrap(), original);
        assert!(!directory.join("state.json").exists());
        std::fs::remove_file(game).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
