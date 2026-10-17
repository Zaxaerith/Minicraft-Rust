//! World drawing adapters, split at Java Renderer level/GUI/Display boundaries.
use super::entity_render::render_entity;
use super::inventory_ui::render_container;
use super::overlay_ui::{render_sign_editor, render_world_book};
use super::player_render::render_attack_slash;
use super::*;
use crate::core::renderer::{RenderHost, Renderer};

impl World {
    pub fn render(&mut self, screen: &mut Screen, assets: &Assets, localization: &Localization) {
        self.render_with_ready(screen, assets, localization, true);
    }

    /// Java Player.getLightRadius: base 5, raised by a held furniture light.
    fn player_light_radius(&self) -> i32 {
        let mut radius = 5;
        if let Some(ActiveItem::Stack(item)) = self.player.active_item
            && let Some(kind) = furniture_for_item(item)
        {
            let held = kind.light_radius();
            if held > radius {
                radius = held;
            }
        }
        radius
    }

    pub(crate) fn render_with_ready(
        &mut self,
        screen: &mut Screen,
        assets: &Assets,
        localization: &Localization,
        ready: bool,
    ) {
        let visible = self.render_gui;
        Renderer::render(
            &mut WorldFrame {
                world: self,
                screen,
                assets,
                localization,
            },
            ready,
            visible,
        );
    }
}

