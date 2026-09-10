use crate::item::{Inventory, ItemId, ItemStack};
use serde::{Deserialize, Serialize};

use super::{Tile, random::JavaRandom, spawn::NaturalMob};
pub use crate::entity::furniture::FurnitureKind;

pub(super) const DESPAWN_AGE: u32 = 18_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMotion {
    pub precise_x: f64,
    pub precise_y: f64,
    pub height: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub lifetime: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mob {
    pub species: NaturalMob,
    pub health: u16,
    pub max_health: u16,
    pub hurt_time: u8,
    pub walk_distance: u32,
    pub x_move: i32,
    pub y_move: i32,
    pub attack_delay: u16,
    pub attack_time: u16,
    pub phase: u8,
    pub sheared: bool,
    pub(super) walk_time: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectileKind {
    Arrow,
    Spark,
    FireSpark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    pub kind: ProjectileKind,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub damage: u8,
    pub hostile: bool,
    pub life: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleKind {
    Smash,
    Fire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityKind {
    Mob(Mob),
    Item(ItemStack),
    Furniture(FurnitureKind),
    Projectile(Projectile),
    Particle(ParticleKind),
    TextParticle(String),
    /// Java ExplosionTileTicker: temporary "explode" tiles settle into holes.
    ExplosionTicker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub age: u32,
    pub kind: EntityKind,
    pub state: u16,
    pub health: u16,
    #[serde(default)]
    pub item_motion: Option<ItemMotion>,
    storage: Option<Inventory>,
    #[serde(default)]
    pub(super) push_x: i32,
    #[serde(default)]
    pub(super) push_y: i32,
    #[serde(default)]
    pub(super) push_time: u8,
    pub(super) removed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    pub species: NaturalMob,
    pub defeated: bool,
    pub health: u16,
    pub damage: u16,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Default)]
pub struct TickOutcome {
    pub player_damage: u8,
    pub explosions: Vec<(i32, i32, i32, bool)>,
    pub defeated_mobs: Vec<NaturalMob>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct EntityArena {
    next_id: u64,
    entities: Vec<Entity>,
}

impl EntityArena {
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    pub fn mark_removed_by_id(&mut self, id: u64) {
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) {
            entity.removed = true;
        }
        self.entities.retain(|entity| !entity.removed);
    }

    pub fn death_chest_storage(&self, id: u64) -> Option<&Inventory> {
        self.entities
            .iter()
            .find(|entity| entity.id == id)
            .and_then(|entity| entity.storage.as_ref())
    }

    pub fn entities_mut_for_storage(&mut self, id: u64) -> Option<&mut Inventory> {
        self.entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .and_then(|entity| entity.storage.as_mut())
    }

    pub fn mob_count(&self) -> usize {
        self.entities
            .iter()
            .filter(|entity| !entity.removed && matches!(entity.kind, EntityKind::Mob(_)))
            .count()
    }

    pub fn active_boss(&self, species: NaturalMob) -> Option<(u16, u16)> {
        self.entities.iter().find_map(|entity| {
            if entity.removed {
                return None;
            }
            let EntityKind::Mob(mob) = &entity.kind else {
                return None;
            };
            (mob.species == species).then_some((mob.health, mob.max_health))
        })
    }

    pub fn spawn_mob(&mut self, species: NaturalMob, x: i32, y: i32) -> u64 {
        let max_health = mob_health(species);
        self.insert(
            x,
            y,
            EntityKind::Mob(Mob {
                species,
                health: max_health,
                max_health,
                hurt_time: 0,
                walk_distance: 0,
                x_move: 0,
                y_move: 0,
                attack_delay: 0,
                attack_time: 0,
                phase: 0,
                sheared: false,
                walk_time: 0,
            }),
        )
    }

    pub(crate) fn import_mob(
        &mut self,
        species: NaturalMob,
        x: i32,
        y: i32,
        health: Option<u16>,
        sheared: bool,
    ) {
        let id = self.spawn_mob(species, x, y);
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id)
            && let EntityKind::Mob(mob) = &mut entity.kind
        {
            if let Some(health) = health {
                mob.health = health.min(mob.max_health);
            }
            mob.sheared = sheared;
        }
    }

    pub(crate) fn import_furniture(
        &mut self,
        kind: FurnitureKind,
        x: i32,
        y: i32,
        state: u16,
        contents: &[(ItemId, u16)],
        tools: &[crate::item::ToolItem],
    ) {
        let id = self.spawn_furniture(kind, x, y);
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) {
            entity.state = state;
            if let Some(storage) = &mut entity.storage {
                for (item, count) in contents {
                    storage.add(*item, *count);
                }
                for tool in tools {
                    let _ = storage.add_tool(*tool);
                }
            }
        }
    }

    /// Java Player.die: DeathChest holds inventory copies plus active/armor;
    /// items are not dropped as loose ItemEntities.
    pub fn spawn_death_chest(
        &mut self,
        x: i32,
        y: i32,
        stacks: Vec<ItemStack>,
        tools: Vec<crate::item::ToolItem>,
    ) -> u64 {
        let id = self.spawn_furniture(FurnitureKind::Chest, x, y);
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) {
            entity.state = 2; // Death chest marker; dungeon chest uses 0/1 for lock state.
            if let Some(storage) = entity.storage.as_mut() {
                for stack in stacks {
                    storage.add(stack.item, stack.count);
                }
                for tool in tools {
                    let _ = storage.add_tool(tool);
                }
            }
        }
        id
    }

    pub fn spawn_item(&mut self, stack: ItemStack, x: i32, y: i32) -> u64 {
        let id = self.insert(x, y, EntityKind::Item(stack));
        let entity = self.entities.last_mut().expect("inserted item");
        let first = split_mix(id.wrapping_add(0xA076_1D64_78BD_642F));
        let second = split_mix(first);
        let third = split_mix(second);
        let fourth = split_mix(third);
        let gaussian_x = gaussian(first, second);
        let gaussian_y = gaussian(third, fourth);
        let mut drop_x = x;
        let mut drop_y = y;
        for attempt in 0..6 {
            let value = split_mix(fourth.wrapping_add(attempt));
            let candidate_x = x + (value % 11) as i32 - 5;
            let candidate_y = y + (value.rotate_left(23) % 11) as i32 - 5;
            if candidate_x.div_euclid(16) == x.div_euclid(16)
                && candidate_y.div_euclid(16) == y.div_euclid(16)
            {
                drop_x = candidate_x;
                drop_y = candidate_y;
                break;
            }
        }
        entity.x = drop_x;
        entity.y = drop_y;
        entity.item_motion = Some(ItemMotion {
            precise_x: f64::from(drop_x),
            precise_y: f64::from(drop_y),
            height: 2.0,
            velocity_x: gaussian_x * 0.3,
            velocity_y: gaussian_y * 0.2,
            velocity_z: unit(split_mix(fourth)) * 0.7 + 1.0,
            lifetime: 600 + (split_mix(first) % 70) as u32,
        });
        id
    }

    pub fn spawn_furniture(&mut self, kind: FurnitureKind, x: i32, y: i32) -> u64 {
        let id = self.insert(x, y, EntityKind::Furniture(kind));
        let entity = self.entities.last_mut().expect("inserted furniture");
        entity.health = if kind.spawner_mob().is_some() {
            100
        } else {
            20
        };
        if matches!(kind, FurnitureKind::Chest | FurnitureKind::DungeonChest) {
            let mut storage = Inventory::new(27);
            if kind == FurnitureKind::DungeonChest {
                storage.add(ItemId::GoldIngot, 3);
                storage.add(ItemId::Gem, 2);
                storage.add(ItemId::Key, 1);
            }
            entity.storage = Some(storage);
        }
        id
    }

    pub fn spawn_arrow(
        &mut self,
        x: i32,
        y: i32,
        target_x: i32,
        target_y: i32,
        damage: u8,
        hostile: bool,
    ) -> u64 {
        let dx = target_x - x;
        let dy = target_y - y;
        // Java Arrow: speed scales with damage; base direction axis-aligned.
        let speed = (7 + i32::from(damage) / 2).min(15) as i16;
        let (velocity_x, velocity_y) = if dx.abs() >= dy.abs() {
            (dx.signum() as i16 * speed, 0)
        } else {
            (0, dy.signum() as i16 * speed)
        };
        self.insert(
            x,
            y,
            EntityKind::Projectile(Projectile {
                kind: ProjectileKind::Arrow,
                velocity_x,
                velocity_y,
                damage,
                hostile,
                life: 180,
            }),
        )
    }

    fn spawn_spark(&mut self, x: i32, y: i32, velocity_x: i16, velocity_y: i16, fire: bool) -> u64 {
        // Java Spark: 360 + rand(30) ticks; no solid-tile destroy.
        let life = 360 + (split_mix(self.next_id) % 30) as u16;
        self.insert(
            x,
            y,
            EntityKind::Projectile(Projectile {
                kind: if fire {
                    ProjectileKind::FireSpark
                } else {
                    ProjectileKind::Spark
                },
                velocity_x,
                velocity_y,
                damage: if fire { 2 } else { 1 },
                hostile: true,
                life,
            }),
        )
    }

    pub fn spawn_particle(&mut self, kind: ParticleKind, x: i32, y: i32) -> u64 {
        self.insert(x, y, EntityKind::Particle(kind))
    }

    /// Java ExplosionTileTicker: state = radius, health = signed level depth.
    pub fn spawn_explosion_ticker(&mut self, x: i32, y: i32, radius: i32, depth: i8) -> u64 {
        let id = self.insert(x, y, EntityKind::ExplosionTicker);
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) {
            entity.state = radius.clamp(0, u16::MAX as i32) as u16;
            entity.health = depth as i16 as u16;
        }
        id
    }

    pub fn spawn_text_particle(&mut self, text: String, x: i32, y: i32) -> u64 {
        self.insert(x, y, EntityKind::TextParticle(text))
    }
}

