use std::{fs, path::Path};

use crate::{
    gfx::{Image, Screen},
    item::{ItemId, ToolItem, ToolKind, ToolTier},
    resource_pack::ResourcePack,
    world::{FurnitureKind, Tile, spawn::NaturalMob},
};

struct Connection {
    border: Image,
    corner: Option<Image>,
    singleton: bool,
}

pub struct Assets {
    pub font: Image,
    pub title: Image,
    pub hud: Image,
    pub inventory_counter: Image,
    pub skin: Image,
    pub skin_row: usize,
    tiles: Vec<Vec<Image>>,
    connections: Vec<Option<Connection>>,
    mobs: Vec<Image>,
    items: Vec<Image>,
    tools: Vec<Image>,
    furniture: Vec<Image>,
    pub warnings: Vec<String>,
}

impl Assets {
    pub fn load(packs: &[ResourcePack]) -> Result<Self, String> {
        let mut assets = Self {
            font: png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/textures/gui/font.png"
            )))?,
            title: png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/textures/gui/title.png"
            )))?,
            hud: png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/textures/gui/hud.png"
            )))?,
            inventory_counter: png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/textures/gui/inventory_counter.png"
            )))?,
            skin: png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/resources/textures/skins.png"
            )))?,
            skin_row: 0,
            tiles: Tile::ALL
                .iter()
                .map(|tile_id| {
                    tile_variants(*tile_id)
                        .into_iter()
                        .map(tile)
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?,
            connections: Tile::ALL
                .iter()
                .map(|tile_id| {
                    let Some((border, corner, singleton)) = connection_names(*tile_id) else {
                        return Ok(None);
                    };
                    Ok(Some(Connection {
                        border: tile(border)?,
                        corner: corner.map(tile).transpose()?,
                        singleton,
                    }))
                })
                .collect::<Result<Vec<_>, String>>()?,
            mobs: NaturalMob::ALL
                .iter()
                .map(|mob| entity(mob.asset_name()))
                .collect::<Result<Vec<_>, _>>()?,
            items: ItemId::ALL
                .iter()
                .map(|item| item_image(item.asset_name()))
                .collect::<Result<Vec<_>, _>>()?,
            tools: ToolKind::ALL
                .into_iter()
                .flat_map(|kind| ToolTier::ALL.into_iter().map(move |tier| (kind, tier)))
                .map(|(kind, tier)| item_image(kind.asset_name(tier)))
                .collect::<Result<Vec<_>, _>>()?,
            furniture: FurnitureKind::ALL
                .iter()
                .map(|kind| entity(kind.asset_name()))
                .collect::<Result<Vec<_>, _>>()?,
            warnings: Vec::new(),
        };
        for pack in packs {
            override_image(
                pack,
                "assets/textures/gui/font.png",
                &mut assets.font,
                &mut assets.warnings,
            );
            override_image(
                pack,
                "assets/textures/gui/title.png",
                &mut assets.title,
                &mut assets.warnings,
            );
            override_image(
                pack,
                "assets/textures/gui/hud.png",
                &mut assets.hud,
                &mut assets.warnings,
            );
            override_image(
                pack,
                "assets/textures/gui/inventory_counter.png",
                &mut assets.inventory_counter,
                &mut assets.warnings,
            );
            for tile_id in Tile::ALL {
                for (variant, name) in tile_variants(tile_id).into_iter().enumerate() {
                    override_image(
                        pack,
                        &format!("assets/textures/tile/{name}.png"),
                        &mut assets.tiles[tile_id.id() as usize][variant],
                        &mut assets.warnings,
                    );
                }
                if let Some(connection) = &mut assets.connections[tile_id.id() as usize]
                    && let Some((border, corner, _)) = connection_names(tile_id)
                {
                    override_image(
                        pack,
                        &format!("assets/textures/tile/{border}.png"),
                        &mut connection.border,
                        &mut assets.warnings,
                    );
                    if let (Some(name), Some(image)) = (corner, &mut connection.corner) {
                        override_image(
                            pack,
                            &format!("assets/textures/tile/{name}.png"),
                            image,
                            &mut assets.warnings,
                        );
                    }
                }
            }
            for mob in NaturalMob::ALL {
                override_image(
                    pack,
                    &format!("assets/textures/entity/{}.png", mob.asset_name()),
                    &mut assets.mobs[mob.id()],
                    &mut assets.warnings,
                );
            }
            for (index, item) in ItemId::ALL.iter().enumerate() {
                override_image(
                    pack,
                    &format!("assets/textures/item/{}.png", item.asset_name()),
                    &mut assets.items[index],
                    &mut assets.warnings,
                );
            }
            for kind in ToolKind::ALL {
                for tier in ToolTier::ALL {
                    override_image(
                        pack,
                        &format!("assets/textures/item/{}.png", kind.asset_name(tier)),
                        &mut assets.tools[kind.id() * ToolTier::ALL.len() + tier.level() as usize],
                        &mut assets.warnings,
                    );
                }
            }
            for kind in FurnitureKind::ALL {
                override_image(
                    pack,
                    &format!("assets/textures/entity/{}.png", kind.asset_name()),
                    &mut assets.furniture[kind.id()],
                    &mut assets.warnings,
                );
            }
        }
        Ok(assets)
    }

    pub fn tile(&self, tile: Tile, data: u16) -> &Image {
        debug_assert_eq!(Tile::from_id(tile.id()), Some(tile));
        let variants = &self.tiles[tile.id() as usize];
        &variants[tile_variant_index(tile, data, variants.len())]
    }

    pub fn mob(&self, mob: NaturalMob) -> &Image {
        &self.mobs[mob.id()]
    }

    pub fn item(&self, item: ItemId) -> &Image {
        &self.items[ItemId::ALL
            .iter()
            .position(|candidate| *candidate == item)
            .expect("registered item")]
    }

    pub fn tool(&self, tool: ToolItem) -> &Image {
        &self.tools[tool.kind.id() * ToolTier::ALL.len() + tool.tier.level() as usize]
    }

    pub fn furniture(&self, kind: FurnitureKind) -> &Image {
        &self.furniture[kind.id()]
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_tile(
        &self,
        screen: &mut Screen,
        tile: Tile,
        data: u16,
        x: i32,
        y: i32,
        frame: usize,
        connected: [bool; 8],
    ) {
        let image = self.tile(tile, data);
        let Some(connection) = &self.connections[tile.id() as usize] else {
            screen.blit_region(
                image,
                x,
                y,
                0,
                frame * 16,
                16.min(image.width),
                16.min(image.height),
                false,
            );
            return;
        };
        let [
            up,
            down,
            left,
            right,
            up_left,
            down_left,
            up_right,
            down_right,
        ] = connected;
        let pieces = [
            quadrant(connection, image, frame, up, left, up_left, 0),
            quadrant(connection, image, frame, up, right, up_right, 1),
            quadrant(connection, image, frame, down, left, down_left, 2),
            quadrant(connection, image, frame, down, right, down_right, 3),
        ];
        for (index, (source, source_x, source_y)) in pieces.into_iter().enumerate() {
            screen.blit_region(
                source,
                x + (index % 2) as i32 * 8,
                y + (index / 2) as i32 * 8,
                source_x,
                source_y,
                8,
                8,
                false,
            );
        }
    }

    pub fn select_skin(&mut self, id: &str, game_dir: &Path) -> Result<(), String> {
        let built_in_row = match id {
            "minicraft.skin.paul" => Some(0),
            "minicraft.skin.paul_cape" => Some(32),
            "minicraft.skin.minecraft_steve" => Some(64),
            "minicraft.skin.minecraft_alex" => Some(96),
            _ => None,
        };
        if let Some(row) = built_in_row {
            self.skin = png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/resources/textures/skins.png"
            )))?;
            self.skin_row = row;
            return Ok(());
        }

        let path = game_dir.join("skins").join(format!("{id}.png"));
        let bytes = fs::read(&path)
            .map_err(|error| format!("cannot read skin {}: {error}", path.display()))?;
        let image = Image::from_png(&bytes)?;
        if image.width < 64
            || image.height < 32
            || !image.width.is_multiple_of(8)
            || !image.height.is_multiple_of(8)
        {
            return Err(format!(
                "skin {} must be at least 64x32 with dimensions divisible by 8",
                path.display()
            ));
        }
        self.skin = image;
        self.skin_row = 0;
        Ok(())
    }
}

