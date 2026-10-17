//! Java Level entity list: spawn, query, damage, storage, furniture search.
use crate::item::{Inventory, ItemId, ItemStack, ToolItem};
use crate::level::noise::JavaRandom;
use serde::{Deserialize, Serialize};

use super::furniture::FurnitureKind;
use super::furniture::chest;
use super::furniture::statue;
use super::helpers::{gaussian, split_mix, squared_distance, unit};
use super::mob::NaturalMob;
use super::particle::ParticleKind;
use super::projectile::{Projectile, ProjectileKind, arrow_velocity, spark_damage, spark_life};
use super::stats::{mob_drops_at_level, mob_health, mob_health_at_level};
use super::types::{Entity, EntityKind, HitResult, ItemMotion, Mob};

#[derive(Serialize, Deserialize)]
pub struct EntityArena {
    /// Starts at 1 so the first entity never gets id 0 (Java rejects 0/negative).
    next_id: u64,
    pub(crate) entities: Vec<Entity>,
}

impl Default for EntityArena {
    fn default() -> Self {
        Self {
            next_id: 1,
            entities: Vec::new(),
        }
    }
}

impl EntityArena {
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Reject corrupt instance fields before a native world is published/loaded.
    /// Historical max_health values remain authoritative: missing level defaults
    /// to one and must not retroactively rescale a pre-migration Rust entity.
    pub(crate) fn validate_states(&self) -> Result<(), String> {
        for entity in &self.entities {
            if let EntityKind::Mob(mob) = &entity.kind
                && (!(1..=super::stats::mob_max_level(mob.species)).contains(&mob.level)
                    || mob.difficulty > 2
                    || mob.max_health == 0
                    || mob.health > mob.max_health
                    || (!entity.removed && mob.health == 0)
                    || mob.phase > 1)
            {
                return Err(format!(
                    "invalid native {:?} level/health/state",
                    mob.species
                ));
            }
            if let Some(state) = &entity.spawner {
                let EntityKind::Furniture(kind) = entity.kind else {
                    return Err("Spawner state attached to a non-furniture entity".into());
                };
                let species = kind
                    .spawner_mob()
                    .ok_or("Spawner state attached to wrong furniture")?;
                if !(1..=super::stats::mob_max_level(species)).contains(&state.level)
                    || state.spawn_ticks > 500
                {
                    return Err("invalid native Spawner level/countdown".into());
                }
            }
            if matches!(entity.kind, EntityKind::Furniture(kind) if kind.spawner_mob().is_some())
                && ((!entity.removed && entity.health == 0) || entity.health > 100)
            {
                return Err("invalid native Spawner health".into());
            }
            if let Some(health) = entity.statue_boss_health
                && (!matches!(
                    entity.kind,
                    EntityKind::Furniture(FurnitureKind::KnightStatue)
                ) || !(1..=5_000).contains(&health))
            {
                return Err("invalid native KnightStatue bossHealth".into());
            }
        }
        Ok(())
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
                level: 1,
                difficulty: 1,
                score_mode: false,
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

    /// Constructs Java per-instance level and difficulty attributes. This is
    /// separate from the historical Rust constructor to preserve old fixtures.
    pub fn spawn_mob_level(
        &mut self,
        species: NaturalMob,
        level: u8,
        difficulty: u8,
        score_mode: bool,
        x: i32,
        y: i32,
    ) -> Result<u64, String> {
        let max_health = mob_health_at_level(species, level, difficulty)
            .ok_or("invalid Java mob level or difficulty")?;
        let id = self.spawn_mob(species, x, y);
        let EntityKind::Mob(mob) = &mut self.entities.last_mut().expect("inserted mob").kind else {
            unreachable!()
        };
        mob.level = level;
        mob.difficulty = difficulty;
        mob.score_mode = score_mode;
        mob.health = max_health;
        mob.max_health = max_health;
        if species == NaturalMob::Skeleton {
            mob.attack_delay = 500 / (u16::from(level) + 5);
        }
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn import_mob_level(
        &mut self,
        species: NaturalMob,
        level: u8,
        difficulty: u8,
        score_mode: bool,
        x: i32,
        y: i32,
        health: u16,
        sheared: bool,
    ) -> Result<(), String> {
        let maximum = mob_health_at_level(species, level, difficulty)
            .ok_or("invalid Java mob level or difficulty")?;
        if health == 0 || health > maximum {
            return Err("invalid Java mob health for level/difficulty".into());
        }
        self.spawn_mob_level(species, level, difficulty, score_mode, x, y)?;
        let EntityKind::Mob(mob) = &mut self.entities.last_mut().expect("inserted mob").kind else {
            unreachable!()
        };
        mob.health = health;
        mob.sheared = sheared;
        Ok(())
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
        contents: &[ItemStack],
        tools: &[ToolItem],
    ) -> Result<(), String> {
        let id = self.spawn_furniture(kind, x, y);
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) {
            entity.state = state;
            if let Some(storage) = &mut entity.storage {
                // Import replaces the official saved contents, not generated loot.
                *storage = Inventory::new(storage.capacity());
                for stack in contents {
                    if storage.add_stack(*stack) != 0 {
                        return Err("Java container exceeds native capacity".into());
                    }
                }
                for tool in tools {
                    storage
                        .add_tool(*tool)
                        .ok_or("Java container exceeds native tool capacity")?;
                }
            }
        }
        Ok(())
    }

    /// Java Player.die: DeathChest holds inventory copies plus active/armor;
    /// items are not dropped as loose ItemEntities.
    pub fn spawn_death_chest(
        &mut self,
        x: i32,
        y: i32,
        stacks: Vec<ItemStack>,
        tools: Vec<ToolItem>,
    ) -> u64 {
        let id = self.spawn_furniture(FurnitureKind::Chest, x, y);
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) {
            entity.state = chest::DEATH_CHEST_STATE;
            if let Some(storage) = entity.storage.as_mut() {
                // Java DeathChest has unlimited storage: a full bag plus armor
                // must fit even though a normal chest has only 27 slots.
                storage.set_capacity(usize::MAX);
                for stack in stacks {
                    storage.add_stack(stack);
                }
                for tool in tools {
                    let _ = storage.add_tool(tool);
                }
            }
        }
        id
    }

