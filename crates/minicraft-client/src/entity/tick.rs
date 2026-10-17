//! Java Level.tickEntity: per-entity kind dispatch for one game tick.
use super::arena::EntityArena;
use super::furniture::FurnitureKind;
use super::furniture::chest;
use super::furniture::spawner;
use super::furniture::tnt;
use super::helpers::{furniture_can_stand, squared_distance};
use super::mob::NaturalMob;
use super::mob::ai;
use super::particle;
use super::projectile::ProjectileKind;
use super::types::{EntityKind, Mob, TickOutcome};
use crate::level::noise::JavaRandom;
use crate::level::tile::Tile;

impl EntityArena {
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        tiles: &mut [Tile],
        data: &[u16],
        width: usize,
        height: usize,
        player_x: i32,
        player_y: i32,
        time_slowed: bool,
        player_passive: bool,
        random: &mut JavaRandom,
    ) -> TickOutcome {
        self.tick_with_player(
            tiles,
            data,
            width,
            height,
            Some((player_x, player_y)),
            time_slowed,
            player_passive,
            random,
        )
    }

    /// None represents a player removed from the level (including Bed.use),
    /// not a creative/passive player at an invented off-map coordinate.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn tick_with_player(
        &mut self,
        tiles: &mut [Tile],
        data: &[u16],
        width: usize,
        height: usize,
        player: Option<(i32, i32)>,
        time_slowed: bool,
        player_passive: bool,
        random: &mut JavaRandom,
    ) -> TickOutcome {
        self.tick_with_player_difficulty(
            tiles,
            data,
            width,
            height,
            player,
            time_slowed,
            player_passive,
            random,
            1,
            0,
            50_000,
            300,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tick_with_player_difficulty(
        &mut self,
        tiles: &mut [Tile],
        data: &[u16],
        width: usize,
        height: usize,
        player: Option<(i32, i32)>,
        time_slowed: bool,
        player_passive: bool,
        random: &mut JavaRandom,
        player_difficulty: u8,
        level_depth: i8,
        day_tick: u32,
        max_mob_count: usize,
        score_mode: bool,
    ) -> TickOutcome {
        let (player_x, player_y) = player.unwrap_or_default();
        let player_passive = player_passive || player.is_none();
        let mut outcome = TickOutcome::default();
        let mut mob_count = self.mob_count();
        let lights: Vec<_> = self.light_sources().collect();
        let mut queued_projectiles = Vec::new();
        let mut queued_mobs = Vec::new();
        let mut queued_statues = Vec::new();
        let mut spilled_stacks = Vec::new();
        let mut spilled_tools = Vec::new();
        let mut projectile_hits = Vec::new();

        for index in 0..self.entities.len() {
            let entity = &mut self.entities[index];
            entity.age = entity.age.saturating_add(1);
            match &mut entity.kind {
                EntityKind::Item(_) | EntityKind::Tool(_) => {
                    tick_item(entity);
                }
                EntityKind::Particle(kind) => {
                    let lifetime = particle::lifetime(*kind);
                    if entity.age > lifetime {
                        entity.removed = true;
                    }
                }
                EntityKind::TextParticle(_) => {
                    if entity.age > particle::TEXT_LIFETIME {
                        entity.removed = true;
                    }
                }
                EntityKind::Projectile(projectile) => {
                    projectile.life = projectile.life.saturating_sub(1);
                    if projectile.life == 0 {
                        entity.removed = true;
                        continue;
                    }
                    entity.x += i32::from(projectile.velocity_x);
                    entity.y += i32::from(projectile.velocity_y);
                    let tile_x = entity.x.div_euclid(16);
                    let tile_y = entity.y.div_euclid(16);
                    if tile_x < 0 || tile_y < 0 || tile_x >= width as i32 || tile_y >= height as i32
                    {
                        entity.removed = true;
                        continue;
                    }
                    let tile_index = tile_x as usize + tile_y as usize * width;
                    // Java Spark ignores solid tiles; Arrow/other projectiles collide.
                    if projectile.kind != ProjectileKind::Spark
                        && projectile.kind != ProjectileKind::FireSpark
                        && tiles[tile_index].solid(data[tile_index])
                    {
                        entity.removed = true;
                        continue;
                    }
                    if projectile.hostile {
                        if player.is_some()
                            && squared_distance(entity.x, entity.y, player_x, player_y) <= 7 * 7
                        {
                            outcome.player_damage = outcome.player_damage.max(projectile.damage);
                            entity.removed = true;
                        }
                    } else {
                        projectile_hits.push((index, entity.x, entity.y, projectile.damage));
                    }
                }
                EntityKind::Furniture(FurnitureKind::Chest | FurnitureKind::DungeonChest)
                    if entity.state == chest::DEATH_CHEST_STATE =>
                {
                    // Old Rust saves retain the normal lifetime; new chests capture difficulty.
                    let used = entity
                        .storage
                        .as_ref()
                        .map(|storage| storage.used_slots())
                        .unwrap_or(0);
                    if entity.age
                        >= entity
                            .death_chest_lifetime
                            .unwrap_or(chest::DEATH_CHEST_LIFETIME)
                        || used == 0
                    {
                        if let Some(storage) = entity.storage.as_mut() {
                            spilled_stacks.extend(
                                storage
                                    .take_all()
                                    .into_iter()
                                    .map(|stack| (stack, entity.x, entity.y)),
                            );
                            spilled_tools.extend(
                                storage
                                    .take_tools()
                                    .into_iter()
                                    .map(|tool| (tool, entity.x, entity.y)),
                            );
                        }
                        entity.removed = true;
                        continue;
                    }
                }
                EntityKind::Furniture(kind) => {
                    if let Some(species) = kind.spawner_mob() {
                        let state = entity
                            .spawner
                            .get_or_insert_with(|| spawner::new_state(1, random));
                        let lit = spawner_lit(tiles, width, height, entity.x, entity.y, &lights);
                        if let Some(spawn) = spawner::tick_spawn(
                            species,
                            state,
                            entity.x,
                            entity.y,
                            tiles,
                            data,
                            width,
                            height,
                            player,
                            level_depth,
                            day_tick,
                            mob_count,
                            max_mob_count,
                            lit,
                            random,
                        ) {
                            queued_mobs.push(spawn);
                            mob_count += 1;
                        }
                    }
                    tick_furniture(
                        *kind,
                        entity.x,
                        entity.y,
                        &mut entity.state,
                        player,
                        &mut outcome,
                    );
                }
                EntityKind::Mob(mob)
                    if mob.species == NaturalMob::ObsidianKnight && player.is_none() =>
                {
                    queued_statues.push((entity.x, entity.y, mob.health));
                    entity.removed = true;
                }
                EntityKind::Mob(_) => {
                    ai::tick_mob(
                        entity,
                        tiles,
                        data,
                        width,
                        height,
                        player,
                        player_passive,
                        time_slowed,
                        random,
                        &mut outcome,
                        &mut queued_projectiles,
                        player_difficulty,
                    );
                }
                EntityKind::ExplosionTicker => {
                    // Java ExplosionTileTicker: 18 ticks then convert explode → hole.
                    const EXPLOSION_TIME: u32 = 18;
                    if entity.age >= EXPLOSION_TIME {
                        settle_explosion(
                            tiles,
                            width,
                            height,
                            entity.x,
                            entity.y,
                            entity.state as i32,
                            entity.health as i16 as i8,
                        );
                        entity.removed = true;
                    }
                }
            }
        }

        for (projectile_index, x, y, damage) in projectile_hits {
            let Some(target) = self
                .entities
                .iter()
                .enumerate()
                .filter(|(_, entity)| matches!(entity.kind, EntityKind::Mob(_)))
                .filter(|(_, entity)| squared_distance(entity.x, entity.y, x, y) <= 7 * 7)
                .min_by_key(|(_, entity)| squared_distance(entity.x, entity.y, x, y))
                .map(|(index, _)| index)
            else {
                continue;
            };
            let blocked = matches!(
                &self.entities[target].kind,
                EntityKind::Mob(Mob {
                    species: NaturalMob::ObsidianKnight,
                    phase: 0,
                    ..
                })
            );
            if !blocked {
                let hit = self.damage_mob_at(
                    target,
                    u16::from(damage),
                    random,
                    &mut outcome.defeated_mobs,
                );
                if hit.defeated {
                    outcome.defeated_levels.push((hit.species, hit.level));
                }
            }
            self.entities[projectile_index].removed = true;
        }

        for (kind, x, y, target_x, target_y, damage) in queued_projectiles {
            match kind {
                ProjectileKind::Arrow => {
                    self.spawn_arrow(x, y, target_x, target_y, damage, true);
                }
                ProjectileKind::Spark | ProjectileKind::FireSpark => {
                    self.spawn_spark(
                        x,
                        y,
                        (target_x - x).clamp(-2, 2) as i16,
                        (target_y - y).clamp(-2, 2) as i16,
                        kind == ProjectileKind::FireSpark,
                    );
                }
            }
        }
        for (species, level, x, y) in queued_mobs {
            // State was validated at creation/import; corrupted native states fail closed.
            let _ = self.spawn_mob_level(species, level, player_difficulty, score_mode, x, y);
            self.spawn_particle(super::particle::ParticleKind::Fire, x, y);
        }
        for (x, y, health) in queued_statues {
            let _ = self.import_statue(x, y, health);
        }
        for (stack, x, y) in spilled_stacks {
            self.spawn_item(stack, x, y);
        }
        for (tool, x, y) in spilled_tools {
            self.spawn_tool(tool, x, y);
        }
        self.tick_furniture_push(tiles, data, width, height);
        self.entities.retain(|entity| !entity.removed);
        outcome
    }

    /// Java Furniture.tick: move once by pushDir, then clear it; pushTime is only a cooldown.
    fn tick_furniture_push(&mut self, tiles: &[Tile], data: &[u16], width: usize, height: usize) {
        let push_moves: Vec<(usize, u64, i32, i32, i32, i32)> = self
            .entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                let EntityKind::Furniture(kind) = entity.kind else {
                    return None;
                };
                if entity.push_x == 0 && entity.push_y == 0 {
                    return None;
                }
                let (xr, yr) = kind.collision_radius();
                Some((
                    index,
                    entity.id,
                    entity.x + entity.push_x,
                    entity.y + entity.push_y,
                    xr,
                    yr,
                ))
            })
            .collect();
        for (index, id, next_x, next_y, xr, yr) in push_moves {
            self.entities[index].push_x = 0;
            self.entities[index].push_y = 0;
            if furniture_can_stand(tiles, data, width, height, next_x, next_y, xr, yr)
                && !self.furniture_hits_other(id, next_x, next_y, xr, yr)
            {
                self.entities[index].x = next_x;
                self.entities[index].y = next_y;
            }
        }
        for entity in self.entities.iter_mut() {
            if matches!(entity.kind, EntityKind::Furniture(_)) {
                entity.push_time = entity.push_time.saturating_sub(1);
            }
        }
    }
}

