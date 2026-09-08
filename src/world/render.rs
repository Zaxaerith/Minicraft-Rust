//! World drawing adapters, split at Java Renderer level/GUI/Display boundaries.
use super::*;
use minicraft_client::core::renderer::{RenderHost, Renderer};

impl World {
    pub fn render(&mut self, screen: &mut Screen, assets: &Assets, localization: &Localization) {
        self.render_with_ready(screen, assets, localization, true);
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
        let map_width = self.width as i32 * TILE_SIZE;
        let map_height = self.height as i32 * TILE_SIZE;
        let camera_x =
            (self.player.x - WIDTH as i32 / 2).clamp(0, (map_width - WIDTH as i32).max(0));
        let camera_y =
            (self.player.y - (HEIGHT as i32 - 8) / 2).clamp(0, (map_height - HEIGHT as i32).max(0));
        let first_x = camera_x / TILE_SIZE;
        let first_y = camera_y / TILE_SIZE;
        let last_x = ((camera_x + WIDTH as i32) / TILE_SIZE + 1).min(self.width as i32);
        let last_y = ((camera_y + HEIGHT as i32) / TILE_SIZE + 1).min(self.height as i32);
        let light_scale = if self.effect_active(PotionKind::Light) {
            12
        } else {
            8
        };
        let mut lights = vec![(
            self.player.x - camera_x,
            self.player.y - camera_y,
            5 * light_scale,
        )];
        lights.extend(
            self.levels[self.current_level]
                .entities
                .light_sources()
                .map(|(x, y, radius)| (x - camera_x, y - camera_y, radius * light_scale)),
        );

        for tile_y in first_y..last_y {
            for tile_x in first_x..last_x {
                let tile = self.levels[self.current_level].tiles
                    [tile_x as usize + tile_y as usize * self.width];
                let data = self.levels[self.current_level].data
                    [tile_x as usize + tile_y as usize * self.width];
                let image = assets.tile(tile, data);
                let frame_count = (image.height / 16).max(1);
                let frame = (self.tick as usize / 12) % frame_count;
                for underlay in underlay_layers(tile, data, self.levels[self.current_level].depth)
                    .into_iter()
                    .flatten()
                {
                    let underlay_image = assets.tile(underlay, 0);
                    let underlay_frames = (underlay_image.height / 16).max(1);
                    assets.render_tile(
                        screen,
                        underlay,
                        0,
                        tile_x * TILE_SIZE - camera_x,
                        tile_y * TILE_SIZE - camera_y,
                        (self.tick as usize / 12) % underlay_frames,
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
                    tile_x * TILE_SIZE - camera_x,
                    tile_y * TILE_SIZE - camera_y,
                    frame,
                    connected,
                );
                let light_radius = tile.light_radius();
                if light_radius > 0 {
                    lights.push((
                        tile_x * TILE_SIZE + 8 - camera_x,
                        tile_y * TILE_SIZE + 8 - camera_y,
                        light_radius * light_scale,
                    ));
                }
            }
        }

        let mut entities = self.levels[self.current_level]
            .entities
            .entities()
            .iter()
            .collect::<Vec<_>>();
        entities.sort_by_key(|entity| (entity.y, entity.id));
        for entity in entities
            .iter()
            .copied()
            .take_while(|entity| entity.y <= self.player.y)
        {
            render_entity(screen, assets, entity, camera_x, camera_y, self.tick);
        }

        if self.sleeping == 0 {
            let (source_x, flip) =
                player_sprite_frame(self.player.direction, self.player.walk_distance);
            let player_x = self.player.x - 8 - camera_x;
            let player_y = self.player.y - 11 - camera_y;
            screen.blit_region(
                &assets.skin,
                player_x,
                player_y,
                source_x,
                assets.skin_row,
                16,
                16,
                flip,
            );
            if self.player.attack_time > 0 {
                render_attack_slash(
                    screen,
                    &assets.hud,
                    player_x,
                    player_y,
                    self.player.attack_direction,
                );
                let image = self.player.attack_item.and_then(|active| match active {
                    ActiveItem::Stack(item) => Some(assets.item(item)),
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
        }
        for entity in entities
            .iter()
            .copied()
            .skip_while(|entity| entity.y <= self.player.y)
        {
            render_entity(screen, assets, entity, camera_x, camera_y, self.tick);
        }

        let depth = self.levels[self.current_level].depth;
        let darkness = if self.mode == GameMode::Creative {
            0
        } else if depth == 0 {
            surface_darkness(self.clock.get().day_tick)
        } else {
            176
        };
        screen.darken_with_lights(&lights, darkness);
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
            screen.text(&assets.font, "VICTORY", WIDTH as i32 - 76, 34);
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
            render_inventory(
                screen,
                assets,
                &self.player,
                self.inventory_item_selection,
                self.inventory_selection,
                self.inventory_pane,
                self.crafting_station,
                &self.crafting_order,
                self.personal_crafting,
                self.mode,
            );
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
        if self.game_over && self.end_game_visible() {
            if self.mode == GameMode::Score {
                self.render_end_game(screen, assets, localization);
                return;
            }
            screen.rect(54, 76, WIDTH as i32 - 108, 74, 0x101018);
            screen.centered_text(
                &assets.font,
                if self.mode == GameMode::Hardcore {
                    "HARDCORE WORLD ENDED"
                } else {
                    "TIME IS UP"
                },
                91,
            );
            screen.centered_text(&assets.font, "SELECT TO RETURN", 132);
        }
    }
}
