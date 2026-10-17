//! Strict dev2 profile. No fallbacks, skipped entities or item-state clamping.
use super::*;
use crate::item::dye::DyeColor;

const DEPTHS: [i8; 6] = [-3, -2, -1, 0, 1, -4];

pub(super) fn tile(name: &str, data: u16) -> Result<(Tile, u16), String> {
    // Dev2 saves names. Numeric tokens belong to pre-1.9.4 saves and must not
    // be mistaken for a Rust ID (notably Java59 vs Rust59, Java69 vs Rust69).
    if name.parse::<u16>().is_ok() {
        return Err(format!(
            "numeric tile ID {name} is not a dev2 saved tile name"
        ));
    }
    const NAMES: [&str; 70] = [
        "GRASS",
        "DIRT",
        "FLOWER",
        "HOLE",
        "STAIRS UP",
        "STAIRS DOWN",
        "WATER",
        "ROCK",
        "TREE",
        "TREE SAPLING",
        "SAND",
        "CACTUS",
        "CACTUS SAPLING",
        "IRON ORE",
        "GOLD ORE",
        "GEM ORE",
        "LAPIS",
        "LAVA",
        "LAVA BRICK",
        "EXPLODE",
        "FARMLAND",
        "WHEAT",
        "HARD ROCK",
        "INFINITE FALL",
        "CLOUD",
        "CLOUD CACTUS",
        "WOOD DOOR",
        "STONE DOOR",
        "OBSIDIAN DOOR",
        "WOOD PLANKS",
        "STONE BRICKS",
        "OBSIDIAN",
        "WOOD WALL",
        "STONE WALL",
        "OBSIDIAN WALL",
        "WHITE WOOL",
        "PATH",
        "RED WOOL",
        "BLUE WOOL",
        "GREEN WOOL",
        "YELLOW WOOL",
        "BLACK WOOL",
        "POTATO",
        "STONE",
        "RAW OBSIDIAN",
        "ORNATE STONE",
        "ORNATE OBSIDIAN",
        "OBSIDIAN BOSS WALL",
        "OBSIDIAN BOSS FLOOR",
        "OBSIDIAN BOSS DOOR",
        "TOMATO",
        "CARROT",
        "HEAVENLY BERRIES",
        "HELLISH BERRIES",
        "WOOD FENCE",
        "STONE FENCE",
        "OBSIDIAN FENCE",
        "TORCH",
        "SIGN",
        "ORNATE WOOD",
        "BROWN WOOL",
        "PURPLE WOOL",
        "CYAN WOOL",
        "LIGHT GRAY WOOL",
        "GRAY WOOL",
        "PINK WOOL",
        "LIME WOOL",
        "LIGHT BLUE WOOL",
        "MAGENTA WOOL",
        "ORANGE WOOL",
    ];
    if let Some(underlying) = name.to_uppercase().strip_prefix("TORCH ") {
        let base = tile(underlying, 0)?.0;
        return overlay(Tile::Torch, base);
    }
    let id = NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("unknown dev2 tile {name:?}"))?;
    let mapped = java_id(id as u16)?;
    if mapped == Tile::Flower230 {
        if data >= 19 {
            return Err(format!("invalid dev2 flower ordinal {data}"));
        }
        return Ok((mapped, data));
    }
    if matches!(
        mapped,
        Tile::Torch | Tile::Sign | Tile::WoodFence | Tile::StoneFence | Tile::ObsidianFence
    ) {
        return overlay(mapped, java_id(data)?);
    }
    Ok((mapped, data))
}

// Explicit source-version registry: only 0..58 are unchanged numeric meanings;
// dev2 inserts Ornate Wood at59 and registers the ten extra wool colors60..69.
fn overlay(tile: Tile, base: Tile) -> Result<(Tile, u16), String> {
    if matches!(
        base,
        Tile::Torch
            | Tile::Flower230
            | Tile::Sign
            | Tile::WoodFence
            | Tile::StoneFence
            | Tile::ObsidianFence
            | Tile::WoodDoor
            | Tile::StoneDoor
            | Tile::ObsidianDoor
            | Tile::BossDoor
            | Tile::Wheat
            | Tile::Potato
            | Tile::Tomato
            | Tile::Carrot
            | Tile::HeavenlyBerries
            | Tile::HellishBerries
            | Tile::TreeSapling
            | Tile::CactusSapling
            | Tile::Farmland
    ) {
        return Err(format!("unrepresentable dev2 {tile:?} underlay {base:?}"));
    }
    Ok((tile, u16::from(base.id())))
}