/// Java TextParticle ballistic path (id-seeded velocities, bounce on z<0).
pub fn text_particle_motion(id: u64, age: u32) -> (i32, i32, i32) {
    let seed = id.wrapping_mul(0x9E37_79B9);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 2.0;
    let mut x_velocity = (seed % 601) as f64 / 1000.0 - 0.3;
    let mut y_velocity = (seed.rotate_left(17) % 401) as f64 / 1000.0 - 0.2;
    let mut z_velocity = 2.0 + (seed.rotate_left(31) % 701) as f64 / 1000.0;
    for _ in 0..age {
        x += x_velocity;
        y += y_velocity;
        z += z_velocity;
        if z < 0.0 {
            z = 0.0;
            z_velocity *= -0.5;
            x_velocity *= 0.6;
            y_velocity *= 0.6;
        }
        z_velocity -= 0.15;
    }
    (x as i32, y as i32, z as i32)
}

impl EntityArena {
    pub fn has_mob(&self, species: NaturalMob) -> bool {
        self.entities
            .iter()
            .any(|entity| matches!(&entity.kind, EntityKind::Mob(mob) if mob.species == species))
    }

    pub fn has_furniture(&self, kind: FurnitureKind) -> bool {
        self.entities.iter().any(|entity| {
            !entity.removed && matches!(entity.kind, EntityKind::Furniture(found) if found == kind)
        })
    }