pub fn skin_options(game_dir: &Path) -> Vec<String> {
    let mut options = vec![
        "minicraft.skin.paul".to_owned(),
        "minicraft.skin.paul_cape".to_owned(),
        "minicraft.skin.minecraft_steve".to_owned(),
        "minicraft.skin.minecraft_alex".to_owned(),
    ];
    let directory = game_dir.join("skins");
    if let Ok(entries) = fs::read_dir(directory) {
        let mut custom = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
            })
            .filter_map(|path| {
                path.file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect::<Vec<_>>();
        custom.sort_by_key(|name| name.to_ascii_lowercase());
        custom.dedup();
        options.extend(custom);
    }
    options
}

fn override_image(pack: &ResourcePack, path: &str, target: &mut Image, warnings: &mut Vec<String>) {
    match pack.read(path) {
        Ok(Some(bytes)) => match Image::from_png(&bytes) {
            Ok(image) if image.width.is_multiple_of(8) && image.height.is_multiple_of(8) => {
                *target = image;
            }
            Ok(_) => warnings.push(format!(
                "{}: {path} dimensions are not multiples of 8; ignored",
                pack.name
            )),
            Err(error) => warnings.push(format!("{}: invalid {path}: {error}", pack.name)),
        },
        Ok(None) => {}
        Err(error) => warnings.push(error),
    }
}