fn java_id(id: u16) -> Result<Tile, String> {
    match id {
        0 => Ok(Tile::Grass),
        1 => Ok(Tile::Dirt),
        2 => Ok(Tile::Flower230),
        3 => Ok(Tile::Hole),
        4 => Ok(Tile::StairsUp),
        5 => Ok(Tile::StairsDown),
        6 => Ok(Tile::Water),
        7 => Ok(Tile::Rock),
        8 => Ok(Tile::Tree),
        9 => Ok(Tile::TreeSapling),
        10 => Ok(Tile::Sand),
        11 => Ok(Tile::Cactus),
        12 => Ok(Tile::CactusSapling),
        13 => Ok(Tile::IronOre),
        14 => Ok(Tile::GoldOre),
        15 => Ok(Tile::GemOre),
        16 => Ok(Tile::LapisOre),
        17 => Ok(Tile::Lava),
        18 => Ok(Tile::LavaBrick),
        19 => Ok(Tile::Exploded),
        20 => Ok(Tile::Farmland),
        21 => Ok(Tile::Wheat),
        22 => Ok(Tile::HardRock),
        23 => Ok(Tile::InfiniteFall),
        24 => Ok(Tile::Cloud),
        25 => Ok(Tile::CloudOre),
        26 => Ok(Tile::WoodDoor),
        27 => Ok(Tile::StoneDoor),
        28 => Ok(Tile::ObsidianDoor),
        29 => Ok(Tile::WoodFloor),
        30 => Ok(Tile::StoneFloor),
        31 => Ok(Tile::ObsidianFloor),
        32 => Ok(Tile::WoodWall),
        33 => Ok(Tile::StoneWall),
        34 => Ok(Tile::ObsidianWall),
        35 => Ok(Tile::WhiteWool),
        36 => Ok(Tile::Path),
        37 => Ok(Tile::RedWool),
        38 => Ok(Tile::BlueWool),
        39 => Ok(Tile::GreenWool),
        40 => Ok(Tile::YellowWool),
        41 => Ok(Tile::BlackWool),
        42 => Ok(Tile::Potato),
        43 => Ok(Tile::RawStone),
        44 => Ok(Tile::RawObsidian),
        45 => Ok(Tile::OrnateStone),
        46 => Ok(Tile::OrnateObsidian),
        47 => Ok(Tile::BossWall),
        48 => Ok(Tile::BossFloor),
        49 => Ok(Tile::BossDoor),
        50 => Ok(Tile::Tomato),
        51 => Ok(Tile::Carrot),
        52 => Ok(Tile::HeavenlyBerries),
        53 => Ok(Tile::HellishBerries),
        54 => Ok(Tile::WoodFence),
        55 => Ok(Tile::StoneFence),
        56 => Ok(Tile::ObsidianFence),
        57 => Ok(Tile::Torch),
        58 => Ok(Tile::Sign),
        59 => Ok(Tile::OrnateWood),
        60 => Ok(Tile::BrownWool),
        61 => Ok(Tile::PurpleWool),
        62 => Ok(Tile::CyanWool),
        63 => Ok(Tile::LightGrayWool),
        64 => Ok(Tile::GrayWool),
        65 => Ok(Tile::PinkWool),
        66 => Ok(Tile::LimeWool),
        67 => Ok(Tile::LightBlueWool),
        68 => Ok(Tile::MagentaWool),
        69 => Ok(Tile::OrangeWool),
        _ => Err(format!("unsupported dev2 tile ID {id}")),
    }
    .map(|tile| {
        if tile == Tile::Flower {
            Tile::Flower230
        } else {
            tile
        }
    })
}

