use crate::gfx::sprite_linker::{LinkedSprite, SpriteLinker, SpriteType};
use std::{fs, path::Path};

pub(crate) fn animation_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

use crate::{
    entity::furniture::FurnitureKind,
    entity::mob::NaturalMob,
    gfx::{Image, Screen},
    item::{ItemId, ToolItem, ToolKind, ToolTier},
    level::tile::Tile,
    resource::pack::ResourcePack,
};

use crate::gfx::connection::Connection;

pub struct Assets {
    tile_animations: std::cell::RefCell<
        std::collections::HashMap<String, crate::gfx::sprite_animation::SpriteAnimation>,
    >,
    sheets: SpriteLinker,
    tile_metadata: std::collections::HashMap<String, crate::gfx::sprite_animation::SpriteMeta>,
    pub default_pack_logo: Image,
    pub fallback_pack_logo: Image,
    pub skin_row: usize,
    tiles: Vec<Vec<&'static str>>,
    pub warnings: Vec<String>,
}

impl Assets {
    pub fn font(&self) -> &Image {
        self.sheets
            .sheet(SpriteType::Gui, "font")
            .expect("loaded font")
    }
    pub fn title(&self) -> &Image {
        self.sheets
            .sheet(SpriteType::Gui, "title")
            .expect("loaded title")
    }
    pub fn hud(&self) -> &Image {
        self.sheets
            .sheet(SpriteType::Gui, "hud")
            .expect("loaded hud")
    }
    pub fn inventory_counter(&self) -> &Image {
        self.sheets
            .sheet(SpriteType::Gui, "inventory_counter")
            .expect("loaded inventory_counter")
    }
    pub fn smash(&self) -> &Image {
        self.sheets
            .sheet(SpriteType::Entity, "smash")
            .expect("loaded smash")
    }
    pub fn skin(&self) -> &Image {
        self.sheets
            .sheet(SpriteType::Entity, "skin.active")
            .expect("loaded skin")
    }
    pub fn load(packs: &[ResourcePack]) -> Result<Self, String> {
        let mut assets = Self {
            tile_animations: Default::default(),
            sheets: SpriteLinker::default(),
            tile_metadata: Default::default(),
            default_pack_logo: png(include_bytes!("../../../assets/pack.png"))?,
            fallback_pack_logo: png(include_bytes!("../../../assets/resources/default_pack.png"))?,
            skin_row: 0,
            tiles: Tile::ALL.iter().map(|id| tile_variants(*id)).collect(),
            warnings: Vec::new(),
        };
        assets.sheets.set(
            SpriteType::Gui,
            "font",
            png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/assets/textures/gui/font.png"
            )))?,
        );
        assets.sheets.set(
            SpriteType::Gui,
            "title",
            png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/assets/textures/gui/title_rust.png"
            )))?,
        );
        assets.sheets.set(
            SpriteType::Gui,
            "hud",
            png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/assets/textures/gui/hud.png"
            )))?,
        );
        assets.sheets.set(
            SpriteType::Gui,
            "inventory_counter",
            png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/assets/textures/gui/inventory_counter.png"
            )))?,
        );
        assets.sheets.set(
            SpriteType::Entity,
            "smash",
            png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/assets/textures/entity/smash.png"
            )))?,
        );
        assets.sheets.set(
            SpriteType::Entity,
            "skin.active",
            png(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/resources/textures/skins.png"
            )))?,
        );
        for id in Tile::ALL {
            let mut names = tile_variants(id);
            if let Some((border, corner, _)) = connection_names(id) {
                names.push(border);
                names.extend(corner);
            }
            for name in names {
                if assets.sheets.sheet(SpriteType::Tile, name).is_none() {
                    assets.sheets.set(SpriteType::Tile, name, tile(name)?);
                }
            }
        }
        for item in ItemId::ALL {
            assets.sheets.set(
                SpriteType::Item,
                item.asset_name(),
                item_image(item.asset_name())?,
            );
        }
        for kind in ToolKind::ALL {
            for tier in ToolTier::ALL {
                let key = kind.asset_name(tier);
                assets.sheets.set(SpriteType::Item, key, item_image(key)?);
            }
        }
        for mob in NaturalMob::ALL {
            assets.sheets.set(
                SpriteType::Entity,
                mob.asset_name(),
                entity(mob.asset_name())?,
            );
        }
        for kind in FurnitureKind::ALL {
            assets.sheets.set(
                SpriteType::Entity,
                kind.asset_name(),
                entity(kind.asset_name())?,
            );
        }
        assets.load_tile_metadata(
            "wood_wall",
            include_bytes!("../../../assets/assets/textures/tile/wood_wall.png.json"),
        )?;
        assets.load_tile_metadata(
            "water",
            include_bytes!("../../../assets/assets/textures/tile/water.png.json"),
        )?;
        assets.load_tile_metadata(
            "stone_wall",
            include_bytes!("../../../assets/assets/textures/tile/stone_wall.png.json"),
        )?;
        assets.load_tile_metadata(
            "sand",
            include_bytes!("../../../assets/assets/textures/tile/sand.png.json"),
        )?;
        assets.load_tile_metadata(
            "grass",
            include_bytes!("../../../assets/assets/textures/tile/grass.png.json"),
        )?;
        assets.load_tile_metadata(
            "rock",
            include_bytes!("../../../assets/assets/textures/tile/rock.png.json"),
        )?;
        assets.load_tile_metadata(
            "obsidian_wall",
            include_bytes!("../../../assets/assets/textures/tile/obsidian_wall.png.json"),
        )?;
        assets.load_tile_metadata(
            "exploded",
            include_bytes!("../../../assets/assets/textures/tile/exploded.png.json"),
        )?;
        assets.load_tile_metadata(
            "cloud",
            include_bytes!("../../../assets/assets/textures/tile/cloud.png.json"),
        )?;
        assets.load_tile_metadata(
            "hardrock",
            include_bytes!("../../../assets/assets/textures/tile/hardrock.png.json"),
        )?;
        assets.load_tile_metadata(
            "hole",
            include_bytes!("../../../assets/assets/textures/tile/hole.png.json"),
        )?;
        assets.load_tile_metadata(
            "lava",
            include_bytes!("../../../assets/assets/textures/tile/lava.png.json"),
        )?;
        for pack in packs {
            override_image(
                pack,
                "assets/textures/gui/font.png",
                assets
                    .sheets
                    .sheet_mut(SpriteType::Gui, "font")
                    .expect("loaded font"),
                &mut assets.warnings,
            );
            override_image(
                pack,
                "assets/textures/entity/smash.png",
                assets
                    .sheets
                    .sheet_mut(SpriteType::Entity, "smash")
                    .expect("loaded smash"),
                &mut assets.warnings,
            );
            override_image(
                pack,
                "assets/textures/gui/title.png",
                assets
                    .sheets
                    .sheet_mut(SpriteType::Gui, "title")
                    .expect("loaded title"),
                &mut assets.warnings,
            );
            override_image(
                pack,
                "assets/textures/gui/hud.png",
                assets
                    .sheets
                    .sheet_mut(SpriteType::Gui, "hud")
                    .expect("loaded hud"),
                &mut assets.warnings,
            );
            override_image(
                pack,
                "assets/textures/gui/inventory_counter.png",
                assets
                    .sheets
                    .sheet_mut(SpriteType::Gui, "inventory_counter")
                    .expect("loaded inventory_counter"),
                &mut assets.warnings,
            );
            let mut keys = std::collections::BTreeSet::new();
            for id in Tile::ALL {
                keys.extend(tile_variants(id));
                if let Some((border, corner, _)) = connection_names(id) {
                    keys.insert(border);
                    keys.extend(corner);
                }
            }
            let mut consumed = std::collections::HashSet::new();
            // Java loads metadata before the remaining ordinary PNG sheets.
            for &key in &keys {
                match pack.read(&format!("assets/textures/tile/{key}.png.json")) {
                    Ok(Some(bytes)) => {
                        consumed.insert(key.to_owned());
                        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            for field in ["key", "corner"] {
                                if let Some(name) = json
                                    .get("border")
                                    .and_then(|v| v.get(field))
                                    .and_then(|v| v.as_str())
                                    .filter(|v| !v.is_empty())
                                {
                                    consumed.insert(name.to_owned());
                                }
                            }
                        }
                        if let Err(error) = assets.apply_tile_metadata_pack(pack, key, &bytes) {
                            assets.warnings.push(format!("{}: {error}", pack.name));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        consumed.insert(key.to_owned());
                        assets.warnings.push(error);
                    }
                }
            }
            for key in keys {
                if !consumed.contains(key) {
                    override_image(
                        pack,
                        &format!("assets/textures/tile/{key}.png"),
                        assets
                            .sheets
                            .sheet_mut(SpriteType::Tile, key)
                            .expect("loaded tile"),
                        &mut assets.warnings,
                    );
                }
            }
            for mob in NaturalMob::ALL {
                override_image(
                    pack,
                    &format!("assets/textures/entity/{}.png", mob.asset_name()),
                    assets
                        .sheets
                        .sheet_mut(SpriteType::Entity, mob.asset_name())
                        .expect("loaded mob"),
                    &mut assets.warnings,
                );
            }
            for item in ItemId::ALL {
                override_image(
                    pack,
                    &format!("assets/textures/item/{}.png", item.asset_name()),
                    assets
                        .sheets
                        .sheet_mut(SpriteType::Item, item.asset_name())
                        .expect("loaded item"),
                    &mut assets.warnings,
                );
            }
            for kind in ToolKind::ALL {
                for tier in ToolTier::ALL {
                    override_image(
                        pack,
                        &format!("assets/textures/item/{}.png", kind.asset_name(tier)),
                        assets
                            .sheets
                            .sheet_mut(SpriteType::Item, kind.asset_name(tier))
                            .expect("loaded tool"),
                        &mut assets.warnings,
                    );
                }
            }
            for kind in FurnitureKind::ALL {
                override_image(
                    pack,
                    &format!("assets/textures/entity/{}.png", kind.asset_name()),
                    assets
                        .sheets
                        .sheet_mut(SpriteType::Entity, kind.asset_name())
                        .expect("loaded furniture"),
                    &mut assets.warnings,
                );
            }
        }
        Ok(assets)
    }

    fn apply_tile_metadata_pack(
        &mut self,
        pack: &ResourcePack,
        key: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let path = format!("assets/textures/tile/{key}.png");
        let image_bytes = pack
            .read(&path)?
            .ok_or_else(|| format!("{path}: missing paired PNG"))?;
        let image = Image::from_png(&image_bytes)?;
        if image.width < 16
            || image.height < 16
            || !image.width.is_multiple_of(8)
            || !image.height.is_multiple_of(8)
        {
            return Err(format!("{path}: invalid tile dimensions"));
        }
        let json: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let height = if json.get("animation").is_some_and(|v| v.is_object()) {
            image.height / 16 * 16
        } else {
            16
        };
        let image = image.crop(16, height)?;
        self.parse_tile_metadata(key, bytes, image.height)?;
        self.sheets.set(SpriteType::Tile, key, image);
        // Java treats missing/invalid connector resources as disabled metadata,
        // rather than falling back to a lower-priority connector image.
        let mut meta = self
            .tile_metadata
            .get(key)
            .expect("parsed metadata")
            .clone();
        for (name, size) in [(&mut meta.border, 24), (&mut meta.corner, 16)] {
            if let Some(resource) = name.as_deref() {
                let path = format!("assets/textures/tile/{resource}.png");
                let loaded = (|| -> Result<Image, String> {
                    let bytes = pack.read(&path)?.ok_or_else(|| format!("missing {path}"))?;
                    Image::from_png(&bytes)?.crop(size, size)
                })();
                match loaded {
                    Ok(image) => self.sheets.set(SpriteType::Tile, resource, image),
                    Err(error) => {
                        self.warnings
                            .push(format!("{}: {error}; connector disabled", pack.name));
                        *name = None;
                    }
                }
            }
        }
        self.tile_metadata.insert(key.to_owned(), meta);
        Ok(())
    }

    fn load_tile_metadata(&mut self, key: &str, bytes: &[u8]) -> Result<(), String> {
        let height = self
            .sheets
            .sheet(SpriteType::Tile, key)
            .ok_or_else(|| format!("missing metadata image {key}"))?
            .height;
        self.parse_tile_metadata(key, bytes, height)
    }

    fn parse_tile_metadata(
        &mut self,
        key: &str,
        bytes: &[u8],
        image_height: usize,
    ) -> Result<(), String> {
        use crate::gfx::sprite_animation::SpriteMeta;
        let json: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let animation = json.get("animation").filter(|value| value.is_object());
        let (frames, frame_time) = if let Some(animation) = animation {
            let time = animation
                .get("frametime")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok())
                .ok_or_else(|| format!("{key}: invalid frametime"))?;
            if time <= 0 || image_height < 16 {
                return Err(format!("{key}: invalid animation timing or height"));
            }
            ((image_height / 16) as i32, time)
        } else {
            (1, 0)
        };
        let border = json.get("border");
        let resource_key = |field| {
            border
                .and_then(|b| b.get(field))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        self.tile_metadata.insert(
            key.to_owned(),
            SpriteMeta {
                frames,
                frame_time,
                border: resource_key("key"),
                corner: resource_key("corner"),
            },
        );
        Ok(())
    }

    pub fn tile(&self, tile: Tile, data: u16) -> &Image {
        debug_assert_eq!(Tile::from_id(tile.id()), Some(tile));
        let variants = &self.tiles[tile.id() as usize];
        self.sheets
            .sheet(
                SpriteType::Tile,
                variants[tile_variant_index(tile, data, variants.len())],
            )
            .expect("loaded tile sheet")
    }

    pub fn mob(&self, mob: NaturalMob) -> &Image {
        self.sheets
            .sheet(SpriteType::Entity, mob.asset_name())
            .expect("loaded mob sheet")
    }

    pub fn item(&self, item: ItemId) -> &Image {
        self.sheets
            .sheet(SpriteType::Item, item.asset_name())
            .expect("loaded item sheet")
    }

    pub fn tool(&self, tool: ToolItem) -> &Image {
        self.sheets
            .sheet(SpriteType::Item, tool.kind.asset_name(tool.tier))
            .expect("loaded tool sheet")
    }

    pub fn sheets(&self) -> &SpriteLinker {
        &self.sheets
    }

    /// Java item.sprite: LinkedSprite(Item, key) with full-sheet default size.
    pub fn item_linked(&self, item: ItemId) -> LinkedSprite {
        LinkedSprite::new(SpriteType::Item, item.asset_name())
    }

    /// Java Furniture.sprite: LinkedSprite(Entity, key) with full-sheet default size.
    pub fn furniture_linked(&self, kind: FurnitureKind) -> LinkedSprite {
        LinkedSprite::new(SpriteType::Entity, kind.asset_name())
    }

    /// Java Mob.compileMobSpriteAnimations for the current walk frame.
    /// `direction`: 0=down, 1=up, 2=left, 3=right. `frame`: 0/1 walk step.
    pub fn mob_linked(
        &self,
        mob: NaturalMob,
        direction: u8,
        frame: usize,
        sheared: bool,
    ) -> LinkedSprite {
        let key = mob.asset_name();
        // Java Sheep cutSprites: compileMobSpriteAnimations(0, 2, "sheep") → sheetY=2 cells.
        let sheet_y = if sheared && mob == NaturalMob::Sheep {
            2
        } else {
            0
        };
        let frame = frame & 1;
        // Java compileSpriteList: setMirror(mirror).setFlip(mirror). Down/up
        // frame 1 and left both flip the 2×2 tile grid as well as each cell.
        let (cell_x, mirror) = match direction {
            1 => (2, u8::from(frame == 1)),
            2 => (4 + frame * 2, 1),
            3 => (4 + frame * 2, 0),
            _ => (0, u8::from(frame == 1)),
        };
        LinkedSprite::new(SpriteType::Entity, key)
            .set_sprite_dim(cell_x, sheet_y, 2, 2)
            .set_mirror(mirror)
            .set_flip(mirror)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_tile(
        &self,
        screen: &mut Screen,
        tile: Tile,
        data: u16,
        x: i32,
        y: i32,
        game_time: i32,
        world_enter_millis: i64,
        connected: [bool; 8],
    ) {
        let image = self.tile(tile, data);
        let variants = &self.tiles[tile.id() as usize];
        let key = variants[tile_variant_index(tile, data, variants.len())];
        let metadata = self.tile_metadata.get(key).cloned().unwrap_or_default();
        let now = animation_millis();
        let mut animations = self.tile_animations.borrow_mut();
        let animation = animations.entry(key.to_owned()).or_insert_with(|| {
            crate::gfx::sprite_animation::SpriteAnimation::new(
                metadata.frames,
                metadata.frame_time,
                game_time,
                now,
            )
        });
        let frame = animation.frame();
        let Some(border) = metadata.border.as_deref() else {
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
            animation.after_render(game_time, now, world_enter_millis);
            return;
        };
        let connection = Connection {
            border: self
                .sheets
                .sheet(SpriteType::Tile, border)
                .expect("loaded border"),
            corner: metadata.corner.as_deref().map(|key| {
                self.sheets
                    .sheet(SpriteType::Tile, key)
                    .expect("loaded corner")
            }),
            singleton: connection_names(tile).is_some_and(|(_, _, singleton)| singleton),
        };
        connection.render(screen, image, frame, (x, y), connected);
        animation.after_render(game_time, now, world_enter_millis);
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
            self.sheets.set(
                SpriteType::Entity,
                "skin.active",
                png(include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../assets/resources/textures/skins.png"
                )))?,
            );
            self.skin_row = row;
            return Ok(());
        }

        // Stage 13: reject path traversal and non-file ids for custom skins.
        if id.is_empty()
            || id.contains("..")
            || id.contains('/')
            || id.contains('\\')
            || id.contains('\0')
        {
            return Err(format!("invalid skin id {id:?}"));
        }

        let relative = format!("{id}.png");
        crate::resource::pack::validate_resource_path(&relative)?;
        let root = fs::canonicalize(game_dir.join("skins")).map_err(|error| error.to_string())?;
        let path = fs::canonicalize(root.join(relative)).map_err(|error| error.to_string())?;
        if !path.starts_with(&root) {
            return Err("custom skin escapes skins directory".to_owned());
        }
        let file = fs::File::open(&path)
            .map_err(|error| format!("cannot read skin {}: {error}", path.display()))?;
        if !file
            .metadata()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            return Err("custom skin is not a regular file".to_owned());
        }
        let bytes = crate::resource::pack::read_entry(file)?;
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
        self.sheets.set(SpriteType::Entity, "skin.active", image);
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

