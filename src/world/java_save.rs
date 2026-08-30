//! Importer for the comma-delimited local save format written by Minicraft+ 2.2.4.
//!
//! The imported state is immediately represented by the native Rust `World`; callers can then
//! persist it as the versioned `state.json` snapshot. Runtime play never reads across projects.

use std::{fs, path::Path};

use serde_json::Value;

use crate::item::{ArmorKind, Inventory, ItemId, ToolItem, ToolKind, ToolTier};

use super::{
    ActiveItem, Direction, FurnitureKind, GameMode, Level, PlayOptions, PotionKind, Tile, World,
    WorldSpec, entity::EntityArena, spawn,
};

const JAVA_VERSION: &str = "2.2.4";
const LEVEL_COUNT: usize = 6;
const MAX_JAVA_FILE_BYTES: u64 = 64 * 1024 * 1024;

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
    require_version(&game)?;
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
    let (mode, _, score_minutes) = parse_mode(field(&game, 2, "mode")?)?;
    Ok(Some(JavaSaveInfo {
        seed: parse(field(&game, 1, "seed")?, "world seed")?,
        size: width,
        mode,
        score_minutes,
    }))
}

pub fn import_java_save(directory: &Path) -> Result<World, String> {
    let game = read_save(&directory.join("Game.miniplussave"))?;
    require_version(&game)?;
    let seed = parse(field(&game, 1, "seed")?, "world seed")?;
    let (mode, score_ticks, score_minutes) = parse_mode(field(&game, 2, "mode")?)?;
    let difficulty = parse::<usize>(field(&game, 5, "difficulty")?, "difficulty")?.min(2);

    let first_level = read_save(&directory.join("Level0.miniplussave"))?;
    let width = parse::<usize>(field(&first_level, 0, "level width")?, "level width")?;
    let height = parse::<usize>(field(&first_level, 1, "level height")?, "level height")?;
    if width != height || width == 0 || width > 512 {
        return Err(format!(
            "unsupported Java world dimensions {width}x{height}"
        ));
    }

    let mut world = World::new_with_play_options(
        seed,
        WorldSpec::new(width, 0, 0),
        PlayOptions {
            difficulty,
            mode,
            score_minutes,
            tutorials: parse_bool(field(&game, 8, "tutorials")?),
            quests: parse_bool(field(&game, 7, "quests")?),
            show_quests: true,
            custom_skin: false,
        },
    );
    world.width = width;
    world.height = height;
    world.seed = seed;
    let day_tick = parse::<u64>(field(&game, 3, "tick count")?, "tick count")?;
    let game_time = parse::<u64>(field(&game, 4, "game time")?, "game time")?;
    world.tick = game_time;
    world.day_tick = (day_tick % super::DAY_LENGTH as u64) as u32;
    world.days = (game_time / super::DAY_LENGTH as u64) as u32 + 1;
    world.difficulty = difficulty;
    world.mode = mode;
    world.score_ticks = score_ticks;
    world.air_wizard_defeated = parse_bool(field(&game, 6, "Air Wizard state")?);
    world.obsidian_knight_defeated = game.get(9).is_some_and(|value| parse_bool(value));
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
        if level.len() < expected + 4 || data.len() < expected {
            return Err(format!(
                "Java level {index} is truncated (tiles {}, data {}, expected {expected})",
                level.len().saturating_sub(4),
                data.len()
            ));
        }
        let level_width = parse::<usize>(field(&level, 0, "level width")?, "level width")?;
        let level_height = parse::<usize>(field(&level, 1, "level height")?, "level height")?;
        if level_width != width || level_height != height {
            return Err(format!("Java level {index} dimensions do not match Level0"));
        }
        let depth = parse::<i8>(field(&level, 3, "level depth")?, "level depth")?;
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
                let (tile, adjusted_data) = tile_from_java(&level[saved_index + 4], saved_data)?;
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

    import_inventory(directory, &mut world)?;
    import_player(directory, &mut world)?;
    import_entities(directory, &mut world)?;
    import_signs(directory, &mut world)?;
    import_unlocks(directory, &mut world)?;

    world.paused = false;
    world.inventory_open = false;
    world.sign_editor = None;
    world.book_open = None;
    world.crafting_station = None;
    world.notification = None;
    world.validate_save()?;
    Ok(world)
}

