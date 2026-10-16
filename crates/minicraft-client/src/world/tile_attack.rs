//! Per-tile attack, tool-on-tile, placement, and harvest (Java Tile. hurt/interact groups).
use super::{
    ActiveItem, GameMode, ItemId, ItemStack, ProgressEvent, SignEditor, SoundEffect, TILE_SIZE,
    Tile, ToolKind, ToolTier, World, damage_tile, entity, tile_placement, tile_progress_name,
};

impl World {
    /// Java attack path after mob/TNT checks: tool terrain, placement, and tile damage.
    pub(super) fn attack_target_tile(&mut self, tile_x: i32, tile_y: i32, index: usize) {
        if let Some(ActiveItem::Stack(item)) = self.player.active_item {
            let target = self.levels[self.current_level].tiles[index];
            if item == ItemId::WateringCan {
                if target == Tile::Water {
                    self.player.watering_content = crate::item::WATERING_CAN_CAPACITY;
                } else if self.player.watering_content > 0 {
                    self.player.watering_content -= 1;
                    if matches!(
                        target,
                        Tile::Wheat
                            | Tile::Potato
                            | Tile::Tomato
                            | Tile::Carrot
                            | Tile::HeavenlyBerries
                            | Tile::HellishBerries
                    ) {
                        let data = self.levels[self.current_level].data[index];
                        self.levels[self.current_level].data[index] =
                            crate::item::watering_can::water_crop_data(data);
                    } else if target == Tile::Farmland {
                        self.levels[self.current_level].data[index] = 7;
                    }
                }
                return;
            }
            if let Some(fishing_level) = item.fishing_level() {
                if target == Tile::Water
                    && self.tile_at_pixel(self.player.x, self.player.y) != Tile::Water
                {
                    self.player.fishing_level = Some(fishing_level);
                    self.player.fishing_ticks = 0;
                }
                return;
            }
            if self.use_bucket(item, index) {
                return;
            }
            if matches!(item, ItemId::Fertilizer | ItemId::ArcaneFertilizer)
                && crate::level::tile::farming::is_crop(target)
            {
                let data = self.levels[self.current_level].data[index];
                let arcane = item == ItemId::ArcaneFertilizer;
                let current = crate::level::tile::farming::crop::fertilization(data);
                let amount = crate::level::tile::farming::crop::fertilizer_amount(current, arcane);
                self.levels[self.current_level].data[index] =
                    crate::level::tile::farming::crop::apply_fertilizer(data, amount, arcane);
                self.consume_active_stack(item);
                return;
            }
            if let Some((placed, data)) = tile_placement(item, target) {
                self.levels[self.current_level].tiles[index] = placed;
                self.levels[self.current_level].data[index] = data;
                self.consume_active_stack(item);
                self.update_progress(ProgressEvent::PlacedTile {
                    tile: tile_progress_name(placed),
                });
                if matches!(
                    placed,
                    Tile::Wheat
                        | Tile::Potato
                        | Tile::Tomato
                        | Tile::Carrot
                        | Tile::HeavenlyBerries
                        | Tile::HellishBerries
                ) {
                    self.unlock_achievement("minicraft.achievement.plant_seed");
                }
                return;
            }
        }

        if matches!(self.player.active_item, Some(ActiveItem::Stack(_))) {
            return;
        }

        if self.mode == GameMode::Creative {
            self.player.attack_time = 5;
            if self.creative_break_tile(index) {
                return;
            }
        }

        let target = self.levels[self.current_level].tiles[index];
        let data = self.levels[self.current_level].data[index];
        if matches!(target, Tile::Flower | Tile::Flower230) {
            let flower = crate::level::tile::flower::harvested_item(target, data);
            if target == Tile::Flower230 && flower.is_none() {
                return;
            }
            let tool = self.active_tool();
            let shovel = tool.is_some_and(|tool| tool.kind == ToolKind::Shovel);
            if shovel {
                if self.pay_tool_terrain_damage(ToolKind::Shovel, 2).is_none() {
                    return;
                }
            } else {
                // Java Shears cannot attack tiles through FlowerTile.hurt.
                if tool.is_some_and(|tool| tool.kind == ToolKind::Shears) {
                    return;
                }
                self.player.attack_time = 5;
                self.pay_basic_attack_tool_durability();
            }
            // Capture the 2.3 variant before replacing its tile. Upstream's
            // shovel path reads the reset data afterward, yielding daisies.
            let drops = if target == Tile::Flower {
                // Keep 2.2.4's two independent flower drops. Shape / mirror bits
                // in an old world do not represent a particular flower item.
                [
                    ItemStack::new(
                        ItemId::Flower,
                        if shovel {
                            1
                        } else {
                            self.random.next_int(2) as u16
                        },
                    ),
                    ItemStack::new(
                        ItemId::Rose,
                        if shovel {
                            1
                        } else {
                            self.random.next_int(2) as u16
                        },
                    ),
                ]
            } else {
                [
                    ItemStack::new(flower.expect("validated flower variant"), 1),
                    ItemStack::new(ItemId::Flower, 0),
                ]
            };
            self.levels[self.current_level].tiles[index] = Tile::Grass;
            self.levels[self.current_level].data[index] = 0;
            for stack in drops {
                if stack.count > 0 {
                    self.levels[self.current_level].entities.spawn_item(
                        stack,
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
            }
            self.sound(SoundEffect::MonsterHurt);
            return;
        }
        if let Some(item) = crate::item::wool::item_for_tile(target) {
            if self.pay_tool_terrain_damage(ToolKind::Shears, 3).is_none() {
                return;
            }
            self.levels[self.current_level].tiles[index] = Tile::Hole;
            self.levels[self.current_level].data[index] = 0;
            self.levels[self.current_level].entities.spawn_item(
                ItemStack::new(item, 1),
                tile_x * TILE_SIZE + 8,
                tile_y * TILE_SIZE + 8,
            );
            self.sound(SoundEffect::MonsterHurt);
            return;
        }

        if let Some(tool) = self.active_tool() {
            let target = self.levels[self.current_level].tiles[index];
            let replacement = match (tool.kind, target) {
                (ToolKind::Shovel, Tile::Dirt) => Some((Tile::Hole, Some(ItemId::Dirt))),
                (ToolKind::Shovel, Tile::Grass) => Some((Tile::Dirt, None)),
                (ToolKind::Shovel, Tile::Sand) => Some((Tile::Hole, Some(ItemId::Sand))),
                (ToolKind::Shovel, Tile::Cloud) => Some((Tile::InfiniteFall, Some(ItemId::Cloud))),
                (ToolKind::Hoe, Tile::Dirt | Tile::Grass) => Some((Tile::Farmland, None)),
                (ToolKind::Pickaxe, Tile::Grass) => Some((Tile::Path, None)),
                _ => None,
            };
            if let Some((tile, drop)) = replacement {
                let cost = if target == Tile::Cloud { 5 } else { 4 };
                if self.pay_tool_terrain_damage(tool.kind, cost).is_none() {
                    return;
                }
                self.levels[self.current_level].tiles[index] = tile;
                self.levels[self.current_level].data[index] = 0;
                self.levels[self.current_level].entities.spawn_particle(
                    entity::ParticleKind::Smash,
                    tile_x * TILE_SIZE,
                    tile_y * TILE_SIZE,
                );
                self.sound(SoundEffect::MonsterHurt);
                if tool.kind == ToolKind::Hoe {
                    self.update_progress(ProgressEvent::ItemUsedOnTile {
                        item: format!(
                            "{} hoe",
                            if tool.tier == ToolTier::Rock {
                                "stone"
                            } else {
                                tool.tier.display_name()
                            }
                            .to_ascii_lowercase()
                        ),
                        tile: tile_progress_name(target),
                    });
                }
                if let Some(item) = drop {
                    let count = if item == ItemId::Cloud {
                        (self.random.next_int(3) + 1) as u16
                    } else {
                        1
                    };
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(item, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                } else if target == Tile::Grass {
                    let seed = if tool.kind == ToolKind::Hoe {
                        (self.random.next_int(5) != 0).then_some(ItemId::WheatSeeds)
                    } else if tool.kind == ToolKind::Shovel {
                        (self.random.next_int(5) == 0).then_some(ItemId::GrassSeeds)
                    } else {
                        None
                    };
                    if let Some(seed) = seed {
                        self.levels[self.current_level].entities.spawn_item(
                            ItemStack::new(seed, 1),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                }
                return;
            }
        }
        let attacked_tile = self.levels[self.current_level].tiles[index];
        if matches!(
            attacked_tile,
            Tile::Tree
                | Tile::Cactus
                | Tile::Rock
                | Tile::HardRock
                | Tile::WoodWall
                | Tile::StoneWall
                | Tile::ObsidianWall
                | Tile::BossWall
                | Tile::Sign
                | Tile::IronOre
                | Tile::GoldOre
                | Tile::GemOre
                | Tile::LapisOre
                | Tile::CloudOre
                | Tile::Wheat
                | Tile::Potato
                | Tile::Tomato
                | Tile::Carrot
                | Tile::HeavenlyBerries
                | Tile::HellishBerries
        ) {
            self.levels[self.current_level].entities.spawn_particle(
                entity::ParticleKind::Smash,
                tile_x * TILE_SIZE,
                tile_y * TILE_SIZE,
            );
            self.sound(SoundEffect::MonsterHurt);
        }
        match self.levels[self.current_level].tiles[index] {
            Tile::Tree => {
                let tool_damage = self.pay_tool_terrain_damage(ToolKind::Axe, 4);
                let damage = tool_damage.unwrap_or_else(|| (self.random.next_int(3) + 1) as u16);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let total = self.levels[self.current_level].data[index] + damage;
                if total >= 20 {
                    self.levels[self.current_level].tiles[index] = Tile::Grass;
                    self.levels[self.current_level].data[index] = 0;
                    let wood = (self.random.next_int(3) + 1) as u16;
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(ItemId::Wood, wood),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                    let acorns = self.random.next_int(3) as u16;
                    if acorns > 0 {
                        self.levels[self.current_level].entities.spawn_item(
                            ItemStack::new(ItemId::Acorn, acorns),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                    self.unlock_achievement("minicraft.achievement.woodcutter");
                } else {
                    self.levels[self.current_level].data[index] = total;
                }
            }
            Tile::WoodDoor | Tile::StoneDoor | Tile::ObsidianDoor => {
                // Java DoorTile.interact: matching tool pays stamina and removes the door.
                let door = self.levels[self.current_level].tiles[index];
                let required = match door {
                    Tile::WoodDoor => ToolKind::Axe,
                    Tile::StoneDoor => ToolKind::Pickaxe,
                    Tile::ObsidianDoor => ToolKind::Pickaxe,
                    _ => unreachable!(),
                };
                if let Some(tool) = self.active_tool()
                    && tool.kind == required
                    && (self.mode == GameMode::Creative
                        || self.pay_tool_terrain_damage(required, 4).is_some())
                {
                    let item = match door {
                        Tile::WoodDoor => ItemId::WoodDoor,
                        Tile::StoneDoor => ItemId::StoneDoor,
                        _ => ItemId::ObsidianDoor,
                    };
                    let floor = match door {
                        Tile::WoodDoor => Tile::WoodFloor,
                        Tile::StoneDoor => Tile::StoneFloor,
                        _ => Tile::ObsidianFloor,
                    };
                    self.levels[self.current_level].tiles[index] = floor;
                    self.levels[self.current_level].data[index] = 0;
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(item, 1),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                    self.sound(SoundEffect::MonsterHurt);
                    return;
                }
                // Empty hand / non-tool: Java DoorTile.hurt toggles open/closed.
                self.levels[self.current_level].data[index] ^= 1;
            }
            Tile::BossDoor => {
                if self.mode == GameMode::Creative || self.obsidian_knight_defeated {
                    self.levels[self.current_level].data[index] ^= 1;
                } else {
                    self.add_notification("minicraft.notification.defeat_obsidian_knight_first");
                }
            }
            Tile::Cactus => {
                let bare_damage = (self.random.next_int(3) + 1) as u16;
                self.player.attack_time = 5;
                self.pay_basic_attack_tool_durability();
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        bare_damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                if damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    bare_damage,
                    10,
                    Tile::Sand,
                ) {
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(ItemId::Cactus, (self.random.next_int(3) + 2) as u16),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
            }
            Tile::Wheat
            | Tile::Potato
            | Tile::Tomato
            | Tile::Carrot
            | Tile::HeavenlyBerries
            | Tile::HellishBerries => {
                self.player.attack_time = 5;
                self.pay_basic_attack_tool_durability();
                let crop = self.levels[self.current_level].tiles[index];
                let data = self.levels[self.current_level].data[index];
                let mature = (data >> 3) & 7 == 7;
                self.levels[self.current_level].tiles[index] = Tile::Farmland;
                self.levels[self.current_level].data[index] = data & 7;
                let (produce, seed) = match crop {
                    Tile::Wheat => (ItemId::Wheat, Some(ItemId::WheatSeeds)),
                    Tile::Potato => (ItemId::Potato, None),
                    Tile::Tomato => (ItemId::Tomato, Some(ItemId::TomatoSeeds)),
                    Tile::Carrot => (ItemId::Carrot, None),
                    Tile::HeavenlyBerries => (ItemId::HeavenlyBerries, None),
                    Tile::HellishBerries => (ItemId::HellishBerries, None),
                    _ => unreachable!(),
                };
                if let Some(seed) = seed {
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(seed, 1),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
                if mature || seed.is_none() {
                    let count = if mature {
                        (self.random.next_int(3) + 2) as u16
                    } else {
                        1
                    };
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(produce, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
                if mature {
                    let crop_score = (self.random.next_int(5) + 1) as u32;
                    self.add_score(crop_score, 0);
                }
            }
            Tile::Rock => {
                let tool_damage = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 5);
                let used_pickaxe = tool_damage.is_some();
                let damage = tool_damage.unwrap_or_else(|| (self.random.next_int(3) + 1) as u16);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let level = &mut self.levels[self.current_level];
                if damage_tile(level, index, damage, 50, Tile::Dirt) {
                    let count = if used_pickaxe {
                        (self.random.next_int(3) + 2) as u16
                    } else {
                        1
                    };
                    level.entities.spawn_item(
                        ItemStack::new(ItemId::Stone, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                    let coal_max = if self.difficulty == 2 { 1 } else { 2 };
                    let coal = if used_pickaxe {
                        self.random.next_int(coal_max + 1) as u16
                    } else {
                        0
                    };
                    if coal > 0 {
                        level.entities.spawn_item(
                            ItemStack::new(ItemId::Coal, coal),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                }
            }
            Tile::HardRock => {
                let gem_pickaxe = self
                    .active_tool()
                    .is_some_and(|tool| tool.kind == ToolKind::Pickaxe && tool.tier.level() == 4);
                if !gem_pickaxe {
                    if self.active_tool().is_some() {
                        // Java HardRockTile adds this key literally, without localization.
                        self.notifications
                            .add("minicraft.notification.gem_pickaxe_required");
                    }
                    self.levels[self.current_level]
                        .entities
                        .spawn_text_particle(
                            "0".to_owned(),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                    return;
                }
                let Some(damage) = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 2) else {
                    return;
                };
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let level = &mut self.levels[self.current_level];
                if damage_tile(level, index, damage, 200, Tile::Dirt) {
                    level.entities.spawn_item(
                        ItemStack::new(ItemId::Stone, (self.random.next_int(3) + 1) as u16),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                    if self.random.next_bool() {
                        level.entities.spawn_item(
                            ItemStack::new(ItemId::Coal, 1),
                            tile_x * TILE_SIZE + 8,
                            tile_y * TILE_SIZE + 8,
                        );
                    }
                }
            }
            Tile::WoodWall | Tile::StoneWall | Tile::ObsidianWall => {
                if self.levels[self.current_level].tiles[index] == Tile::ObsidianWall
                    && self.levels[self.current_level].depth == -3
                    && !self.air_wizard_defeated
                    && self.mode != GameMode::Creative
                {
                    self.add_notification("minicraft.notification.defeat_air_wizard_first");
                    return;
                }
                let required = if self.levels[self.current_level].tiles[index] == Tile::WoodWall {
                    ToolKind::Axe
                } else {
                    ToolKind::Pickaxe
                };
                let tool_damage = self.pay_tool_terrain_damage(required, 4);
                let damage = tool_damage.unwrap_or(0);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let replacement = match self.levels[self.current_level].tiles[index] {
                    Tile::WoodWall => Tile::WoodFloor,
                    Tile::StoneWall => Tile::StoneFloor,
                    _ => Tile::ObsidianFloor,
                };
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    100,
                    replacement,
                );
            }
            Tile::BossWall => {
                if !self.obsidian_knight_defeated && self.mode != GameMode::Creative {
                    self.add_notification("minicraft.notification.defeat_obsidian_knight_first");
                    return;
                }
                let Some(damage) = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 4) else {
                    return;
                };
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    100,
                    Tile::BossFloor,
                );
            }
            Tile::Sign => {
                let axe = self
                    .active_tool()
                    .is_some_and(|tool| tool.kind == ToolKind::Axe);
                if !axe {
                    self.sign_editor = Some(SignEditor {
                        level: self.current_level,
                        tile: index,
                        text: self.signs[self.current_level]
                            .get(&index)
                            .cloned()
                            .unwrap_or_default(),
                    });
                    return;
                }
                if self.pay_tool_terrain_damage(ToolKind::Axe, 4).is_none() {
                    return;
                }
                let underlying = Tile::from_id(self.levels[self.current_level].data[index] as u8)
                    .unwrap_or(Tile::Dirt);
                self.levels[self.current_level].tiles[index] = underlying;
                self.levels[self.current_level].data[index] = 0;
                self.signs[self.current_level].remove(&index);
                self.levels[self.current_level].entities.spawn_item(
                    ItemStack::new(ItemId::Sign, 1),
                    tile_x * TILE_SIZE + 8,
                    tile_y * TILE_SIZE + 8,
                );
            }
            Tile::IronOre | Tile::GoldOre | Tile::GemOre | Tile::LapisOre | Tile::CloudOre => {
                let ore_tile = self.levels[self.current_level].tiles[index];
                let tool_damage = self.pay_tool_terrain_damage(ToolKind::Pickaxe, 6);
                let damage = tool_damage.unwrap_or(0);
                if tool_damage.is_none() {
                    self.player.attack_time = 5;
                    self.pay_basic_attack_tool_durability();
                }
                self.levels[self.current_level]
                    .entities
                    .spawn_text_particle(
                        damage.to_string(),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                let replacement = if ore_tile == Tile::CloudOre {
                    Tile::Cloud
                } else {
                    Tile::Dirt
                };
                let health = (self.random.next_int(10) * 4 + 20) as u16;
                let broken = damage_tile(
                    &mut self.levels[self.current_level],
                    index,
                    damage,
                    health,
                    replacement,
                );
                let count = self.random.next_int(2) as u16 + if broken { 2 } else { 0 };
                if count > 0 {
                    let item = match ore_tile {
                        Tile::IronOre => ItemId::IronOre,
                        Tile::GoldOre => ItemId::GoldOre,
                        Tile::GemOre => ItemId::Gem,
                        Tile::LapisOre => ItemId::Lapis,
                        Tile::CloudOre => ItemId::CloudOre,
                        _ => unreachable!(),
                    };
                    self.levels[self.current_level].entities.spawn_item(
                        ItemStack::new(item, count),
                        tile_x * TILE_SIZE + 8,
                        tile_y * TILE_SIZE + 8,
                    );
                }
            }
            _ => {}
        }
    }
}