fn png(bytes: &[u8]) -> Result<Image, String> {
    Image::from_png(bytes)
}

fn tile_variants(tile: Tile) -> Vec<&'static str> {
    match tile {
        Tile::Flower => vec!["flower_shape0", "flower_shape1"],
        Tile::Farmland => vec!["farmland", "farmland_moist"],
        Tile::Wheat => vec![
            "wheat_stage0",
            "wheat_stage1",
            "wheat_stage2",
            "wheat_stage3",
            "wheat_stage4",
            "wheat_stage5",
        ],
        Tile::Potato => vec![
            "potato_stage0",
            "potato_stage1",
            "potato_stage2",
            "potato_stage3",
            "potato_stage4",
            "potato_stage5",
        ],
        Tile::Tomato => vec![
            "tomato_stage0",
            "tomato_stage1",
            "tomato_stage2",
            "tomato_stage3",
        ],
        Tile::Carrot => vec![
            "carrot_stage0",
            "carrot_stage1",
            "carrot_stage2",
            "carrot_stage3",
        ],
        Tile::HeavenlyBerries => vec![
            "heavenly_berries_stage0",
            "heavenly_berries_stage1",
            "heavenly_berries_stage2",
            "heavenly_berries_stage3",
        ],
        Tile::HellishBerries => vec![
            "hellish_berries_stage0",
            "hellish_berries_stage1",
            "hellish_berries_stage2",
            "hellish_berries_stage3",
        ],
        Tile::WoodDoor => vec!["wood_door", "wood_door_opened"],
        Tile::StoneDoor => vec!["stone_door", "stone_door_opened"],
        Tile::ObsidianDoor | Tile::BossDoor => {
            vec!["obsidian_door", "obsidian_door_opened"]
        }
        _ => vec![tile.asset_name()],
    }
}