fn tile(name: &str) -> Result<Image, String> {
    macro_rules! bytes {
        ($file:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/assets/textures/tile/",
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
                "/../../assets/assets/textures/entity/",
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
                "/../../assets/assets/textures/item/",
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
        "rose" => bytes!("rose.png"),
        "oxeye_daisy" => bytes!("oxeye_daisy.png"),
        "cactus" => bytes!("cactus.png"),
        "sand" => bytes!("sand.png"),
        "glass" => bytes!("glass.png"),
        "glass_bottle" => bytes!("glass_bottle.png"),
        "fertilizer" => bytes!("fertilizer.png"),
        "dirt" => bytes!("dirt.png"),
        "acorn" => bytes!("acorn.png"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_connector_cannot_read_outside_folder_pack() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-connector-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let folder = root.join("resourcepacks/pack");
        std::fs::create_dir_all(folder.join("assets/textures/tile")).unwrap();
        std::fs::write(
            folder.join("pack.json"),
            br#"{"pack_format":1,"name":"Connector"}"#,
        )
        .unwrap();
        let tile = include_bytes!("../../../assets/assets/textures/tile/water.png");
        std::fs::write(folder.join("assets/textures/tile/water.png"), tile).unwrap();
        std::fs::write(root.join("resourcepacks/outside.png"), tile).unwrap();
        let (packs, warnings) = crate::resource::pack::discover(&root);
        assert!(warnings.is_empty());
        let mut assets = Assets::load(&[]).unwrap();
        assets
            .apply_tile_metadata_pack(
                &packs[0],
                "water",
                br#"{"border":{"key":"../../../../outside"},"animation":{"frametime":4}}"#,
            )
            .unwrap();
        assert!(assets.tile_metadata["water"].border.is_none());
        assert_eq!(assets.tile_metadata["water"].frame_time, 4);
        assert!(
            assets
                .warnings
                .iter()
                .any(|warning| warning.contains("invalid resource-pack path"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn every_java_item_with_a_named_sprite_has_a_real_texture() {
        let assets = Assets::load(&[]).unwrap();
        let missing = item_image("missing_item").unwrap();
        let java_placeholders = [
            ItemId::PowerGlove,
            ItemId::FarmlandItem,
            ItemId::HoleItem,
            ItemId::LavaItem,
            ItemId::PathItem,
            ItemId::WaterItem,
        ];

        for &item in ItemId::ALL {
            if !java_placeholders.contains(&item) {
                assert_ne!(
                    assets.item(item),
                    &missing,
                    "{} unexpectedly uses missing_item.png",
                    item.display_name()
                );
            }
        }
    }

    #[test]
    fn custom_skin_ids_reject_path_traversal() {
        let mut assets = Assets::load(&[]).unwrap();
        let dir = std::env::temp_dir();
        for bad in ["", "..", "a/b", "a\\b", "../escape"] {
            assert!(
                assets.select_skin(bad, &dir).is_err(),
                "expected reject for {bad:?}"
            );
        }
        // Built-ins still load without touching disk.
        assets.select_skin("minicraft.skin.paul", &dir).unwrap();
    }

    #[test]
    fn linked_sprite_factories_resolve_for_items_furniture_mobs_and_smash() {
        let assets = Assets::load(&[]).unwrap();
        // Placeholder items still produce a drawable LinkedSprite (missing art).
        let _ = assets.item_linked(ItemId::PowerGlove);
        for item in [ItemId::Wood, ItemId::Workbench, ItemId::Apple] {
            let linked = assets.item_linked(item);
            drop(linked);
        }
        for kind in [
            crate::entity::furniture::FurnitureKind::Workbench,
            crate::entity::furniture::FurnitureKind::Bed,
            crate::entity::furniture::FurnitureKind::Tnt,
            crate::entity::furniture::FurnitureKind::KnightStatue,
        ] {
            let linked = assets.furniture_linked(kind);
            drop(linked);
        }
        for species in crate::entity::mob::NaturalMob::ALL {
            let linked = assets.mob_linked(species, 0, 0, false);
            drop(linked);
            let sheared = assets.mob_linked(species, 0, 0, true);
            drop(sheared);
        }
        let smash = assets.smash();
        assert!(smash.width > 0);
        assert!(smash.height > 0);
    }
}
