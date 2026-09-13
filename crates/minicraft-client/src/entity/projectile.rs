//! Java Arrow / Spark / FireSpark projectile runtime.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectileKind {
    Arrow,
    Spark,
    FireSpark,
}

/// Java Arrow / Spark instance fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    pub kind: ProjectileKind,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub damage: u8,
    pub hostile: bool,
    pub life: u16,
}

/// Java Arrow constructor: speed scales with damage; axis-aligned toward target.
pub fn arrow_velocity(dx: i32, dy: i32, damage: u8) -> (i16, i16) {
    let speed = (7 + i32::from(damage) / 2).min(15) as i16;
    if dx.abs() >= dy.abs() {
        (dx.signum() as i16 * speed, 0)
    } else {
        (0, dy.signum() as i16 * speed)
    }
}

/// Java Spark life: 360 + rand(30) ticks; no solid-tile destroy.
pub fn spark_life(seed: u64) -> u16 {
    let life = 360 + (crate::entity::helpers::split_mix(seed) % 30) as u16;
    life
}

/// Java Spark damage: FireSpark 2, Spark 1.
pub const fn spark_damage(fire: bool) -> u8 {
    if fire {
        2
    } else {
        1
    }
}
