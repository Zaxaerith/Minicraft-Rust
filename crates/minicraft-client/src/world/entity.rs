//! Compatibility shim — implementation lives in `crate::entity`.
//!
//! Keeps `crate::world::entity::*` imports working while the logic is split
//! across `entity/{types,arena,tick,mob,furniture,particle}/*` (Java `entity/*`).
pub use crate::entity::{
    Entity, EntityArena, EntityKind, FurnitureKind, ParticleKind, ProjectileKind, TickOutcome,
    text_particle_motion,
};