/// Java ItemEntity.tick: ballistic hop + lifetime.
fn tick_item(entity: &mut super::types::Entity) {
    let motion = entity
        .item_motion
        .get_or_insert_with(|| super::types::ItemMotion {
            precise_x: f64::from(entity.x),
            precise_y: f64::from(entity.y),
            height: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
            lifetime: 600,
        });
    motion.precise_x += motion.velocity_x;
    motion.precise_y += motion.velocity_y;
    motion.height += motion.velocity_z;
    if motion.height < 0.0 {
        motion.height = 0.0;
        motion.velocity_z *= -0.5;
        motion.velocity_x *= 0.6;
        motion.velocity_y *= 0.6;
    }
    motion.velocity_z -= 0.15;
    entity.x = motion.precise_x as i32;
    entity.y = motion.precise_y as i32;
    if entity.age >= motion.lifetime {
        entity.removed = true;
    }
}

/// Java Spawner / Tnt furniture per-tick body (mutates state, queues spawns).
#[allow(clippy::too_many_arguments)]
fn tick_furniture(
    kind: FurnitureKind,
    x: i32,
    y: i32,
    state: &mut u16,
    player: Option<(i32, i32)>,
    outcome: &mut TickOutcome,
) {
    if kind == FurnitureKind::Tnt
        && let Some((ex, ey, radius, player_tnt, player_damage)) =
            tnt::tick_fuse(state, x, y, player)
    {
        outcome.explosions.push((ex, ey, radius, player_tnt));
        if player_damage > 0 {
            outcome.player_damage = outcome.player_damage.max(player_damage);
        }
    }
}