fn tile_variant_index(tile: Tile, data: u16, count: usize) -> usize {
    match tile {
        Tile::Flower => ((data >> 4) as usize) % count,
        Tile::Farmland => usize::from(data & 7 > 0).min(count - 1),
        Tile::Wheat
        | Tile::Potato
        | Tile::Tomato
        | Tile::Carrot
        | Tile::HeavenlyBerries
        | Tile::HellishBerries => ((data >> 3) & 7) as usize * (count - 1) / 7,
        Tile::WoodDoor | Tile::StoneDoor | Tile::ObsidianDoor | Tile::BossDoor => {
            usize::from(data & 1 != 0).min(count - 1)
        }
        _ => 0,
    }
}

fn connection_names(tile: Tile) -> Option<(&'static str, Option<&'static str>, bool)> {
    Some(match tile {
        Tile::Grass => ("grass_border", None, true),
        Tile::Sand => ("sand_border", None, true),
        Tile::Water => ("water_border", None, true),
        Tile::Lava => ("lava_border", None, true),
        Tile::Hole => ("hole_border", None, true),
        Tile::Rock => ("rock_border", Some("rock_corner"), true),
        Tile::HardRock => ("hardrock_border", Some("hardrock_corner"), true),
        Tile::Cloud => ("cloud_border", Some("cloud_corner"), true),
        Tile::Exploded => ("exploded_border", None, false),
        Tile::WoodWall => ("wood_wall_border", None, false),
        Tile::StoneWall => ("stone_wall_border", None, false),
        Tile::ObsidianWall | Tile::BossWall => ("obsidian_wall_border", None, false),
        _ => return None,
    })
}

fn quadrant<'a>(
    connection: &'a Connection,
    full: &'a Image,
    frame: usize,
    vertical: bool,
    horizontal: bool,
    diagonal: bool,
    corner: usize,
) -> (&'a Image, usize, usize) {
    if vertical && horizontal {
        if !diagonal && let Some(sides) = &connection.corner {
            return (sides, corner % 2 * 8, corner / 2 * 8);
        }
        if connection.singleton || !diagonal {
            let source_x = if corner.is_multiple_of(2) { 8 } else { 0 };
            let source_y = frame * 16 + if corner < 2 { 8 } else { 0 };
            return (full, source_x, source_y);
        }
        return (&connection.border, 8, 8);
    }
    let source_x = match (corner % 2, horizontal) {
        (0, false) => 0,
        (0, true) => 8,
        (1, true) => 8,
        _ => 16,
    };
    let source_y = match (corner / 2, vertical) {
        (0, false) => 0,
        (0, true) => 8,
        (1, true) => 8,
        _ => 16,
    };
    (&connection.border, source_x, source_y)
}