    pub fn furniture_near(&self, x: i32, y: i32, radius: i32) -> Option<FurnitureKind> {
        self.furniture_entry_near(x, y, radius).map(|(_, kind, _)| kind)
    }

    /// Java Entity.interact / use search: nearest furniture with its id and state.
    pub fn furniture_entry_near(
        &self,
        x: i32,
        y: i32,
        radius: i32,
    ) -> Option<(u64, FurnitureKind, u16)> {
        self.entities.iter().find_map(|entity| {
            let EntityKind::Furniture(kind) = entity.kind else {
                return None;
            };
            (squared_distance(entity.x, entity.y, x, y) <= radius * radius)
                .then_some((entity.id, kind, entity.state))
        })
    }

    pub fn pickup_furniture_near(
        &mut self,
        x: i32,
        y: i32,
        radius: i32,
        creative: bool,
    ) -> Option<FurnitureKind> {
        let entity = self.entities.iter_mut().find(|entity| {
            if entity.removed || squared_distance(entity.x, entity.y, x, y) > radius * radius {
                return false;
            }
            let EntityKind::Furniture(kind) = entity.kind else {
                return false;
            };
            if kind == FurnitureKind::KnightStatue
                || (kind == FurnitureKind::Tnt && entity.state > 0)
            {
                return false;
            }
            if !creative
                && (kind.spawner_mob().is_some()
                    || entity
                        .storage
                        .as_ref()
                        .is_some_and(|storage| storage.used_slots() > 0))
            {
                return false;
            }
            true
        })?;
        let EntityKind::Furniture(kind) = entity.kind else {
            unreachable!();
        };
        entity.removed = true;
        self.entities.retain(|entity| !entity.removed);
        Some(kind)
    }

    #[allow(dead_code)] // Point tests kept for parity; player uses AABB helper.
    pub fn furniture_blocks(&self, x: i32, y: i32) -> bool {
        self.entities.iter().any(|entity| {
            let EntityKind::Furniture(kind) = entity.kind else {
                return false;
            };
            let (xr, yr) = kind.collision_radius();
            (entity.x - x).abs() <= xr && (entity.y - y).abs() <= yr
        })
    }

    /// Java Entity hitbox: player Mob xr=4 yr=3 vs furniture xr/yr AABB.
    /// Java Rectangle.intersects uses inclusive edges (a.left <= b.right).
    pub fn furniture_blocks_player(&self, player_x: i32, player_y: i32) -> bool {
        const PRX: i32 = 4;
        const PRY: i32 = 3;
        self.entities.iter().any(|entity| {
            let EntityKind::Furniture(kind) = entity.kind else {
                return false;
            };
            let (xr, yr) = kind.collision_radius();
            let ax0 = player_x - PRX;
            let ax1 = player_x + PRX;
            let ay0 = player_y - PRY;
            let ay1 = player_y + PRY;
            let bx0 = entity.x - xr;
            let bx1 = entity.x + xr;
            let by0 = entity.y - yr;
            let by1 = entity.y + yr;
            ax0 <= bx1 && ax1 >= bx0 && ay0 <= by1 && ay1 >= by0
        })
    }

