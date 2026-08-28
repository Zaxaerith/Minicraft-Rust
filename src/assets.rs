use std::{fs, path::Path};

use crate::{
    gfx::{Image, Screen},
    resource_pack::ResourcePack,
    world::Tile,
};

struct Connection {
    border: Image,
    corner: Option<Image>,
    singleton: bool,
}

pub struct Assets {
    pub font: Image,
    pub title: Image,
    pub skin: Image,
    pub skin_row: usize,
    tiles: Vec<Vec<Image>>,
    connections: Vec<Option<Connection>>,
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
        }
        Ok(assets)
    }

    pub fn tile(&self, tile: Tile, data: u16) -> &Image {
        debug_assert_eq!(Tile::from_id(tile.id()), Some(tile));
        let variants = &self.tiles[tile.id() as usize];
        &variants[tile_variant_index(tile, data, variants.len())]
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