fn exact(fields: &[String], count: usize, name: &str) -> Result<(), String> {
    if fields.len() != count {
        Err(format!(
            "dev2 {name}: expected {count} fields, found {}",
            fields.len()
        ))
    } else {
        Ok(())
    }
}
fn boolean(value: &str) -> Result<(), String> {
    if value == "true" || value == "false" {
        Ok(())
    } else {
        Err(format!("invalid dev2 boolean {value:?}"))
    }
}
fn bounded(value: &str, max: u64, name: &str) -> Result<(), String> {
    if parse::<u64>(value, name)? > max {
        Err(format!("out-of-range dev2 {name}: {value}"))
    } else {
        Ok(())
    }
}
fn item(encoded: &str) -> Result<ImportedItem, String> {
    let (name, amount) = match encoded.rsplit_once('_') {
        Some((name, value)) => (name, parse::<u16>(value, "item count/durability")?),
        None => (encoded, 1),
    };
    let parsed = item_from_java(encoded)?;
    match &parsed {
        ImportedItem::Stack(id, _) => {
            if *id == ItemId::WateringCan {
                return Err("dev2 Watering Can instance state is not representable by the current shared player field; import refused".into());
            }
            if amount == 0 {
                return Err(format!("invalid zero-count item {encoded}"));
            }
            if !name.eq_ignore_ascii_case(id.display_name())
                && !(name.eq_ignore_ascii_case("White Wool") && *id == ItemId::Wool)
            {
                return Err(format!("unsupported dev2 item alias {name:?}"));
            }
        }
        ImportedItem::Tool(tool) => {
            if !name.eq_ignore_ascii_case(&tool.display_name())
                || amount > tool.max_durability
                || amount == 0
            {
                return Err(format!("invalid/unrepresentable dev2 tool {encoded}"));
            }
        }
    }
    Ok(parsed)
}
fn inventory(entries: &[String], capacity: usize) -> Result<Inventory, String> {
    let mut inv = Inventory::new(capacity);
    for entry in entries {
        match item(entry)? {
            ImportedItem::Stack(id, count) => {
                if u32::from(inv.count(id)) + u32::from(count) > u32::from(u16::MAX)
                    || inv.add(id, count) > 0
                {
                    return Err(format!("dev2 inventory does not fit losslessly: {entry}"));
                }
            }
            ImportedItem::Tool(tool) => {
                if inv.add_tool(tool).is_none() {
                    return Err("dev2 held+inventory exceeds Rust capacity".into());
                }
            }
        }
    }
    Ok(inv)
}

pub(super) fn validate(directory: &Path) -> Result<(), String> {
    let game = read_save(&directory.join("Game.miniplussave"))?;
    exact(&game, 10, "Game")?;
    let mode: Vec<_> = game[2].split(';').collect();
    bounded(mode[0], 3, "mode")?;
    if mode[0] == "3" {
        if mode.len() != 3 {
            return Err("invalid dev2 Score mode".into());
        }
        parse::<i32>(mode[1], "score ticks")?;
        if !["10", "20", "40", "60", "120"].contains(&mode[2]) {
            return Err("invalid dev2 Score initial time".into());
        }
    } else if mode.len() != 1 {
        return Err("unexpected mode fields".into());
    }
    bounded(&game[3], u64::from(DAY_LENGTH) - 1, "day tick")?;
    parse::<i32>(&game[4], "game time")?;
    bounded(&game[5], 2, "difficulty")?;
    for i in [6, 7, 8, 9] {
        boolean(&game[i])?;
    }
    let mut size = 0;
    for (index, depth) in DEPTHS.iter().enumerate() {
        let level = read_save(&directory.join(format!("Level{index}.miniplussave")))?;
        if level.len() < 4 {
            return Err(format!("truncated dev2 Level{index}"));
        }
        let width = parse::<usize>(&level[0], "width")?;
        let height = parse::<usize>(&level[1], "height")?;
        if !matches!(width, 128 | 256 | 512) || width != height || (index > 0 && width != size) {
            return Err("invalid dev2 world dimensions".into());
        }
        size = width;
        parse::<i64>(&level[2], "level seed")?;
        if parse::<i8>(&level[3], "depth")? != *depth {
            return Err(format!("dev2 Level{index} depth must be {depth}"));
        }
        exact(&level, size * size + 4, "Level tiles")?;
        let data = read_save(&directory.join(format!("Level{index}data.miniplussave")))?;
        exact(&data, size * size, "Level data")?;
        for (name, data) in level[4..].iter().zip(&data) {
            tile(name, parse(data, "tile data")?)?;
        }
    }
    let player = read_save(&directory.join("Player.miniplussave"))?;
    exact(&player, 15, "Player")?;
    for i in [0, 1] {
        bounded(&player[i], (size * 16 - 1) as u64, "player position")?;
    }
    for i in [2, 3] {
        bounded(&player[i], (size - 1) as u64, "spawn position")?;
    }
    bounded(&player[5], 10, "extra health")?;
    bounded(
        &player[4],
        10 + parse::<u64>(&player[5], "extra health")?,
        "health",
    )?;
    bounded(&player[6], 10, "hunger")?;
    bounded(&player[11], 5, "player level")?;
    if !player[9].eq_ignore_ascii_case("NULL") && armor_from_java(&player[9]).is_none() {
        return Err(format!("unknown dev2 armor {}", player[9]));
    }
    let mut potions = [0; PotionKind::ALL.len()];
    import_potions(&player[12], &mut potions)?;
    for effect in player[12]
        .trim_start_matches("PotionEffects[")
        .trim_end_matches(']')
        .split(':')
        .filter(|s| !s.is_empty())
    {
        let (name, _) = effect.split_once(';').ok_or("invalid potion")?;
        if !PotionKind::ALL
            .into_iter()
            .any(|k| compact(k.display_name()) == compact(name))
        {
            return Err(format!("unknown dev2 potion {name}"));
        }
    }
    inventory(&read_save(&directory.join("Inventory.miniplussave"))?, 27)?;
    // Parse container contents too; the legacy importer must never silently
    // skip an unsupported item or overfill/drop a container in this profile.
    inspect_entities(directory, |_, _, _, _, _| Ok(()))?;
    Ok(())
}