    /// Java Entity.touchedBy → Furniture.tryPush (only when push timer is idle).
    /// KnightStatue.tryPush is a no-op; locked dungeon chests and death chests cannot move.
    pub fn try_push_furniture_near(
        &mut self,
        player_x: i32,
        player_y: i32,
        push_x: i32,
        push_y: i32,
    ) -> bool {
        let mut pushed = false;
        for entity in self.entities.iter_mut() {
            let EntityKind::Furniture(kind) = entity.kind else {
                continue;
            };
            if entity.state == 2 || entity.push_time != 0 {
                continue;
            }
            if kind == FurnitureKind::KnightStatue {
                continue;
            }
            if kind == FurnitureKind::DungeonChest && entity.state == 0 {
                continue;
            }
            let (xr, yr) = kind.collision_radius();
            let ax0 = player_x - 4;
            let ax1 = player_x + 4;
            let ay0 = player_y - 3;
            let ay1 = player_y + 3;
            let bx0 = entity.x - xr;
            let bx1 = entity.x + xr;
            let by0 = entity.y - yr;
            let by1 = entity.y + yr;
            if ax0 <= bx1 && ax1 >= bx0 && ay0 <= by1 && ay1 >= by0 {
                // Java: only arm one 1px step; next ticks no-op until pushTime expires.
                entity.push_x = push_x;
                entity.push_y = push_y;
                entity.push_time = 10;
                pushed = true;
            }
        }
        pushed
    }

    /// Java Entity.moveByEntityHitBoxChecks: furniture must not overlap another furniture AABB.
    fn furniture_hits_other(&self, id: u64, x: i32, y: i32, xr: i32, yr: i32) -> bool {
        self.entities.iter().any(|other| {
            if other.id == id || other.removed || !matches!(other.kind, EntityKind::Furniture(_)) {
                return false;
            }
            let EntityKind::Furniture(kind) = other.kind else {
                return false;
            };
            let (oxr, oyr) = kind.collision_radius();
            let ax0 = x - xr;
            let ax1 = x + xr;
            let ay0 = y - yr;
            let ay1 = y + yr;
            let bx0 = other.x - oxr;
            let bx1 = other.x + oxr;
            let by0 = other.y - oyr;
            let by1 = other.y + oyr;
            ax0 <= bx1 && ax1 >= bx0 && ay0 <= by1 && ay1 >= by0
        })
    }

