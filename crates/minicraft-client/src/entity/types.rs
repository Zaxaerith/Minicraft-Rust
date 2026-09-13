//! Java Entity / Mob shared data shapes.
use crate::item::{Inventory, ItemStack};
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
    pub(crate) walk_time: u16,
}

/// Java EntityKind tag: which Java subclass this arena entry models.
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

/// Java Entity instance fields (x/y/age/eid/removed + storage/push timers).
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
}