struct WorldFrame<'a> {
    world: &'a mut World,
    screen: &'a mut Screen,
    assets: &'a Assets,
    localization: &'a Localization,
}
impl RenderHost for WorldFrame<'_> {
    fn render_level(&mut self) {
        self.world.render_level(self.screen, self.assets);
    }
    fn render_gui(&mut self) {
        self.world
            .render_gui_layer(self.screen, self.assets, self.localization);
    }
    fn render_display(&mut self) {
        self.world
            .render_display_layer(self.screen, self.assets, self.localization);
    }
}
impl World {
    fn render_level(&mut self, screen: &mut Screen, assets: &Assets) {
        // Java Renderer intentionally does not clamp the scroll to the map.
        let camera_x = self.player.x - WIDTH as i32 / 2;
        let camera_y = self.player.y - (HEIGHT as i32 - 8) / 2;
        let light_scale = if self.effect_active(PotionKind::Light) {
            12
        } else {
            8
        };
        let depth = self.levels[self.current_level].depth;
        let level_index = crate::gfx::lighting::java_level_index(depth);

        // Java Level.renderBackground: inclusive tile window from the unclamped scroll.
        let xo = camera_x >> 4;
        let yo = camera_y >> 4;
        let view_w = (WIDTH as i32) >> 4;
        let view_h = (HEIGHT as i32) >> 4;
        for tile_y in yo..=(view_h + yo) {
            for tile_x in xo..=(view_w + xo) {
                if tile_x < 0
                    || tile_y < 0
                    || tile_x >= self.width as i32
                    || tile_y >= self.height as i32
                {
                    continue;
                }
                let draw_x = tile_x * TILE_SIZE - camera_x;
                let draw_y = tile_y * TILE_SIZE - camera_y;
                let tile = self.levels[self.current_level].tiles
                    [tile_x as usize + tile_y as usize * self.width];
                let data = self.levels[self.current_level].data
                    [tile_x as usize + tile_y as usize * self.width];
                for underlay in underlay_layers(tile, data, self.levels[self.current_level].depth)
                    .into_iter()
                    .flatten()
                {
                    assets.render_tile(
                        screen,
                        underlay,
                        0,
                        draw_x,
                        draw_y,
                        self.clock.get().game_time,
                        self.animation_enter_millis,
                        connection_mask(
                            &self.levels[self.current_level].tiles,
                            self.width,
                            self.height,
                            tile_x as usize,
                            tile_y as usize,
                            underlay,
                        ),
                    );
                }
                let connected = connection_mask(
                    &self.levels[self.current_level].tiles,
                    self.width,
                    self.height,
                    tile_x as usize,
                    tile_y as usize,
                    tile,
                );
                assets.render_tile(
                    screen,
                    tile,
                    data,
                    draw_x,
                    draw_y,
                    self.clock.get().game_time,
                    self.animation_enter_millis,
                    connected,
                );
            }
        }

        // Java Level.renderLight: entities first (x-1, y-4), then tiles in an
        // 8-tile margin around the viewport. Player is not in the arena here.
        let light_radius = self.player_light_radius();
        if light_radius > 0 {
            screen.render_light(
                self.player.x - 1 - camera_x,
                self.player.y - 4 - camera_y,
                light_radius * light_scale,
            );
        }
        for (x, y, radius) in self.levels[self.current_level].entities.light_sources() {
            screen.render_light(x - 1 - camera_x, y - 4 - camera_y, radius * light_scale);
        }
        let light_xo = camera_x >> 4;
        let light_yo = camera_y >> 4;
        let light_view_w = (WIDTH as i32 + 15) >> 4;
        let light_view_h = (HEIGHT as i32 + 15) >> 4;
        let margin = 8;
        for tile_y in (light_yo - margin)..=(light_view_h + light_yo + margin) {
            for tile_x in (light_xo - margin)..=(light_view_w + light_xo + margin) {
                if tile_x < 0
                    || tile_y < 0
                    || tile_x >= self.width as i32
                    || tile_y >= self.height as i32
                {
                    continue;
                }
                let tile = self.levels[self.current_level].tiles
                    [tile_x as usize + tile_y as usize * self.width];
                let radius = tile.light_radius();
                if radius > 0 {
                    screen.render_light(
                        (tile_x << 4) + 8 - camera_x,
                        (tile_y << 4) + 8 - camera_y,
                        radius * light_scale,
                    );
                }
            }
        }

        let mut entities = self.levels[self.current_level]
            .entities
            .entities()
            .iter()
            .collect::<Vec<_>>();
        entities.sort_by_key(|entity| (entity.y, entity.id));
        let tiles = &self.levels[self.current_level].tiles;
        let width = self.width;
        let height = self.height;
        let entity_tick = self.tick;
        for entity in entities
            .iter()
            .copied()
            .take_while(|entity| entity.y <= self.player.y)
        {
            render_entity(
                screen,
                assets,
                entity,
                camera_x,
                camera_y,
                tiles,
                width,
                height,
                entity_tick,
            );
        }

        if self.sleeping == 0 {
            let player_swimming = self.player_is_swimming();
            let on_water = self.tile_at_pixel(self.player.x, self.player.y) == Tile::Water;
            // Java Player.render: swimming sinks the sprite and draws liquid overlay.
            let player_x = self.player.x - 8 - camera_x;
            let mut player_y = self.player.y - 11 - camera_y;
            if player_swimming {
                player_y += 4;
                // Java tickTime / 8 % 2
                let frame = ((self.tick / 8) % 2) as usize;
                let (sheet_x, left_flip, right_flip) = if on_water {
                    (5usize, false, true)
                } else {
                    (6usize, true, false)
                };
                let source_x = sheet_x * 8;
                let source_y = frame * 8;
                screen.blit_region(
                    assets.hud(),
                    player_x,
                    player_y + 3,
                    source_x,
                    source_y,
                    8,
                    8,
                    left_flip,
                );
                screen.blit_region(
                    assets.hud(),
                    player_x + 8,
                    player_y + 3,
                    source_x,
                    source_y,
                    8,
                    8,
                    right_flip,
                );
            }
            let (source_x, flip) =
                player_sprite_frame(self.player.direction, self.player.walk_distance);
            // Java Player: FurnitureItem uses carrySprites (skin rows +2 tiles / +16px).
            let carrying_furniture = matches!(
                self.player.active_item,
                Some(ActiveItem::Stack(item)) if furniture_for_item(item).is_some()
            );
            let skin_row = if carrying_furniture {
                assets.skin_row + 16
            } else {
                assets.skin_row
            };
            // Java playerHurtTime = 30; white flash while hurtTime > 20.
            let hurt = self.player.hurt_time > 20;
            let tint = if hurt { 0x00FF_FFFFu32 } else { 0 };
            let shirt = self
                .player
                .imported_shirt_color
                .map(|color| color as u32)
                .or_else(|| crate::item::clothing::player_color(self.player.clothing));
            screen.render_sprite(
                assets.skin(),
                player_x,
                player_y,
                crate::gfx::sprite::SpriteRender {
                    source_x,
                    source_y: skin_row,
                    width: 16,
                    height: if player_swimming { 8 } else { 16 },
                    mirror_x: flip,
                    white_tint: shirt,
                    color: tint,
                    full_bright: hurt,
                    ..Default::default()
                },
            );
            if self.player.attack_time > 0 {
                render_attack_slash(
                    screen,
                    assets.hud(),
                    player_x,
                    player_y,
                    self.player.attack_direction,
                );
                let image = self.player.attack_item.and_then(|active| match active {
                    ActiveItem::Stack(item) => Some(assets.item(item)),
                    ActiveItem::Slot { .. } => self
                        .player
                        .stack_for_active(active)
                        .map(|s| assets.item_stack(s)),
                    ActiveItem::Tool(index) => self
                        .player
                        .inventory
                        .tools()
                        .get(index)
                        .map(|tool| assets.tool(*tool)),
                });
                let (held_x, held_y, flip_item) = match self.player.attack_direction {
                    Direction::Up => (player_x + 4, player_y - 4, true),
                    Direction::Left => (player_x - 4, player_y + 4, true),
                    Direction::Right => (player_x + 12, player_y + 4, false),
                    Direction::Down => (player_x + 4, player_y + 12, false),
                };
                if let Some(image) = image {
                    screen.blit_region(
                        image,
                        held_x,
                        held_y,
                        0,
                        0,
                        image.width,
                        image.height,
                        flip_item,
                    );
                }
            }
            // Java Player.render: TileItem placement frame (hud tile 3,2 four corners).
            if !player_swimming
                && let Some(ActiveItem::Stack(item)) = self.player.active_item
                && crate::world::tile::is_tile_item(item)
            {
                let (tile_x, tile_y) = self.interaction_tile();
                let frame_x = tile_x * TILE_SIZE;
                let frame_y = tile_y * TILE_SIZE;
                let source_x = 3 * 8;
                let source_y = 2 * 8;
                // Java Player.render bits: 0,1,2,3 = none, X, Y, XY mirrors.
                screen.blit_region(
                    assets.hud(),
                    frame_x - camera_x,
                    frame_y - camera_y,
                    source_x,
                    source_y,
                    8,
                    8,
                    false,
                );
                screen.blit_region(
                    assets.hud(),
                    frame_x + 8 - camera_x,
                    frame_y - camera_y,
                    source_x,
                    source_y,
                    8,
                    8,
                    true,
                );
                screen.blit_region_override(
                    assets.hud(),
                    source_x,
                    source_y,
                    8,
                    8,
                    frame_x - camera_x,
                    frame_y + 8 - camera_y,
                    false,
                    true,
                    0,
                    false,
                );
                screen.blit_region_override(
                    assets.hud(),
                    source_x,
                    source_y,
                    8,
                    8,
                    frame_x + 8 - camera_x,
                    frame_y + 8 - camera_y,
                    true,
                    true,
                    0,
                    false,
                );
            }
            // Java Player.render: FurnitureItem always draws held furniture at yo-4
            // (entity y), then furniture.render at y-8 → screen Y = yo - 12 = y - 23.
            // Do not gate on swimming: Java has no isSwimming check here.
            if let Some(ActiveItem::Stack(item)) = self.player.active_item
                && let Some(kind) = furniture_for_item(item)
            {
                let held_y = if player_swimming {
                    // Java applies yo += 4 while swimming before furniture.y = yo - 4.
                    self.player.y - 11 + 4 - 12 - camera_y
                } else {
                    player_y - 12
                };
                let sprite = if let Some(color) = crate::item::dye::DyeColor::from_bed_item(item) {
                    assets.colored_bed_linked(color)
                } else {
                    assets.furniture_linked(kind)
                };
                sprite.render(
                    assets.sheets(),
                    screen,
                    self.player.x - 8 - camera_x,
                    held_y,
                    0,
                    false,
                    0,
                );
            }
        }
        for entity in entities
            .iter()
            .copied()
            .skip_while(|entity| entity.y <= self.player.y)
        {
            render_entity(
                screen,
                assets,
                entity,
                camera_x,
                camera_y,
                tiles,
                width,
                height,
                entity_tick,
            );
        }

        // Java Renderer: skip overlay on creative, or surface during the day phase.
        if self.mode != GameMode::Creative
            && !crate::gfx::lighting::should_skip_overlay(
                level_index,
                self.clock.get().day_tick as i32,
            )
        {
            let past_day_one = self.days > 1 || self.clock.get().game_time > 65_000;
            screen.overlay(
                level_index,
                camera_x,
                camera_y,
                self.clock.get().day_tick as i32,
                past_day_one,
            );
        }
    }
    fn render_gui_layer(
        &mut self,
        screen: &mut Screen,
        assets: &Assets,
        localization: &Localization,
    ) {
        let depth = self.levels[self.current_level].depth;
        render_hud(
            screen,
            assets,
            &self.player,
            self.mode,
            self.seed,
            depth,
            self.clock.get().day_tick,
            self.days,
        );
        if self.mode == GameMode::Score {
            render_score_overlay(
                screen,
                assets,
                self.score_ticks,
                self.score,
                self.score_multiplier,
            );
        }
        let mut potion_offset = 0;
        if !self.pause.is_rendering()
            && self.show_quests
            && (self.tutorials_enabled || self.quests_enabled)
        {
            let inventory = self.inventory_progress();
            let progress = if self.tutorials_enabled {
                self.progress.current_tutorial().map(|step| {
                    format!(
                        "TUTORIAL {}",
                        short_progress_id(&step.id).to_ascii_uppercase()
                    )
                })
            } else {
                None
            }
            .or_else(|| {
                self.quests_enabled.then(|| {
                    self.progress.current_quest(&inventory).map(|quest| {
                        format!(
                            "QUEST {}",
                            short_progress_id(&quest.id).to_ascii_uppercase()
                        )
                    })
                })?
            });
            if let Some(progress) = progress {
                render_quest_overlay(screen, assets, &progress);
                potion_offset = 33;
            }
        }
        render_potion_overlay(screen, assets, &self.player, potion_offset);
        if let Some((health, max_health)) = self.current_boss_health() {
            render_boss_bar(
                screen,
                assets,
                health,
                max_health,
                if depth == 1 {
                    "Air wizard"
                } else {
                    "Obsidian Knight"
                },
            );
        }
        if self.story_complete {
            screen.rect(WIDTH as i32 - 82, 33, 80, 10, 0x101018);
            screen.text(assets.font(), "VICTORY", WIDTH as i32 - 76, 34);
        }
        self.render_status(screen, assets, localization);
    }
    fn render_display_layer(
        &mut self,
        screen: &mut Screen,
        assets: &Assets,
        localization: &Localization,
    ) {
        if self.inventory_open {
            if let Some(chest_id) = self.container_id {
                let empty = Inventory::new(27);
                let chest_slots = self.levels[self.current_level]
                    .entities
                    .death_chest_storage(chest_id)
                    .unwrap_or(&empty);
                render_container(
                    screen,
                    assets,
                    &self.player,
                    chest_slots,
                    self.inventory_item_selection,
                    self.inventory_selection,
                    self.inventory_pane,
                );
            } else {
                render_inventory(
                    screen,
                    assets,
                    &self.player,
                    &self.progress,
                    self.inventory_item_selection,
                    self.inventory_selection,
                    self.inventory_pane,
                    self.crafting_station,
                    &self.crafting_order,
                    self.personal_crafting,
                    self.mode,
                );
            }
        }
        if self.pause.is_rendering() {
            render_pause(screen, assets, localization, self);
        }
        if let Some((book, page)) = self.book_open {
            render_world_book(screen, assets, book, page);
        }
        if let Some(editor) = &self.sign_editor {
            render_sign_editor(screen, assets, &editor.text);
        }
        self.render_level_transition(screen, assets);
        if self.death_open {
            self.render_death_menu(screen, assets, localization);
        }
        if self.game_over && self.end_game_visible() {
            if self.mode == GameMode::Score {
                self.render_end_game(screen, assets, localization);
                return;
            }
            screen.rect(54, 76, WIDTH as i32 - 108, 74, 0x101018);
            screen.centered_text(
                assets.font(),
                if self.mode == GameMode::Hardcore {
                    "HARDCORE WORLD ENDED"
                } else {
                    "TIME IS UP"
                },
                91,
            );
            screen.centered_text(assets.font(), "SELECT TO RETURN", 132);
        }
    }
}