    pub(crate) fn spawn_tool(&mut self, tool: ToolItem, x: i32, y: i32) -> u64 {
        // Share ItemEntity motion/lifetime while preserving the tool instance.
        let id = self.spawn_item(ItemStack::new(ItemId::Wood, 1), x, y);
        self.entities.last_mut().expect("inserted tool").kind = EntityKind::Tool(tool);
        id
    }

    pub(crate) fn set_death_chest_remaining(&mut self, id: u64, ticks: u32) {
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) {
            entity.death_chest_lifetime = Some(entity.age.saturating_add(ticks));
        }
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
        if chest::is_container(kind) {
            let storage = if kind == FurnitureKind::DungeonChest {
                chest::dungeon_chest_loot()
            } else {
                Inventory::new(27)
            };
            entity.storage = Some(storage);
        }
        id
    }

    pub fn spawn_furniture_with_state(
        &mut self,
        kind: FurnitureKind,
        x: i32,
        y: i32,
        state: u16,
    ) -> u64 {
        let id = self.spawn_furniture(kind, x, y);
        self.entities.last_mut().expect("inserted furniture").state = state;
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
        let (velocity_x, velocity_y) = arrow_velocity(target_x - x, target_y - y, damage);
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

    pub(crate) fn spawn_spark(
        &mut self,
        x: i32,
        y: i32,
        velocity_x: i16,
        velocity_y: i16,
        fire: bool,
    ) -> u64 {
        let life = spark_life(self.next_id);
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
                damage: spark_damage(fire),
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
        self.furniture_entry_near(x, y, radius)
            .map(|(_, kind, _)| kind)
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
            (squared_distance(entity.x, entity.y, x, y) <= radius * radius).then_some((
                entity.id,
                kind,
                entity.state,
            ))
        })
    }

    /// Checks the same Power Glove rules without removing anything. The caller
    /// can reserve inventory room before committing the pickup.
    pub fn pickup_furniture_candidate_near(
        &self,
        x: i32,
        y: i32,
        radius: i32,
        creative: bool,
    ) -> Option<(u64, FurnitureKind, u16)> {
        self.entities.iter().find_map(|entity| {
            if entity.removed || squared_distance(entity.x, entity.y, x, y) > radius * radius {
                return None;
            }
            let EntityKind::Furniture(kind) = entity.kind else {
                return None;
            };
            if (chest::is_container(kind) && entity.state == chest::DEATH_CHEST_STATE)
                || kind == FurnitureKind::KnightStatue
                // A furniture ItemId cannot retain the selected Spawner level.
                // Until it can, reject pickup instead of resetting the instance.
                || entity.spawner.as_ref().is_some_and(|spawner| spawner.level != 1)
                || (kind == FurnitureKind::Tnt && entity.state > 0)
                || (!creative
                    && (kind.spawner_mob().is_some()
                        || entity
                            .storage
                            .as_ref()
                            .is_some_and(|storage| storage.used_slots() > 0)))
            {
                return None;
            }
            Some((entity.id, kind, entity.state))
        })
    }

    pub fn pickup_furniture_near(
        &mut self,
        x: i32,
        y: i32,
        radius: i32,
        creative: bool,
    ) -> Option<FurnitureKind> {
        let (id, kind, _) = self.pickup_furniture_candidate_near(x, y, radius, creative)?;
        self.mark_removed_by_id(id);
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
            aabb_overlap(player_x, player_y, PRX, PRY, entity.x, entity.y, xr, yr)
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
            if (chest::is_container(kind) && entity.state == chest::DEATH_CHEST_STATE)
                || entity.push_time != 0
            {
                continue;
            }
            if !statue::can_push() && kind == FurnitureKind::KnightStatue {
                continue;
            }
            if kind == FurnitureKind::DungeonChest && entity.state == 0 {
                continue;
            }
            let (xr, yr) = kind.collision_radius();
            if aabb_overlap(player_x, player_y, 4, 3, entity.x, entity.y, xr, yr) {
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
    pub(crate) fn furniture_hits_other(&self, id: u64, x: i32, y: i32, xr: i32, yr: i32) -> bool {
        self.entities.iter().any(|other| {
            if other.id == id || other.removed || !matches!(other.kind, EntityKind::Furniture(_)) {
                return false;
            }
            let EntityKind::Furniture(kind) = other.kind else {
                return false;
            };
            let (oxr, oyr) = kind.collision_radius();
            aabb_overlap(x, y, xr, yr, other.x, other.y, oxr, oyr)
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

    pub(crate) fn insert(&mut self, x: i32, y: i32, kind: EntityKind) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.entities.push(Entity {
            id,
            x,
            y,
            age: 0,
            death_chest_lifetime: None,
            kind,
            spawner: None,
            statue_boss_health: None,
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

    pub(crate) fn damage_mob_at(
        &mut self,
        index: usize,
        damage: u16,
        random: &mut JavaRandom,
        defeated_mobs: &mut Vec<NaturalMob>,
    ) -> HitResult {
        if let EntityKind::Mob(mob) = &self.entities[index].kind
            && mob.hurt_time > 0
        {
            let entity = &self.entities[index];
            return HitResult {
                species: mob.species,
                level: mob.level,
                defeated: false,
                health: mob.health,
                damage: 0,
                x: entity.x,
                y: entity.y,
            };
        }
        let (x, y, species, level, difficulty, score_mode, defeated, health) = {
            let entity = &mut self.entities[index];
            let EntityKind::Mob(mob) = &mut entity.kind else {
                unreachable!();
            };
            mob.health = mob.health.saturating_sub(damage);
            // Java Mob.doHurt: hurtTime = 10 (was misaligned at 8).
            mob.hurt_time = 10;
            let defeated = mob.health == 0;
            if defeated {
                entity.removed = true;
            }
            (
                entity.x,
                entity.y,
                mob.species,
                mob.level,
                mob.difficulty,
                mob.score_mode,
                defeated,
                mob.health,
            )
        };
        if defeated {
            defeated_mobs.push(species);
            for stack in mob_drops_at_level(species, level, difficulty, score_mode, random) {
                self.spawn_item(stack, x, y);
            }
        }
        HitResult {
            species,
            level,
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
        super::furniture::tnt::ignite(&mut entity.state);
        true
    }

    /// Source Save persists only bossHealth; taps restart at zero on Java load.
    pub(crate) fn import_statue(&mut self, x: i32, y: i32, health: u16) -> Result<(), String> {
        if health == 0 || health > 5_000 {
            return Err("invalid KnightStatue bossHealth".into());
        }
        self.spawn_furniture(FurnitureKind::KnightStatue, x, y);
        self.entities
            .last_mut()
            .expect("inserted statue")
            .statue_boss_health = Some(health);
        Ok(())
    }

    pub(crate) fn import_spawner(
        &mut self,
        kind: FurnitureKind,
        x: i32,
        y: i32,
        level: u8,
        random: &mut JavaRandom,
    ) -> Result<(), String> {
        let species = kind.spawner_mob().ok_or("unsupported Spawner species")?;
        if !(1..=super::stats::mob_max_level(species)).contains(&level) {
            return Err("invalid Spawner mob level".into());
        }
        self.spawn_furniture(kind, x, y);
        self.entities.last_mut().expect("inserted spawner").spawner =
            Some(super::furniture::spawner::new_state(level, random));
        Ok(())
    }

    /// Java Spawner.interact: any tool can damage, pickaxe gains 2..6; no loot
    /// and no durability payment occurs in this Java furniture method.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn damage_spawner_near(
        &mut self,
        x: i32,
        y: i32,
        tier: u8,
        pickaxe: bool,
        creative: bool,
        haste: bool,
        random: &mut JavaRandom,
    ) -> Option<bool> {
        let entity = self.entities.iter_mut().find(|e| {
            !e.removed
                && matches!(e.kind, EntityKind::Furniture(kind) if kind.spawner_mob().is_some())
                && squared_distance(e.x, e.y, x, y) <= 14 * 14
        })?;
        let damage = if creative {
            entity.health
        } else {
            let amount = u16::from(tier)
                + random.next_int(2) as u16
                + if pickaxe {
                    random.next_int(5) as u16 + 2
                } else {
                    0
                };
            amount * if haste { 2 } else { 1 }
        };
        entity.health = entity.health.saturating_sub(damage);
        let defeated = entity.health == 0;
        entity.removed = defeated;
        self.entities.retain(|e| !e.removed);
        Some(defeated)
    }

    pub fn tap_statue_near(&mut self, target_x: i32, target_y: i32) -> Option<u16> {
        let index = self.entities.iter().position(|entity| {
            matches!(
                entity.kind,
                EntityKind::Furniture(FurnitureKind::KnightStatue)
            ) && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        })?;
        if self.has_mob(NaturalMob::ObsidianKnight) {
            return None;
        }
        self.entities[index].state = self.entities[index].state.saturating_add(1);
        let touches = self.entities[index].state;
        if touches >= statue::TAPS_TO_AWAKEN {
            let x = self.entities[index].x;
            let y = self.entities[index].y;
            self.entities[index].removed = true;
            let health = self.entities[index].statue_boss_health.unwrap_or(5_000);
            let id = self.spawn_mob(NaturalMob::ObsidianKnight, x, y);
            if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id)
                && let EntityKind::Mob(mob) = &mut entity.kind
            {
                // Java constructor restores health while phase starts at zero;
                // its first actual tick transitions the damaged boss.
                mob.health = health;
            }
            self.entities.retain(|entity| !entity.removed);
        }
        Some(touches)
    }

    /// Java DungeonChest.use after Load restores chestCount from locked boxes.
    /// Derive it from persisted lock states instead of storing a second counter.
    pub(crate) fn unlock_dungeon_chest(&mut self, id: u64, inventory: &mut Inventory) -> bool {
        let Some(index) = self.entities.iter().position(|entity| {
            entity.id == id
                && !entity.removed
                && entity.state == 0
                && matches!(
                    entity.kind,
                    EntityKind::Furniture(FurnitureKind::DungeonChest)
                )
        }) else {
            return false;
        };
        if !inventory.remove(ItemId::Key, 1) {
            return false;
        }
        let (x, y) = (self.entities[index].x, self.entities[index].y);
        self.entities[index].state = 1;
        if !self.entities.iter().any(|entity| {
            !entity.removed
                && entity.state == 0
                && matches!(
                    entity.kind,
                    EntityKind::Furniture(FurnitureKind::DungeonChest)
                )
        }) {
            // Java Level.dropItem(x,y,5,Gold Apple) creates five separate drops.
            for _ in 0..5 {
                self.spawn_item(ItemStack::new(ItemId::GoldenApple, 1), x, y);
            }
        }
        true
    }

    pub fn use_container_near(
        &mut self,
        target_x: i32,
        target_y: i32,
        inventory: &mut Inventory,
        active: Option<ItemId>,
    ) -> Option<String> {
        let slot = active.map(|item| {
            inventory
                .slots()
                .iter()
                .position(|stack| stack.item == item)
                .unwrap_or(usize::MAX)
        });
        self.use_container_slot_near(target_x, target_y, inventory, slot)
    }

    /// Exact-instance counterpart used by a selected stateful inventory slot.
    pub(crate) fn use_container_slot_near(
        &mut self,
        target_x: i32,
        target_y: i32,
        inventory: &mut Inventory,
        active: Option<usize>,
    ) -> Option<String> {
        let index = self.entities.iter().position(|entity| {
            matches!(
                entity.kind,
                EntityKind::Furniture(FurnitureKind::Chest | FurnitureKind::DungeonChest)
            ) && entity.state != chest::DEATH_CHEST_STATE
                && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
        })?;
        let EntityKind::Furniture(kind) = self.entities[index].kind else {
            unreachable!();
        };
        if kind == FurnitureKind::DungeonChest && self.entities[index].state == 0 {
            if !self.unlock_dungeon_chest(self.entities[index].id, inventory) {
                return Some("A KEY IS REQUIRED".to_owned());
            }
            return Some("DUNGEON CHEST UNLOCKED".to_owned());
        }
        let entity = &mut self.entities[index];
        let storage = entity.storage.as_mut().expect("container storage");
        if let Some(index) = active {
            let Some(mut stack) = inventory.slots().get(index).copied() else {
                return Some("ITEM MISSING".into());
            };
            let item = stack.item;
            stack.count = 1;
            if storage.add_stack(stack) != 0 {
                return Some("CHEST FULL".into());
            }
            inventory.remove_slot(index, 1);
            return Some(format!("STORED {item}"));
        }
        let Some(stack) = storage.slots().first().copied() else {
            if let Some(tool) = storage.tools().first().copied() {
                if inventory.add_tool(tool).is_none() {
                    return Some("INVENTORY FULL".into());
                }
                storage.tools.remove(0);
                return Some(format!("TOOK {}", tool.kind.display_name()));
            }
            return Some("CHEST EMPTY".to_owned());
        };
        let remaining = inventory.add_stack(stack);
        let inserted = stack.count - remaining;
        if inserted > 0 {
            storage.remove_slot(0, inserted);
        }
        if remaining == 0 {
            Some(format!("TOOK {} x{}", stack.item, inserted))
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
        let value = super::furniture::composter::compost_value(item);
        if value == 0 || !inventory.remove(item, 1) {
            return Some("COMPOSTABLE FOOD REQUIRED".to_owned());
        }
        entity.state += value;
        if entity.state >= super::furniture::composter::THRESHOLD {
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
            if let EntityKind::Tool(tool) = entity.kind {
                if inventory.add_tool(tool).is_some() {
                    entity.removed = true;
                }
                continue;
            }
            let EntityKind::Item(stack) = &mut entity.kind else {
                continue;
            };
            let before = stack.count;
            stack.count = inventory.add_stack(*stack);
            let inserted = before - stack.count;
            if inserted > 0 {
                collected.push(ItemStack {
                    count: inserted,
                    ..*stack
                });
            }
            if stack.count == 0 {
                entity.removed = true;
            }
        }
        self.entities.retain(|entity| !entity.removed);
        collected
    }
}

/// Java Rectangle.intersects: inclusive edges.
// Keep the existing coordinate/render API without adding wrapper state.
#[allow(clippy::too_many_arguments)]
fn aabb_overlap(
    ax: i32,
    ay: i32,
    axr: i32,
    ayr: i32,
    bx: i32,
    by: i32,
    bxr: i32,
    byr: i32,
) -> bool {
    let ax0 = ax - axr;
    let ax1 = ax + axr;
    let ay0 = ay - ayr;
    let ay1 = ay + ayr;
    let bx0 = bx - bxr;
    let bx1 = bx + bxr;
    let by0 = by - byr;
    let by1 = by + byr;
    ax0 <= bx1 && ax1 >= bx0 && ay0 <= by1 && ay1 >= by0
}