/// Java ExplosionTileTicker: Exploded → Hole (or InfiniteFall on sky depth).
fn settle_explosion(
    tiles: &mut [Tile],
    width: usize,
    height: usize,
    center_px: i32,
    center_py: i32,
    radius: i32,
    depth: i8,
) {
    let center_x = center_px.div_euclid(16);
    let center_y = center_py.div_euclid(16);
    for tile_y in center_y - radius..=center_y + radius {
        for tile_x in center_x - radius..=center_x + radius {
            if tile_x < 0 || tile_y < 0 || tile_x >= width as i32 || tile_y >= height as i32 {
                continue;
            }
            let index = tile_x as usize + tile_y as usize * width;
            if tiles[index] == Tile::Exploded {
                tiles[index] = if depth == 1 {
                    Tile::InfiniteFall
                } else {
                    Tile::Hole
                };
            }
        }
    }
}

/// Java Level.isLight: any torch in the 7x7 tile square, or lantern range.
fn spawner_lit(
    tiles: &[Tile],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    lights: &[(i32, i32, i32)],
) -> bool {
    let (tx, ty) = (x.div_euclid(16), y.div_euclid(16));
    for cy in (ty - 3).max(0)..=(ty + 3).min(height as i32 - 1) {
        for cx in (tx - 3).max(0)..=(tx + 3).min(width as i32 - 1) {
            if tiles[cx as usize + cy as usize * width] == Tile::Torch {
                return true;
            }
        }
    }
    lights.iter().any(|(lx, ly, r)| {
        squared_distance(lx.div_euclid(16), ly.div_euclid(16), tx, ty) < (r - 1) * (r - 1)
    })
}
