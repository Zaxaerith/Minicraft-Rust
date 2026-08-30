use crate::item::{Inventory, ItemId, ItemStack};
use serde::{Deserialize, Serialize};

use super::{Tile, random::JavaRandom, spawn::NaturalMob};

const DESPAWN_AGE: u32 = 18_000;

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
    walk_time: u16,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FurnitureKind {
    Workbench,
    Oven,
    Furnace,
    Anvil,
    Enchanter,
    Loom,
    Chest,
    DungeonChest,
    Lantern,
    IronLantern,
    GoldLantern,
    Tnt,
    Bed,
    Composter,
    KnightStatue,
    CowSpawner,
    PigSpawner,
    SheepSpawner,
    SlimeSpawner,
    ZombieSpawner,
    CreeperSpawner,
    SkeletonSpawner,
    SnakeSpawner,
    KnightSpawner,
}

impl FurnitureKind {
    pub const ALL: [Self; 24] = [
        Self::Workbench,
        Self::Oven,
        Self::Furnace,
        Self::Anvil,
        Self::Enchanter,
        Self::Loom,
        Self::Chest,
        Self::DungeonChest,
        Self::Lantern,
        Self::IronLantern,
        Self::GoldLantern,
        Self::Tnt,
        Self::Bed,
        Self::Composter,
        Self::KnightStatue,
        Self::CowSpawner,
        Self::PigSpawner,
        Self::SheepSpawner,
        Self::SlimeSpawner,
        Self::ZombieSpawner,
        Self::CreeperSpawner,
        Self::SkeletonSpawner,
        Self::SnakeSpawner,
        Self::KnightSpawner,
    ];