fn inspect_entities(
    directory: &Path,
    mut accept: impl FnMut(&str, i32, i32, usize, &[String]) -> Result<(), String>,
) -> Result<(), String> {
    for encoded in read_save(&directory.join("Entities.miniplussave"))? {
        let (name, body) = encoded
            .split_once('[')
            .ok_or_else(|| format!("invalid dev2 entity {encoded:?}"))?;
        let body = body.strip_suffix(']').ok_or("truncated dev2 entity")?;
        let fields: Vec<String> = body.split(':').map(str::to_owned).collect();
        if fields.len() < 3 {
            return Err("truncated entity fields".into());
        }
        let x = parse(&fields[0], "entity x")?;
        let y = parse(&fields[1], "entity y")?;
        let level = parse::<usize>(fields.last().unwrap(), "entity level")?;
        if level >= 6 {
            return Err(format!("invalid dev2 entity level {level}"));
        }
        accept(name, x, y, level, &fields)?;
    }
    Ok(())
}

pub(super) fn import_entities(directory: &Path, world: &mut World) -> Result<(), String> {
    inspect_entities(directory, |name, x, y, level, fields| {
        if x < 0 || y < 0 || x >= world.width as i32 * 16 || y >= world.height as i32 * 16 {
            return Err(format!("invalid entity position {name} {x},{y}"));
        }
        if name == "Player" {
            exact(fields, 4, "Player entity")?;
            if x != world.player.x
                || y != world.player.y
                || level != world.current_level
                || parse::<u8>(&fields[2], "Player entity health")? != world.player.health
            {
                return Err("inconsistent Player entity and Player file".into());
            }
            return Ok(());
        }
        if name == "SignTileEntity" {
            exact(fields, 3, "SignTileEntity")?;
            let index = (x as usize / 16) + (y as usize / 16) * world.width;
            if world.levels[level].tiles[index] != Tile::Sign {
                return Err("orphan dev2 SignTileEntity".into());
            }
            // The native Sign tile + signs text map represent this Java helper.
            return Ok(());
        }
        if let Some(species) = mob_from_java(name) {
            let boss = matches!(
                species,
                spawn::NaturalMob::AirWizard | spawn::NaturalMob::ObsidianKnight
            );
            if species.hostile() && !boss {
                return Err(format!(
                    "dev2 hostile {name}: level/scaled stats not represented; import refused"
                ));
            }
            if boss && fields.get(3).map(String::as_str) != Some("1") {
                return Err("unsupported boss level".into());
            }
            exact(
                fields,
                if species == spawn::NaturalMob::Sheep || boss {
                    5
                } else {
                    4
                },
                "passive mob",
            )?;
            let health = parse::<u16>(&fields[2], "mob health")?;
            if health > crate::entity::stats::mob_health(species) {
                return Err("unrepresentable mob health".into());
            }
            if species == spawn::NaturalMob::Sheep {
                boolean(&fields[3])?;
            }
            world.levels[level].entities.import_mob(
                species,
                x,
                y,
                Some(health),
                species == spawn::NaturalMob::Sheep && fields[3] == "true",
            );
            return Ok(());
        }
        let key = compact(name);
        let bed = DyeColor::ALL
            .into_iter()
            .find(|c| c.bed_item().display_name().eq_ignore_ascii_case(name));
        let (kind, state) = if let Some(color) = bed {
            (
                FurnitureKind::Bed,
                crate::entity::furniture::bed::state_for_color(color),
            )
        } else if key == "dyevat" {
            (FurnitureKind::DyeVat, 0)
        } else {
            (
                furniture_from_java(name, None)
                    .ok_or_else(|| format!("unsupported dev2 entity {name:?}"))?,
                0,
            )
        };
        if matches!(kind, FurnitureKind::KnightStatue) {
            return Err("dev2 KnightStatue boss-health state unsupported".into());
        }
        if matches!(kind, FurnitureKind::Chest | FurnitureKind::DungeonChest) {
            let death = key == "deathchest";
            let extra = death || kind == FurnitureKind::DungeonChest;
            if fields.len() < 3 + usize::from(extra) {
                return Err("truncated chest".into());
            }
            let end = fields.len() - 1 - usize::from(extra);
            let contents = inventory(&fields[2..end], if death { usize::MAX } else { 27 })?;
            let arena = &mut world.levels[level].entities;
            let id = if death {
                let remaining = parse::<u32>(&fields[end], "death chest time")?;
                let id = arena.spawn_death_chest(x, y, Vec::new(), Vec::new());
                arena.set_death_chest_remaining(id, remaining);
                id
            } else {
                let id = arena.spawn_furniture(kind, x, y);
                if extra {
                    boolean(&fields[end])?;
                    arena
                        .entities
                        .iter_mut()
                        .find(|e| e.id == id)
                        .unwrap()
                        .state = u16::from(fields[end] == "false"); // Native 0 means locked.
                }
                id
            };
            arena
                .entities
                .iter_mut()
                .find(|e| e.id == id)
                .unwrap()
                .storage = Some(contents);
            return Ok(());
        }
        let kind = if kind == FurnitureKind::Lantern {
            exact(fields, 4, "Lantern")?;
            match fields[2].as_str() {
                "0" => kind,
                "1" => FurnitureKind::IronLantern,
                "2" => FurnitureKind::GoldLantern,
                _ => return Err("invalid lantern type".into()),
            }
        } else {
            exact(fields, 3, "furniture")?;
            kind
        };
        world.levels[level]
            .entities
            .import_furniture(kind, x, y, state, &[], &[])?;
        Ok(())
    })
}

pub(super) fn import_signs(directory: &Path, world: &mut World) -> Result<(), String> {
    let path = directory.join("signs.json");
    if !path.exists() {
        return Ok(());
    }
    let value: Value = serde_json::from_str(&read_text_limited(&path, MAX_JAVA_FILE_BYTES)?)
        .map_err(|e| e.to_string())?;
    let signs = value["signs"].as_array().ok_or("invalid dev2 signs")?;
    for sign in signs {
        let depth = sign["level"].as_i64().ok_or("sign depth")?;
        let level = world
            .levels
            .iter()
            .position(|l| i64::from(l.depth) == depth)
            .ok_or("unknown sign depth")?;
        let x = sign["x"].as_u64().ok_or("sign x")? as usize;
        let y = sign["y"].as_u64().ok_or("sign y")? as usize;
        if level >= 6 || x >= world.width || y >= world.height {
            return Err("sign outside level".into());
        }
        let lines = sign["lines"].as_array().ok_or("sign lines")?;
        let text = lines
            .iter()
            .map(|v| v.as_str().ok_or("invalid sign line"))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        if world.signs[level]
            .insert(x + y * world.width, text)
            .is_some()
        {
            return Err("duplicate sign position".into());
        }
    }
    Ok(())
}