fn tile(name: &str) -> Result<Image, String> {
    macro_rules! bytes {
        ($file:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/textures/tile/",
                $file
            ))
        };
    }
    let bytes: &[u8] = match name {
        "grass" => bytes!("grass.png"),
        "grass_border" => bytes!("grass_border.png"),
        "dirt" => bytes!("dirt.png"),
        "flower_shape0" => bytes!("flower_shape0.png"),
        "flower_shape1" => bytes!("flower_shape1.png"),
        "hole" => bytes!("hole.png"),
        "hole_border" => bytes!("hole_border.png"),
        "stairs_up" => bytes!("stairs_up.png"),
        "stairs_down" => bytes!("stairs_down.png"),
        "water" => bytes!("water.png"),
        "water_border" => bytes!("water_border.png"),
        "rock" => bytes!("rock.png"),
        "rock_border" => bytes!("rock_border.png"),
        "rock_corner" => bytes!("rock_corner.png"),
        "oak" => bytes!("oak.png"),
        "sapling" => bytes!("sapling.png"),
        "sand" => bytes!("sand.png"),
        "sand_border" => bytes!("sand_border.png"),
        "cactus" => bytes!("cactus.png"),
        "iron_ore" => bytes!("iron_ore.png"),
        "gold_ore" => bytes!("gold_ore.png"),
        "gem_ore" => bytes!("gem_ore.png"),
        "lapis_ore" => bytes!("lapis_ore.png"),
        "lava" => bytes!("lava.png"),
        "lava_border" => bytes!("lava_border.png"),
        "missing_tile" => bytes!("missing_tile.png"),
        "stone" => bytes!("stone.png"),
        "exploded" => bytes!("exploded.png"),
        "exploded_border" => bytes!("exploded_border.png"),
        "farmland" => bytes!("farmland.png"),
        "farmland_moist" => bytes!("farmland_moist.png"),
        "wheat_stage0" => bytes!("wheat_stage0.png"),
        "wheat_stage1" => bytes!("wheat_stage1.png"),
        "wheat_stage2" => bytes!("wheat_stage2.png"),
        "wheat_stage3" => bytes!("wheat_stage3.png"),
        "wheat_stage4" => bytes!("wheat_stage4.png"),
        "wheat_stage5" => bytes!("wheat_stage5.png"),
        "hardrock" => bytes!("hardrock.png"),
        "hardrock_border" => bytes!("hardrock_border.png"),
        "hardrock_corner" => bytes!("hardrock_corner.png"),
        "cloud_background" => bytes!("cloud_background.png"),
        "cloud" => bytes!("cloud.png"),
        "cloud_border" => bytes!("cloud_border.png"),
        "cloud_corner" => bytes!("cloud_corner.png"),
        "cloud_ore" => bytes!("cloud_ore.png"),
        "wood_door" => bytes!("wood_door.png"),
        "wood_door_opened" => bytes!("wood_door_opened.png"),
        "stone_door" => bytes!("stone_door.png"),
        "stone_door_opened" => bytes!("stone_door_opened.png"),
        "obsidian_door" => bytes!("obsidian_door.png"),
        "obsidian_door_opened" => bytes!("obsidian_door_opened.png"),
        "wood_floor" => bytes!("wood_floor.png"),
        "stone_floor" => bytes!("stone_floor.png"),
        "obsidian_floor" => bytes!("obsidian_floor.png"),
        "wood_wall" => bytes!("wood_wall.png"),
        "wood_wall_border" => bytes!("wood_wall_border.png"),
        "stone_wall" => bytes!("stone_wall.png"),
        "stone_wall_border" => bytes!("stone_wall_border.png"),
        "obsidian_wall" => bytes!("obsidian_wall.png"),
        "obsidian_wall_border" => bytes!("obsidian_wall_border.png"),
        "white_wool" => bytes!("white_wool.png"),
        "path" => bytes!("path.png"),
        "red_wool" => bytes!("red_wool.png"),
        "blue_wool" => bytes!("blue_wool.png"),
        "green_wool" => bytes!("green_wool.png"),
        "yellow_wool" => bytes!("yellow_wool.png"),
        "black_wool" => bytes!("black_wool.png"),
        "potato_stage5" => bytes!("potato_stage5.png"),
        "potato_stage0" => bytes!("potato_stage0.png"),
        "potato_stage1" => bytes!("potato_stage1.png"),
        "potato_stage2" => bytes!("potato_stage2.png"),
        "potato_stage3" => bytes!("potato_stage3.png"),
        "potato_stage4" => bytes!("potato_stage4.png"),
        "obsidian" => bytes!("obsidian.png"),
        "ornate_stone" => bytes!("ornate_stone.png"),
        "ornate_obsidian" => bytes!("ornate_obsidian.png"),
        "tomato_stage3" => bytes!("tomato_stage3.png"),
        "tomato_stage0" => bytes!("tomato_stage0.png"),
        "tomato_stage1" => bytes!("tomato_stage1.png"),
        "tomato_stage2" => bytes!("tomato_stage2.png"),
        "carrot_stage3" => bytes!("carrot_stage3.png"),
        "carrot_stage0" => bytes!("carrot_stage0.png"),
        "carrot_stage1" => bytes!("carrot_stage1.png"),
        "carrot_stage2" => bytes!("carrot_stage2.png"),
        "heavenly_berries_stage3" => bytes!("heavenly_berries_stage3.png"),
        "heavenly_berries_stage0" => bytes!("heavenly_berries_stage0.png"),
        "heavenly_berries_stage1" => bytes!("heavenly_berries_stage1.png"),
        "heavenly_berries_stage2" => bytes!("heavenly_berries_stage2.png"),
        "hellish_berries_stage3" => bytes!("hellish_berries_stage3.png"),
        "hellish_berries_stage0" => bytes!("hellish_berries_stage0.png"),
        "hellish_berries_stage1" => bytes!("hellish_berries_stage1.png"),
        "hellish_berries_stage2" => bytes!("hellish_berries_stage2.png"),
        "wood_fence" => bytes!("wood_fence.png"),
        "stone_fence" => bytes!("stone_fence.png"),
        "obsidian_fence" => bytes!("obsidian_fence.png"),
        "torch" => bytes!("torch.png"),
        "sign" => bytes!("sign.png"),
        _ => return Err(format!("unknown built-in tile asset: {name}")),
    };
    png(bytes)
}

