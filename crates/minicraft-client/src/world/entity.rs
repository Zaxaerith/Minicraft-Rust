//! Compatibility shim — implementation lives in `crate::entity`.
//!
//! Keeps `crate::world::entity::*` imports working while the logic is split
//! across `entity/{types,arena,tick,mob,furniture,particle}/*` (Java `entity/*`).
pub use crate::entity::{
    DESPAWN_AGE, Entity, EntityArena, EntityKind, FurnitureKind, HitResult, ItemMotion, Mob,
    ParticleKind, Projectile, ProjectileKind, TickOutcome, can_stand, furniture_can_stand,
    mob_drops, mob_health, squared_distance, text_particle_motion,
};
