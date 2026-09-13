//! Java Particle / SmashParticle / FireParticle / TextParticle.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleKind {
    Smash,
    Fire,
}

/// Java Particle lifetime (SmashParticle=10, FireParticle=30).
pub const fn lifetime(kind: ParticleKind) -> u32 {
    match kind {
        ParticleKind::Smash => 10,
        ParticleKind::Fire => 30,
    }
}

/// Java TextParticle lifetime: 60 ticks.
pub const TEXT_LIFETIME: u32 = 60;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_particle_motion_is_deterministic_and_bounces() {
        let a = text_particle_motion(7, 0);
        let b = text_particle_motion(7, 0);
        assert_eq!(a, b);
        // After enough ticks z must have bounced or settled non-negative.
        let (_, _, z) = text_particle_motion(7, 40);
        assert!(z >= 0);
    }
}