    pub const fn id(self) -> usize {
        self as usize
    }

    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::Workbench => "workbench",
            Self::Oven => "oven",
            Self::Furnace => "furnace",
            Self::Anvil => "anvil",
            Self::Enchanter => "enchanter",
            Self::Loom => "loom",
            Self::Chest => "chest",
            Self::DungeonChest => "dungeon_chest",
            Self::Lantern => "lantern",
            Self::IronLantern => "iron_lantern",
            Self::GoldLantern => "gold_lantern",
            Self::Tnt => "tnt",
            Self::Bed => "bed",
            Self::Composter => "composter",
            Self::KnightStatue => "knight_statue",
            Self::CowSpawner
            | Self::PigSpawner
            | Self::SheepSpawner
            | Self::SlimeSpawner
            | Self::ZombieSpawner
            | Self::CreeperSpawner
            | Self::SkeletonSpawner
            | Self::SnakeSpawner
            | Self::KnightSpawner => "spawner",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Workbench => "WORKBENCH",
            Self::Oven => "OVEN",
            Self::Furnace => "FURNACE",
            Self::Anvil => "ANVIL",
            Self::Enchanter => "ENCHANTER",
            Self::Loom => "LOOM",
            Self::Chest => "CHEST",
            Self::DungeonChest => "DUNGEON CHEST",
            Self::Lantern => "LANTERN",
            Self::IronLantern => "IRON LANTERN",
            Self::GoldLantern => "GOLD LANTERN",
            Self::Tnt => "TNT",
            Self::Bed => "BED",
            Self::Composter => "COMPOSTER",
            Self::KnightStatue => "KNIGHT STATUE",
            Self::CowSpawner => "COW SPAWNER",
            Self::PigSpawner => "PIG SPAWNER",
            Self::SheepSpawner => "SHEEP SPAWNER",
            Self::SlimeSpawner => "SLIME SPAWNER",
            Self::ZombieSpawner => "ZOMBIE SPAWNER",
            Self::CreeperSpawner => "CREEPER SPAWNER",
            Self::SkeletonSpawner => "SKELETON SPAWNER",
            Self::SnakeSpawner => "SNAKE SPAWNER",
            Self::KnightSpawner => "KNIGHT SPAWNER",
        }
    }

    pub const fn crafting(self) -> bool {
        matches!(
            self,
            Self::Workbench
                | Self::Oven
                | Self::Furnace
                | Self::Anvil
                | Self::Enchanter
                | Self::Loom
        )
    }

    pub const fn spawner_mob(self) -> Option<NaturalMob> {
        match self {
            Self::CowSpawner => Some(NaturalMob::Cow),
            Self::PigSpawner => Some(NaturalMob::Pig),
            Self::SheepSpawner => Some(NaturalMob::Sheep),
            Self::SlimeSpawner => Some(NaturalMob::Slime),
            Self::ZombieSpawner => Some(NaturalMob::Zombie),
            Self::CreeperSpawner => Some(NaturalMob::Creeper),
            Self::SkeletonSpawner => Some(NaturalMob::Skeleton),
            Self::SnakeSpawner => Some(NaturalMob::Snake),
            Self::KnightSpawner => Some(NaturalMob::Knight),
            _ => None,
        }
    }

    pub const fn light_radius(self) -> i32 {
        match self {
            Self::Lantern => 8,
            Self::IronLantern => 12,
            Self::GoldLantern => 15,
            _ => 0,
        }
    }
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
    removed: bool,
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
        let (velocity_x, velocity_y) = if dx.abs() >= dy.abs() {
            (dx.signum() as i16 * 7, 0)
        } else {
            (0, dy.signum() as i16 * 7)
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
                life: 360,
            }),
        )
    }

    pub fn spawn_particle(&mut self, kind: ParticleKind, x: i32, y: i32) -> u64 {
        self.insert(x, y, EntityKind::Particle(kind))
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
        self.entities.iter().find_map(|entity| {
            let EntityKind::Furniture(kind) = entity.kind else {
                return None;
            };
            (squared_distance(entity.x, entity.y, x, y) <= radius * radius).then_some(kind)
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

    pub fn furniture_blocks(&self, x: i32, y: i32) -> bool {
        self.entities.iter().any(|entity| {
            matches!(entity.kind, EntityKind::Furniture(_))
                && (entity.x - x).abs() < 7
                && (entity.y - y).abs() < 7
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
            removed: false,
        });
        id
    }

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
                    if tiles[tile_index].solid(data[tile_index]) {
                        entity.removed = true;
                        continue;
                    }
                    if projectile.hostile {
                        if squared_distance(entity.x, entity.y, player_x, player_y) <= 7 * 7 {
                            outcome.player_damage = outcome.player_damage.max(projectile.damage);
                            entity.removed = true;
                        }
                    } else {
                        projectile_hits.push((index, entity.x, entity.y, projectile.damage));
                    }
                }
                EntityKind::Furniture(kind) => {
                    if let Some(species) = kind.spawner_mob()
                        && entity.age.is_multiple_of(300)
                        && squared_distance(entity.x, entity.y, player_x, player_y) <= 128 * 128
                    {
                        queued_mobs.push((species, entity.x + 16, entity.y));
                    }
                    if *kind == FurnitureKind::Tnt && entity.state > 0 {
                        entity.state -= 1;
                        if entity.state == 0 {
                            outcome.explosions.push((entity.x, entity.y, 2, true));
                            if squared_distance(entity.x, entity.y, player_x, player_y) <= 32 * 32 {
                                outcome.player_damage = outcome.player_damage.max(4);
                            }
                            entity.removed = true;
                        }
                    }
                }
                EntityKind::Mob(mob) => {
                    if time_slowed && entity.age.is_multiple_of(2) {
                        continue;
                    }
                    mob.hurt_time = mob.hurt_time.saturating_sub(1);
                    if entity.age > DESPAWN_AGE
                        && !matches!(
                            mob.species,
                            NaturalMob::AirWizard | NaturalMob::ObsidianKnight
                        )
                        && squared_distance(entity.x, entity.y, player_x, player_y) > 160 * 160
                    {
                        entity.removed = true;
                        continue;
                    }

                    let distance = squared_distance(entity.x, entity.y, player_x, player_y);
                    if mob.species == NaturalMob::Sheep && random.next_int(1000) == 0 {
                        let tx = entity.x.div_euclid(16);
                        let ty = entity.y.div_euclid(16);
                        if tx >= 0 && ty >= 0 && tx < width as i32 && ty < height as i32 {
                            let tile_index = tx as usize + ty as usize * width;
                            if tiles[tile_index] == Tile::Grass {
                                tiles[tile_index] = Tile::Dirt;
                                mob.sheared = false;
                            }
                        }
                    }

                    if !player_passive {
                        match mob.species {
                            NaturalMob::Skeleton => {
                                mob.attack_delay = mob.attack_delay.saturating_sub(1);
                                if distance < 100 * 100 && mob.attack_delay == 0 {
                                    queued_projectiles.push((
                                        ProjectileKind::Arrow,
                                        entity.x,
                                        entity.y,
                                        player_x,
                                        player_y,
                                        1,
                                    ));
                                    mob.attack_delay = 83;
                                }
                            }
                            NaturalMob::Creeper => {
                                if distance <= 12 * 12 && mob.attack_time == 0 {
                                    mob.attack_time = 60;
                                }
                                if mob.attack_time > 0 {
                                    mob.attack_time -= 1;
                                    mob.x_move = 0;
                                    mob.y_move = 0;
                                    if mob.attack_time == 0 && distance < 64 * 64 {
                                        outcome.explosions.push((entity.x, entity.y, 1, false));
                                        outcome.player_damage = outcome.player_damage.max(3);
                                        entity.removed = true;
                                        continue;
                                    }
                                }
                            }
                            NaturalMob::AirWizard | NaturalMob::ObsidianKnight => {
                                mob.phase = u8::from(mob.health <= mob.max_health / 2);
                                mob.attack_delay = mob.attack_delay.saturating_sub(1);
                                if mob.attack_time > 0 {
                                    mob.attack_time -= 1;
                                    mob.x_move = 0;
                                    mob.y_move = 0;
                                    if mob.attack_time.is_multiple_of(5) {
                                        let directions = [
                                            (-2, 0),
                                            (-1, -1),
                                            (0, -2),
                                            (1, -1),
                                            (2, 0),
                                            (1, 1),
                                            (0, 2),
                                            (-1, 1),
                                        ];
                                        let direction = directions[(mob.attack_time as usize / 5
                                            + mob.phase as usize)
                                            % 8];
                                        queued_projectiles.push((
                                            if mob.species == NaturalMob::ObsidianKnight {
                                                ProjectileKind::FireSpark
                                            } else {
                                                ProjectileKind::Spark
                                            },
                                            entity.x,
                                            entity.y,
                                            entity.x + direction.0,
                                            entity.y + direction.1,
                                            1 + mob.phase,
                                        ));
                                    }
                                } else if mob.attack_delay == 0 && distance < 50 * 50 {
                                    mob.attack_delay = 120;
                                    mob.attack_time = 120;
                                }
                            }
                            _ => {}
                        }
                    }

                    if !player_passive
                        && mob.species.hostile()
                        && distance < 80 * 80
                        && mob.attack_time == 0
                    {
                        mob.x_move = (player_x - entity.x).signum();
                        mob.y_move = (player_y - entity.y).signum();
                        mob.walk_time = 20;
                    } else if mob.walk_time == 0 && random.next_int(40) == 0 {
                        mob.x_move = random.next_int(3) - 1;
                        mob.y_move = random.next_int(3) - 1;
                        mob.walk_time = 45;
                    }
                    mob.walk_time = mob.walk_time.saturating_sub(1);
                    if mob.walk_time == 0 {
                        mob.x_move = 0;
                        mob.y_move = 0;
                    }

                    let slime_jumping = mob.species != NaturalMob::Slime || entity.age % 20 < 10;
                    let boss_speed = if mob.species == NaturalMob::ObsidianKnight && mob.phase == 1
                    {
                        2
                    } else {
                        1
                    };
                    if slime_jumping && entity.age.is_multiple_of(2) {
                        for _ in 0..boss_speed {
                            let next_x = entity.x + mob.x_move;
                            if can_stand(tiles, data, width, height, next_x, entity.y) {
                                entity.x = next_x;
                            } else {
                                mob.x_move = 0;
                            }
                            let next_y = entity.y + mob.y_move;
                            if can_stand(tiles, data, width, height, entity.x, next_y) {
                                entity.y = next_y;
                            } else {
                                mob.y_move = 0;
                            }
                        }
                        if mob.x_move != 0 || mob.y_move != 0 {
                            mob.walk_distance = mob.walk_distance.wrapping_add(1);
                        }
                    }
                    if !player_passive
                        && mob.species.hostile()
                        && squared_distance(entity.x, entity.y, player_x, player_y) <= 10 * 10
                    {
                        outcome.player_damage = outcome.player_damage.max(
                            if mob.species == NaturalMob::ObsidianKnight {
                                2
                            } else {
                                1
                            },
                        );
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
            ) && squared_distance(entity.x, entity.y, target_x, target_y) <= 14 * 14
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

impl NaturalMob {
    pub const fn hostile(self) -> bool {
        !matches!(self, Self::Cow | Self::Pig | Self::Sheep)
    }

    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::Slime => "slime",
            Self::Zombie => "zombie",
            Self::Creeper => "creeper",
            Self::Skeleton => "skeleton",
            Self::Snake => "snake",
            Self::Knight => "knight",
            Self::Cow => "cow",
            Self::Pig => "pig",
            Self::Sheep => "sheep",
            Self::AirWizard => "air_wizard",
            Self::ObsidianKnight => "obsidian_knight",
        }
    }
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

fn squared_distance(x: i32, y: i32, other_x: i32, other_y: i32) -> i32 {
    let dx = x - other_x;
    let dy = y - other_y;
    dx * dx + dy * dy
}

fn can_stand(tiles: &[Tile], data: &[u16], width: usize, height: usize, x: i32, y: i32) -> bool {
    [(-4, -3), (4, -3), (-4, 4), (4, 4)]
        .into_iter()
        .all(|(offset_x, offset_y)| {
            let tile_x = (x + offset_x).div_euclid(16);
            let tile_y = (y + offset_y).div_euclid(16);
            if tile_x < 0 || tile_y < 0 || tile_x >= width as i32 || tile_y >= height as i32 {
                return false;
            }
            let index = tile_x as usize + tile_y as usize * width;
            !tiles[index].solid(data[index])
        })
}

#[cfg(test)]
mod tests {
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