fn import_player(directory: &Path, world: &mut World) -> Result<(), String> {
    let path = directory.join("Player.miniplussave");
    if !path.exists() {
        return Ok(());
    }
    let player = read_save(&path)?;
    if player.len() < 13 {
        return Err("Java player save is truncated".to_owned());
    }
    world.player.x = parse(&player[0], "player x")?;
    world.player.y = parse(&player[1], "player y")?;
    world.player.direction = Direction::Down;
    world.player.health = parse::<u8>(&player[4], "player health")?.min(super::MAX_HEALTH);
    let extra_health = parse::<u8>(&player[5], "extra health")?;
    world.player.max_health = (10_u8.saturating_add(extra_health)).min(super::MAX_HEALTH);
    world.player.health = world.player.health.min(world.player.max_health);
    world.player.hunger = parse::<u8>(&player[6], "hunger")?.min(super::MAX_STAT);
    world.player.armor = parse(&player[7], "armor durability")?;
    world.player.armor_damage_buffer = parse(&player[8], "armor damage buffer")?;
    world.player.armor_kind = armor_from_java(&player[9]);
    world.score = parse(&player[10], "score")?;
    world.current_level = parse::<usize>(&player[11], "current level")?;
    if world.current_level >= world.levels.len() {
        return Err(format!(
            "invalid Java current level {}",
            world.current_level
        ));
    }
    import_potions(&player[12], &mut world.player.potion_effects)?;
    Ok(())
}

fn import_inventory(directory: &Path, world: &mut World) -> Result<(), String> {
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
        if compact(entry).contains("powerglove") || compact(entry).contains("totemofwind") {
            continue;
        }
        match item_from_java(entry)? {
            ImportedItem::Stack(item, count) => {
                inventory.add(item, count);
                if index == 0 {
                    active = Some(ActiveItem::Stack(item));
                }
                if item == ItemId::WateringCan {
                    world.player.watering_content = entry
                        .rsplit_once('_')
                        .and_then(|(_, content)| content.parse().ok())
                        .unwrap_or(0)
                        .min(1_800);
                }
            }
            ImportedItem::Tool(mut tool) => {
                tool.durability = tool.durability.min(tool.max_durability);
                let tool_index = inventory
                    .add_tool(tool)
                    .ok_or_else(|| "Java inventory exceeds Rust inventory capacity".to_owned())?;
                if index == 0 {
                    active = Some(ActiveItem::Tool(tool_index));
                }
            }
        }
    }
    world.player.inventory = inventory;
    world.player.active_item = active;
    Ok(())
}

fn import_entities(directory: &Path, world: &mut World) -> Result<(), String> {
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
            let sheared = species == spawn::NaturalMob::Sheep
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
                    match item_from_java(entry) {
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
    let value: Value = serde_json::from_str(
        &fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?,
    )
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
    let value: Value = serde_json::from_str(
        &fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    if let Some(achievements) = value.get("unlockedAchievements").and_then(Value::as_array) {
        for id in achievements.iter().filter_map(Value::as_str) {
            world.progress.unlock_achievement(id);
        }
    }
    Ok(())
}

fn read_save(path: &Path) -> Result<Vec<String>, String> {
    let size = fs::metadata(path)
        .map_err(|error| format!("cannot inspect Java save {}: {error}", path.display()))?
        .len();
    if size > MAX_JAVA_FILE_BYTES {
        return Err(format!("Java save {} exceeds 64 MiB", path.display()));
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read Java save {}: {error}", path.display()))?;
    split_unwrapped(&text).map_err(|error| format!("{}: {error}", path.display()))
}

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

fn require_version(game: &[String]) -> Result<(), String> {
    let version = field(game, 0, "version")?;
    if version != JAVA_VERSION {
        return Err(format!(
            "unsupported Java save version {version}; expected {JAVA_VERSION}"
        ));
    }
    Ok(())
}

fn parse_mode(text: &str) -> Result<(GameMode, u32, usize), String> {
    let fields: Vec<&str> = text.split(';').collect();
    let mode = GameMode::from_index(parse::<usize>(fields[0], "game mode")?);
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

fn tile_from_java(name: &str, data: u16) -> Result<(Tile, u16), String> {
    if let Ok(id) = name.trim().parse::<u16>() {
        return Tile::from_legacy_id(id).ok_or_else(|| format!("unknown legacy tile id {id}"));
    }
    let key = compact(name);
    if let Some(underlying) = key.strip_prefix("torch") {
        let base = tile_by_key(underlying).unwrap_or(Tile::Grass);
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
        "cloudore" => Tile::CloudOre,
        "wooddoor" => Tile::WoodDoor,
        "stonedoor" => Tile::StoneDoor,
        "obsidiandoor" => Tile::ObsidianDoor,
        "woodfloor" | "woodplanks" | "planks" => Tile::WoodFloor,
        "stonefloor" | "stonebricks" => Tile::StoneFloor,
        "obsidianfloor" | "obsidianbricks" => Tile::ObsidianFloor,
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
        "rawstone" => Tile::RawStone,
        "rawobsidian" => Tile::RawObsidian,
        "ornatestone" => Tile::OrnateStone,
        "ornateobsidian" => Tile::OrnateObsidian,
        "bosswall" => Tile::BossWall,
        "bossfloor" => Tile::BossFloor,
        "bossdoor" => Tile::BossDoor,
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
        assert!(error.contains("exceeds 64 MiB"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