fn entity(name: &str) -> Result<Image, String> {
    macro_rules! bytes {
        ($file:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/textures/entity/",
                $file
            ))
        };
    }
    let bytes: &[u8] = match name {
        "slime" => bytes!("slime.png"),
        "zombie" => bytes!("zombie.png"),
        "creeper" => bytes!("creeper.png"),
        "skeleton" => bytes!("skeleton.png"),
        "snake" => bytes!("snake.png"),
        "knight" => bytes!("knight.png"),
        "cow" => bytes!("cow.png"),
        "pig" => bytes!("pig.png"),
        "sheep" => bytes!("sheep.png"),
        "air_wizard" => bytes!("air_wizard.png"),
        "obsidian_knight" => bytes!("obsidian_knight_armored.png"),
        "workbench" => bytes!("workbench.png"),
        "oven" => bytes!("oven.png"),
        "furnace" => bytes!("furnace.png"),
        "anvil" => bytes!("anvil.png"),
        "enchanter" => bytes!("enchanter.png"),
        "loom" => bytes!("loom.png"),
        "chest" => bytes!("chest.png"),
        "dungeon_chest" => bytes!("dungeon_chest.png"),
        "lantern" => bytes!("lantern.png"),
        "iron_lantern" => bytes!("iron_lantern.png"),
        "gold_lantern" => bytes!("gold_lantern.png"),
        "tnt" => bytes!("tnt.png"),
        "bed" => bytes!("bed.png"),
        "composter" => bytes!("composter.png"),
        "knight_statue" => bytes!("knight_statue.png"),
        "spawner" => bytes!("spawner.png"),
        _ => bytes!("missing_entity.png"),
    };
    png(bytes)
}

