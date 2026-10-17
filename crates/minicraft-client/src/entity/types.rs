//! Java Entity / Mob shared data shapes.
use crate::item::{Inventory, ItemStack, ToolItem};
use serde::{Deserialize, Serialize};

use super::furniture::FurnitureKind;
use super::mob::NaturalMob;
use super::particle::ParticleKind;
use super::projectile::Projectile;

/// Java MobAi lifetime (60 * Updater.normSpeed ≈ 18000) before forced despawn.
pub const DESPAWN_AGE: u32 = 18_000;

/// Java ItemEntity ballistic motion state.
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

/// Java Mob runtime fields shared by every species (health, walk, attack timers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mob {
    pub species: NaturalMob,
    /// Java EnemyMob.lvl; absent in historical Rust states means level one.
    #[serde(default = "default_mob_level")]
    pub level: u8,
    /// Difficulty used by the Java species drop table (easy=0, normal=1, hard=2).
    #[serde(default = "default_mob_level")]
    pub difficulty: u8,
    #[serde(default)]
    pub score_mode: bool,
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
    /// Old native sheep were white; Java Load also constructs white sheep
    /// because its official Save omits color. This never reconstructs history.
    #[serde(default = "default_sheep_color")]
    pub sheep_color: u8,
    pub(crate) walk_time: u16,
}

pub(crate) const fn default_sheep_color() -> u8 {
    15
}

pub(crate) const fn default_mob_level() -> u8 {
    1
}

/// Java Spawner constructor state. Java Save omits health and timer; native
/// saves retain them after the first import instead of restarting each load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnerState {
    pub level: u8,
    pub spawn_ticks: u16,
}

/// Java EntityKind tag: which Java subclass this arena entry models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityKind {
    Mob(Mob),
    Item(ItemStack),
    Tool(ToolItem),
    Furniture(FurnitureKind),
    Projectile(Projectile),
    Particle(ParticleKind),
    TextParticle(String),
    /// Java ExplosionTileTicker: temporary "explode" tiles settle into holes.
    ExplosionTicker,
}

/// Java Entity instance fields (x/y/age/eid/removed + storage/push timers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub age: u32,
    /// Absolute age at expiry; absent in old saves means the normal lifetime.
    #[serde(default)]
    pub(crate) death_chest_lifetime: Option<u32>,
    pub kind: EntityKind,
    #[serde(default)]
    pub spawner: Option<SpawnerState>,
    #[serde(default)]
    pub statue_boss_health: Option<u16>,
    pub state: u16,
    pub health: u16,
    #[serde(default)]
    pub item_motion: Option<ItemMotion>,
    pub(crate) storage: Option<Inventory>,
    #[serde(default)]
    pub(crate) push_x: i32,
    #[serde(default)]
    pub(crate) push_y: i32,
    #[serde(default)]
    pub(crate) push_time: u8,
    pub(crate) removed: bool,
}

/// Result of damaging a mob (Java Mob.doHurt + die).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    pub species: NaturalMob,
    pub level: u8,
    pub defeated: bool,
    pub health: u16,
    pub damage: u16,
    pub x: i32,
    pub y: i32,
}

/// Aggregated per-tick side effects from EntityArena::tick.
#[derive(Debug, Default)]
pub struct TickOutcome {
    pub player_damage: u8,
    pub explosions: Vec<(i32, i32, i32, bool)>,
    pub defeated_mobs: Vec<NaturalMob>,
    pub defeated_levels: Vec<(NaturalMob, u8)>,
}