    pub fn light_sources(&self) -> impl Iterator<Item = (i32, i32, i32)> + '_ {
        self.entities.iter().filter_map(|entity| {
            let EntityKind::Furniture(kind) = entity.kind else {
                return None;
            };
            (kind.light_radius() > 0).then_some((entity.x, entity.y, kind.light_radius()))
        })
    }

    fn insert(&mut self, x: i32, y: i32, kind: EntityKind) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.entities.push(Entity {
            id,
            x,
            y,
            age: 0,
            kind,
            state: 0,
            health: 0,
            item_motion: None,
            storage: None,
            push_x: 0,
            push_y: 0,
            push_time: 0,
            removed: false,
        });
        id
    }

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
    ) -> TickOutcome {
        let (player_x, player_y) = player.unwrap_or_default();
        let player_passive = player_passive || player.is_none();
        let mut outcome = TickOutcome::default();
        let mut queued_projectiles = Vec::new();
        let mut queued_mobs = Vec::new();
        let mut projectile_hits = Vec::new();

        for index in 0..self.entities.len() {
            let entity = &mut self.entities[index];
            entity.age = entity.age.saturating_add(1);
            match &mut entity.kind {
                EntityKind::Item(_) => {
                    let motion = entity.item_motion.get_or_insert_with(|| ItemMotion {
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
                EntityKind::Particle(kind) => {
                    let lifetime = match kind {
                        ParticleKind::Smash => 10,
                        ParticleKind::Fire => 30,
                    };
                    if entity.age > lifetime {
                        entity.removed = true;
                    }
                }
                EntityKind::TextParticle(_) => {
                    if entity.age > 60 {
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
                    if entity.state == 1 =>
                {
                    // Java DeathChest: 300s on normal; host handles touch-retrieve.
                    if entity.age >= 300 * 60 {
                        entity.removed = true;
                        continue;
                    }
                    if entity
                        .storage
                        .as_ref()
                        .is_some_and(|storage| storage.used_slots() == 0)
                    {
                        entity.removed = true;
                        continue;
                    }
                }
                EntityKind::Furniture(kind) => {
                    if let Some(species) = kind.spawner_mob() {
                        // Java Spawner: random 200–500 interval; hostile skips light/day surface.
                        let interval = 200 + (entity.age % 301) as u32;
                        let hostile = species.hostile();
                        let lit = {
                            let tx = entity.x.div_euclid(16);
                            let ty = entity.y.div_euclid(16);
                            if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
                                false
                            } else {
                                tiles[tx as usize + ty as usize * width].light_radius() > 0
                            }
                        };
                        // Java Spawner.trySpawn: surface/day skip + light skip for hostiles.
                        let surface_day = level_depth >= 0 && hostile && lit;
                        let near_player = player.is_some_and(|(px, py)| {
                            squared_distance(entity.x, entity.y, px, py) <= 128 * 128
                        });
                        if near_player && !(hostile && lit) && !(hostile && surface_day) {
                            if entity.age % interval == 0 {
                                let mut spawn_x = entity.x + 16;
                                let mut spawn_y = entity.y;
                                for (dx, dy) in
                                    [(16, 0), (-16, 0), (0, 16), (0, -16), (16, 16), (-16, -16)]
                                {
                                    let candidate_x = entity.x + dx;
                                    let candidate_y = entity.y + dy;
                                    if can_stand(tiles, data, width, height, candidate_x, candidate_y)
                                    {
                                        spawn_x = candidate_x;
                                        spawn_y = candidate_y;
                                        break;
                                    }
                                }
                                queued_mobs.push((species, spawn_x, spawn_y));
                            }
                        }
                    }
                    if *kind == FurnitureKind::Tnt && entity.state > 0 {
                        entity.state -= 1;
                        if entity.state == 0 {
                            outcome.explosions.push((entity.x, entity.y, 2, true));
                            if player.is_some()
                                && squared_distance(entity.x, entity.y, player_x, player_y)
                                    <= 32 * 32
                            {
                                outcome.player_damage = outcome.player_damage.max(4);
                            }
                            entity.removed = true;
                        }
                    }
                }
                EntityKind::Mob(_) => {
                    super::entity_mob_ai::tick_mob(
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
                        let radius = entity.state as i32;
                        let depth = entity.health as i16 as i8;
                        let center_x = entity.x.div_euclid(16);
                        let center_y = entity.y.div_euclid(16);
                        for tile_y in center_y - radius..=center_y + radius {
                            for tile_x in center_x - radius..=center_x + radius {
                                if tile_x < 0
                                    || tile_y < 0
                                    || tile_x >= width as i32
                                    || tile_y >= height as i32
                                {
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
                self.damage_mob_at(
                    target,
                    u16::from(damage),
                    random,
                    &mut outcome.defeated_mobs,
                );
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
        for (species, x, y) in queued_mobs {
            self.spawn_mob(species, x, y);
        }
        // Java Furniture.tick: move once by pushDir, then clear it; pushTime is only a cooldown.
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
        self.entities.retain(|entity| !entity.removed);
        outcome
    }

    fn damage_mob_at(
        &mut self,
        index: usize,
        damage: u16,
        random: &mut JavaRandom,
        defeated_mobs: &mut Vec<NaturalMob>,
    ) -> HitResult {
        let (x, y, species, defeated, health) = {
            let entity = &mut self.entities[index];
            let EntityKind::Mob(mob) = &mut entity.kind else {
                unreachable!();
            };
            mob.health = mob.health.saturating_sub(damage);
            mob.hurt_time = 8;
            let defeated = mob.health == 0;
            if defeated {
                entity.removed = true;
            }
            (entity.x, entity.y, mob.species, defeated, mob.health)
        };
        if defeated {
            defeated_mobs.push(species);
            for stack in mob_drops(species, random) {
                self.spawn_item(stack, x, y);
            }
        }
        HitResult {
            species,
            defeated,
            health,
            damage,
            x,
            y,
        }
    }

    pub fn damage_nearest(
        &mut self,
        target_x: i32,
        target_y: i32,
        damage: u16,
        random: &mut JavaRandom,
    ) -> Option<HitResult> {
        let target = self
            .entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| matches!(entity.kind, EntityKind::Mob(_)))
            .filter_map(|(index, entity)| {
                let distance = squared_distance(entity.x, entity.y, target_x, target_y);
                (distance <= 14 * 14).then_some((index, distance))
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(index, _)| index)?;
        let hit = self.damage_mob_at(target, damage, random, &mut Vec::new());
        self.entities.retain(|entity| !entity.removed);
        Some(hit)
    }

    pub fn shear_nearest(&mut self, target_x: i32, target_y: i32, random: &mut JavaRandom) -> bool {
        let Some(index) = self.entities.iter().position(|entity| {
            matches!(
                &entity.kind,
                EntityKind::Mob(Mob {
                    species: NaturalMob::Sheep,
                    sheared: false,
                    ..
                })
            ) && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        }) else {
            return false;
        };
        let (x, y) = {
            let entity = &mut self.entities[index];
            let EntityKind::Mob(mob) = &mut entity.kind else {
                unreachable!();
            };
            mob.sheared = true;
            (entity.x, entity.y)
        };
        let count = (random.next_int(3) + 1) as u16;
        self.spawn_item(ItemStack::new(ItemId::Wool, count), x, y);
        true
    }

    pub fn mob_near(&self, target_x: i32, target_y: i32) -> bool {
        self.entities.iter().any(|entity| {
            matches!(entity.kind, EntityKind::Mob(_))
                && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        })
    }

    pub fn ignite_tnt_near(&mut self, target_x: i32, target_y: i32) -> bool {
        let Some(entity) = self.entities.iter_mut().find(|entity| {
            matches!(entity.kind, EntityKind::Furniture(FurnitureKind::Tnt))
                && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        }) else {
            return false;
        };
        if entity.state == 0 {
            entity.state = 120;
        }
        true
    }

    pub fn tap_statue_near(&mut self, target_x: i32, target_y: i32) -> Option<u16> {
        let index = self.entities.iter().position(|entity| {
            matches!(
                entity.kind,
                EntityKind::Furniture(FurnitureKind::KnightStatue)
            ) && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        })?;
        self.entities[index].state += 1;
        let touches = self.entities[index].state;
        if touches >= 3 {
            let x = self.entities[index].x;
            let y = self.entities[index].y;
            self.entities[index].removed = true;
            self.spawn_mob(NaturalMob::ObsidianKnight, x, y);
            self.entities.retain(|entity| !entity.removed);
        }
        Some(touches)
    }

    pub fn use_container_near(
        &mut self,
        target_x: i32,
        target_y: i32,
        inventory: &mut Inventory,
        active: Option<ItemId>,
    ) -> Option<String> {
        let entity = self.entities.iter_mut().find(|entity| {
            matches!(
                entity.kind,
                EntityKind::Furniture(FurnitureKind::Chest | FurnitureKind::DungeonChest)
            ) && entity.state != 2 // Death chest marker: Java cannot open it.
            && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        })?;
        let EntityKind::Furniture(kind) = entity.kind else {
            unreachable!();
        };
        if kind == FurnitureKind::DungeonChest && entity.state == 0 {
            if !inventory.remove(ItemId::Key, 1) {
                return Some("A KEY IS REQUIRED".to_owned());
            }
            entity.state = 1;
            return Some("DUNGEON CHEST UNLOCKED".to_owned());
        }
        let storage = entity.storage.as_mut().expect("container storage");
        if let Some(item) = active
            && inventory.remove(item, 1)
        {
            storage.add(item, 1);
            return Some(format!("STORED {item}"));
        }
        let Some(stack) = storage.slots().first().copied() else {
            return Some("CHEST EMPTY".to_owned());
        };
        if inventory.add(stack.item, stack.count) == 0 {
            storage.remove(stack.item, stack.count);
            Some(format!("TOOK {} x{}", stack.item, stack.count))
        } else {
            Some("INVENTORY FULL".to_owned())
        }
    }

    pub fn use_composter_near(
        &mut self,
        target_x: i32,
        target_y: i32,
        inventory: &mut Inventory,
        active: Option<ItemId>,
    ) -> Option<String> {
        let entity = self.entities.iter_mut().find(|entity| {
            matches!(entity.kind, EntityKind::Furniture(FurnitureKind::Composter))
                && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        })?;
        let item = active?;
        if !matches!(
            item,
            ItemId::BakedPotato
                | ItemId::Bread
                | ItemId::Apple
                | ItemId::Potato
                | ItemId::Carrot
                | ItemId::Wheat
                | ItemId::RawPork
                | ItemId::RawBeef
                | ItemId::RawFish
        ) || !inventory.remove(item, 1)
        {
            return Some("COMPOSTABLE FOOD REQUIRED".to_owned());
        }
        entity.state += if matches!(item, ItemId::BakedPotato | ItemId::Bread) {
            2
        } else {
            1
        };
        if entity.state >= 3 {
            entity.state = 0;
            inventory.add(ItemId::Fertilizer, 1);
            Some("COMPOST READY: FERTILIZER".to_owned())
        } else {
            Some(format!("COMPOST {}/3", entity.state))
        }
    }

    pub fn collect_near(
        &mut self,
        player_x: i32,
        player_y: i32,
        inventory: &mut Inventory,
    ) -> Vec<ItemStack> {
        let mut collected = Vec::new();
        for entity in &mut self.entities {
            if entity.age <= 30 {
                continue;
            }
            if squared_distance(entity.x, entity.y, player_x, player_y) > 12 * 12 {
                continue;
            }
            let EntityKind::Item(stack) = &mut entity.kind else {
                continue;
            };
            let before = stack.count;
            stack.count = inventory.add(stack.item, stack.count);
            let inserted = before - stack.count;
            if inserted > 0 {
                collected.push(ItemStack::new(stack.item, inserted));
            }
            if stack.count == 0 {
                entity.removed = true;
            }
        }
        self.entities.retain(|entity| !entity.removed);
        collected
    }
}

fn split_mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn unit(value: u64) -> f64 {
    ((value >> 11) as f64 + 1.0) / ((1_u64 << 53) as f64 + 1.0)
}

fn gaussian(first: u64, second: u64) -> f64 {
    (-2.0 * unit(first).ln()).sqrt() * (std::f64::consts::TAU * unit(second)).cos()
}

fn mob_health(species: NaturalMob) -> u16 {
    match species {
        NaturalMob::Slime => 1,
        NaturalMob::Zombie | NaturalMob::Cow => 5,
        NaturalMob::Skeleton => 6,
        NaturalMob::Snake => 7,
        NaturalMob::Knight => 9,
        NaturalMob::Creeper => 10,
        NaturalMob::Pig | NaturalMob::Sheep => 8,
        NaturalMob::AirWizard => 2_000,
        NaturalMob::ObsidianKnight => 5_000,
    }
}

fn mob_drops(species: NaturalMob, random: &mut JavaRandom) -> Vec<ItemStack> {
    match species {
        NaturalMob::AirWizard => vec![ItemStack::new(
            ItemId::CloudOre,
            (random.next_int(6) + 5) as u16,
        )],
        NaturalMob::ObsidianKnight => vec![
            ItemStack::new(ItemId::Shard, (random.next_int(11) + 15) as u16),
            ItemStack::new(ItemId::ObsidianHeart, 1),
        ],
        _ => {
            let (item, maximum) = match species {
                NaturalMob::Slime => (ItemId::Slime, 4),
                NaturalMob::Zombie => (ItemId::Cloth, 3),
                NaturalMob::Creeper => (ItemId::Gunpowder, 3),
                NaturalMob::Skeleton => (ItemId::Bone, 2),
                NaturalMob::Snake => (ItemId::Scale, 2),
                NaturalMob::Knight => (ItemId::Shard, 3),
                NaturalMob::Cow => (ItemId::RawBeef, 3),
                NaturalMob::Pig => (ItemId::RawPork, 3),
                NaturalMob::Sheep => (ItemId::Wool, 3),
                NaturalMob::AirWizard | NaturalMob::ObsidianKnight => unreachable!(),
            };
            vec![ItemStack::new(item, (random.next_int(maximum) + 1) as u16)]
        }
    }
}

pub(super) fn squared_distance(x: i32, y: i32, other_x: i32, other_y: i32) -> i32 {
    let dx = x - other_x;
    let dy = y - other_y;
    dx * dx + dy * dy
}

pub(super) fn can_stand(
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
) -> bool {
    // Player Mob xr=4 yr=3; keep the historical y+4 corner used by World::can_stand.
    [(-4, -3), (4, -3), (-4, 4), (4, 4)]
        .into_iter()
        .all(|(offset_x, offset_y)| tile_free(tiles, data, width, height, x + offset_x, y + offset_y))
}

/// Java Entity.moveX/Y tile corners for an xr/yr hitbox.
pub(super) fn furniture_can_stand(
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    xr: i32,
    yr: i32,
) -> bool {
    [(-xr, -yr), (xr, -yr), (-xr, yr), (xr, yr)]
        .into_iter()
        .all(|(offset_x, offset_y)| tile_free(tiles, data, width, height, x + offset_x, y + offset_y))
}

fn tile_free(
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
) -> bool {
    let tile_x = x.div_euclid(16);
    let tile_y = y.div_euclid(16);
    if tile_x < 0 || tile_y < 0 || tile_x >= width as i32 || tile_y >= height as i32 {
        return false;
    }
    let index = tile_x as usize + tile_y as usize * width;
    !tiles[index].solid(data[index])
}

#[cfg(test)]
mod tests {
    #[test]
    fn text_particle_motion_is_deterministic_and_bounces() {
        let a = super::text_particle_motion(7, 0);
        let b = super::text_particle_motion(7, 0);
        assert_eq!(a, b);
        // After enough ticks z must have bounced or settled non-negative.
        let (_, _, z) = super::text_particle_motion(7, 40);
        assert!(z >= 0);
    }
    use super::*;

    #[test]
    fn mob_damage_creates_collectible_drop() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(4);
        arena.spawn_mob(NaturalMob::Slime, 40, 40);
        let hit = arena.damage_nearest(40, 40, 2, &mut random).unwrap();
        assert!(hit.defeated);
        assert_eq!(arena.mob_count(), 0);
        for entity in &mut arena.entities {
            if matches!(entity.kind, EntityKind::Item(_)) {
                entity.age = 31;
            }
        }
        let mut inventory = Inventory::new(8);
        let collected = arena.collect_near(40, 40, &mut inventory);
        assert_eq!(collected[0].item, ItemId::Slime);
        assert!(inventory.count(ItemId::Slime) > 0);
    }

    #[test]
    fn explosion_ticker_converts_exploded_tiles_to_holes_after_18_ticks() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(1);
        let mut tiles = vec![Tile::Grass; 16 * 16];
        // Pre-place Exploded tiles as apply_explosion does before the ticker settles them.
        for y in 7..=9 {
            for x in 7..=9 {
                tiles[x + y * 16] = Tile::Exploded;
            }
        }
        let data = vec![0u16; 16 * 16];
        arena.spawn_explosion_ticker(8 * 16 + 8, 8 * 16 + 8, 1, 0);
        for _ in 0..18 {
            arena.tick(&mut tiles, &data, 16, 16, 0, 0, false, false, &mut random);
        }
        assert_eq!(tiles[8 + 8 * 16], Tile::Hole);
        assert_eq!(tiles[7 + 7 * 16], Tile::Hole);
        assert_eq!(tiles[9 + 9 * 16], Tile::Hole);
        assert!(arena
            .entities()
            .iter()
            .all(|entity| !matches!(entity.kind, EntityKind::ExplosionTicker)));
    }

    #[test]
    fn explosion_ticker_on_sky_depth_becomes_infinite_fall() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(1);
        let mut tiles = vec![Tile::Exploded; 16 * 16];
        let data = vec![0u16; 16 * 16];
        arena.spawn_explosion_ticker(8 * 16 + 8, 8 * 16 + 8, 0, 1);
        for _ in 0..18 {
            arena.tick(&mut tiles, &data, 16, 16, 0, 0, false, false, &mut random);
        }
        assert_eq!(tiles[8 + 8 * 16], Tile::InfiniteFall);
    }

    #[test]
    fn skeleton_projectile_and_creeper_fuse_are_stateful() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(9);
        let mut tiles = vec![Tile::Grass; 16 * 16];
        let data = vec![0; 16 * 16];
        arena.spawn_mob(NaturalMob::Skeleton, 80, 80);
        arena.spawn_mob(NaturalMob::Creeper, 145, 80);
        arena.tick(
            &mut tiles,
            &data,
            16,
            16,
            150,
            80,
            false,
            false,
            &mut random,
        );
        assert!(
            arena
                .entities()
                .iter()
                .any(|entity| matches!(entity.kind, EntityKind::Projectile(_)))
        );
        assert!(arena.entities().iter().any(|entity| {
            matches!(
                &entity.kind,
                EntityKind::Mob(Mob {
                    species: NaturalMob::Creeper,
                    attack_time: 1..,
                    ..
                })
            )
        }));
    }

    #[test]
    fn dungeon_chest_unlocks_and_transfers_its_local_loot() {
        let mut arena = EntityArena::default();
        arena.spawn_furniture(FurnitureKind::DungeonChest, 40, 40);
        let mut inventory = Inventory::new(8);
        inventory.add(ItemId::Key, 1);
        assert_eq!(
            arena.use_container_near(40, 40, &mut inventory, None),
            Some("DUNGEON CHEST UNLOCKED".to_owned())
        );
        assert_eq!(inventory.count(ItemId::Key), 0);
        assert!(
            arena
                .use_container_near(40, 40, &mut inventory, None)
                .unwrap()
                .starts_with("TOOK")
        );
        assert_eq!(inventory.count(ItemId::GoldIngot), 3);
    }

    #[test]
    fn both_bosses_drop_their_progression_items() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(12);
        arena.spawn_mob(NaturalMob::AirWizard, 40, 40);
        assert_eq!(
            arena.active_boss(NaturalMob::AirWizard),
            Some((2_000, 2_000))
        );
        assert!(
            arena
                .damage_nearest(40, 40, 2_000, &mut random)
                .unwrap()
                .defeated
        );
        assert_eq!(arena.active_boss(NaturalMob::AirWizard), None);
        arena.spawn_mob(NaturalMob::ObsidianKnight, 60, 40);
        assert!(
            arena
                .damage_nearest(60, 40, 5_000, &mut random)
                .unwrap()
                .defeated
        );
        for entity in &mut arena.entities {
            if matches!(entity.kind, EntityKind::Item(_)) {
                entity.age = 31;
            }
        }
        let mut inventory = Inventory::new(8);
        arena.collect_near(40, 40, &mut inventory);
        arena.collect_near(60, 40, &mut inventory);
        assert!(inventory.count(ItemId::CloudOre) >= 5);
        assert!(inventory.count(ItemId::Shard) >= 15);
        assert_eq!(inventory.count(ItemId::ObsidianHeart), 1);
    }

    #[test]
    fn dropped_items_use_java_ballistics_and_the_thirty_tick_pickup_delay() {
        let mut arena = EntityArena::default();
        arena.spawn_item(ItemStack::new(ItemId::Acorn, 1), 40, 40);
        let item = &arena.entities[0];
        let motion = item.item_motion.as_ref().unwrap();
        assert_eq!(motion.height, 2.0);
        assert!((1.0..1.7).contains(&motion.velocity_z));

        let mut inventory = Inventory::new(8);
        assert!(arena.collect_near(40, 40, &mut inventory).is_empty());
        arena.entities[0].age = 31;
        assert_eq!(arena.collect_near(40, 40, &mut inventory).len(), 1);
        assert_eq!(inventory.count(ItemId::Acorn), 1);
    }
}