fn item_image(name: &str) -> Result<Image, String> {
    macro_rules! bytes {
        ($file:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/textures/item/",
                $file
            ))
        };
    }
    let bytes: &[u8] = match name {
        "wood" => bytes!("wood.png"),
        "stone" => bytes!("stone.png"),
        "coal" => bytes!("coal.png"),
        "slime" => bytes!("slime.png"),
        "cloth" => bytes!("cloth.png"),
        "leather" => bytes!("leather.png"),
        "beef" => bytes!("beef.png"),
        "pork" => bytes!("pork.png"),
        "wool" => bytes!("wool.png"),
        "bone" => bytes!("bone.png"),
        "arrow" => bytes!("arrow.png"),
        "gunpowder" => bytes!("gunpowder.png"),
        "scale" => bytes!("scale.png"),
        "shard" => bytes!("shard.png"),
        "plank" => bytes!("plank.png"),
        "torch" => bytes!("torch.png"),
        "workbench" => bytes!("workbench.png"),
        "iron_ore" => bytes!("iron_ore.png"),
        "gold_ore" => bytes!("gold_ore.png"),
        "gem" => bytes!("gem.png"),
        "lapis" => bytes!("lapis.png"),
        "cloud_ore" => bytes!("cloud_ore.png"),
        "iron_ingot" => bytes!("iron_ingot.png"),
        "gold_ingot" => bytes!("gold_ingot.png"),
        "oven" => bytes!("oven.png"),
        "furnace" => bytes!("furnace.png"),
        "anvil" => bytes!("anvil.png"),
        "enchanter" => bytes!("enchanter.png"),
        "loom" => bytes!("loom.png"),
        "string" => bytes!("string.png"),
        "cooked_pork" => bytes!("cooked_pork.png"),
        "cooked_beef" => bytes!("cooked_beef.png"),
        "arcane_fertilizer" => bytes!("arcane_fertilizer.png"),
        "apple" => bytes!("apple.png"),
        "fish" => bytes!("fish.png"),
        "bread" => bytes!("bread.png"),
        "cooked_fish" => bytes!("cooked_fish.png"),
        "golden_apple" => bytes!("golden_apple.png"),
        "potato" => bytes!("potato.png"),
        "baked_potato" => bytes!("baked_potato.png"),
        "wheat" => bytes!("wheat.png"),
        "key" => bytes!("key.png"),
        "red_flower" => bytes!("red_flower.png"),
        "white_flower" => bytes!("white_flower.png"),
        "cactus" => bytes!("cactus.png"),
        "sand" => bytes!("sand.png"),
        "glass" => bytes!("glass.png"),
        "glass_bottle" => bytes!("glass_bottle.png"),
        "fertilizer" => bytes!("fertilizer.png"),
        "dirt" => bytes!("dirt.png"),
        "cloud" => bytes!("cloud.png"),
        "plank_wall" => bytes!("plank_wall.png"),
        "wood_door" => bytes!("wood_door.png"),
        "wood_fence" => bytes!("wood_fence.png"),
        "stone_brick" => bytes!("stone_brick.png"),
        "stone_wall" => bytes!("stone_wall.png"),
        "stone_door" => bytes!("stone_door.png"),
        "stone_fence" => bytes!("stone_fence.png"),
        "obsidian" => bytes!("obsidian.png"),
        "obsidian_brick" => bytes!("obsidian_brick.png"),
        "obsidian_wall" => bytes!("obsidian_wall.png"),
        "obsidian_door" => bytes!("obsidian_door.png"),
        "obsidian_fence" => bytes!("obsidian_fence.png"),
        "red_wool" => bytes!("red_wool.png"),
        "blue_wool" => bytes!("blue_wool.png"),
        "green_wool" => bytes!("green_wool.png"),
        "yellow_wool" => bytes!("yellow_wool.png"),
        "black_wool" => bytes!("black_wool.png"),
        "red_clothes" => bytes!("red_clothes.png"),
        "blue_clothes" => bytes!("blue_clothes.png"),
        "green_clothes" => bytes!("green_clothes.png"),
        "yellow_clothes" => bytes!("yellow_clothes.png"),
        "black_clothes" => bytes!("black_clothes.png"),
        "orange_clothes" => bytes!("orange_clothes.png"),
        "purple_clothes" => bytes!("purple_clothes.png"),
        "cyan_clothes" => bytes!("cyan_clothes.png"),
        "reg_clothes" => bytes!("reg_clothes.png"),
        "leather_armor" => bytes!("leather_armor.png"),
        "snake_armor" => bytes!("snake_armor.png"),
        "iron_armor" => bytes!("iron_armor.png"),
        "gold_armor" => bytes!("gold_armor.png"),
        "gem_armor" => bytes!("gem_armor.png"),
        "bucket" => bytes!("bucket.png"),
        "water_bucket" => bytes!("water_bucket.png"),
        "lava_bucket" => bytes!("lava_bucket.png"),
        "potion" => bytes!("potion.png"),
        "air_totem" => bytes!("air_totem.png"),
        "knight_statue" => bytes!("knight_statue.png"),
        "obsidian_heart" => bytes!("obsidian_heart.png"),
        "wooden_fishing_rod" => bytes!("wooden_fishing_rod.png"),
        "iron_fishing_rod" => bytes!("iron_fishing_rod.png"),
        "gold_fishing_rod" => bytes!("gold_fishing_rod.png"),
        "gem_fishing_rod" => bytes!("gem_fishing_rod.png"),
        "watering_can" => bytes!("watering_can.png"),
        "seed" => bytes!("seed.png"),
        "carrot" => bytes!("carrot.png"),
        "tomato" => bytes!("tomato.png"),
        "heavenly_berries" => bytes!("heavenly_berries.png"),
        "hellish_berries" => bytes!("hellish_berries.png"),
        "sign" => bytes!("sign.png"),
        "chest" => bytes!("chest.png"),
        "dungeon_chest" => bytes!("dungeon_chest.png"),
        "tnt" => bytes!("tnt.png"),
        "bed" => bytes!("bed.png"),
        "composter" => bytes!("composter.png"),
        "lantern" => bytes!("lantern.png"),
        "iron_lantern" => bytes!("iron_lantern.png"),
        "gold_lantern" => bytes!("gold_lantern.png"),
        "cow_spawner" => bytes!("cow_spawner.png"),
        "pig_spawner" => bytes!("pig_spawner.png"),
        "sheep_spawner" => bytes!("sheep_spawner.png"),
        "slime_spawner" => bytes!("slime_spawner.png"),
        "zombie_spawner" => bytes!("zombie_spawner.png"),
        "creeper_spawner" => bytes!("creeper_spawner.png"),
        "skeleton_spawner" => bytes!("skeleton_spawner.png"),
        "snake_spawner" => bytes!("snake_spawner.png"),
        "knight_spawner" => bytes!("knight_spawner.png"),
        "book" => bytes!("book.png"),
        "antidious_book" => bytes!("antidious_book.png"),
        "wooden_shovel" => bytes!("wooden_shovel.png"),
        "stone_shovel" => bytes!("stone_shovel.png"),
        "iron_shovel" => bytes!("iron_shovel.png"),
        "gold_shovel" => bytes!("gold_shovel.png"),
        "gem_shovel" => bytes!("gem_shovel.png"),
        "wooden_hoe" => bytes!("wooden_hoe.png"),
        "stone_hoe" => bytes!("stone_hoe.png"),
        "iron_hoe" => bytes!("iron_hoe.png"),
        "gold_hoe" => bytes!("gold_hoe.png"),
        "gem_hoe" => bytes!("gem_hoe.png"),
        "wooden_sword" => bytes!("wooden_sword.png"),
        "stone_sword" => bytes!("stone_sword.png"),
        "iron_sword" => bytes!("iron_sword.png"),
        "gold_sword" => bytes!("gold_sword.png"),
        "gem_sword" => bytes!("gem_sword.png"),
        "wooden_pickaxe" => bytes!("wooden_pickaxe.png"),
        "stone_pickaxe" => bytes!("stone_pickaxe.png"),
        "iron_pickaxe" => bytes!("iron_pickaxe.png"),
        "gold_pickaxe" => bytes!("gold_pickaxe.png"),
        "gem_pickaxe" => bytes!("gem_pickaxe.png"),
        "wooden_axe" => bytes!("wooden_axe.png"),
        "stone_axe" => bytes!("stone_axe.png"),
        "iron_axe" => bytes!("iron_axe.png"),
        "gold_axe" => bytes!("gold_axe.png"),
        "gem_axe" => bytes!("gem_axe.png"),
        "wooden_bow" => bytes!("wooden_bow.png"),
        "stone_bow" => bytes!("stone_bow.png"),
        "iron_bow" => bytes!("iron_bow.png"),
        "gold_bow" => bytes!("gold_bow.png"),
        "gem_bow" => bytes!("gem_bow.png"),
        "wooden_claymore" => bytes!("wooden_claymore.png"),
        "stone_claymore" => bytes!("stone_claymore.png"),
        "iron_claymore" => bytes!("iron_claymore.png"),
        "gold_claymore" => bytes!("gold_claymore.png"),
        "gem_claymore" => bytes!("gem_claymore.png"),
        "shears" => bytes!("shears.png"),
        _ => bytes!("missing_item.png"),
    };
    png(bytes)
}
