const MULTIPLIER: u64 = 0x5DEECE66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1_u64 << 48) - 1;

use serde::{Deserialize, Serialize};

/// java.util.Random compatible generator used by Minicraft+ 2.2.4.
#[derive(Serialize, Deserialize)]
pub struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed as u64 ^ MULTIPLIER) & MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    pub fn next_float(&mut self) -> f64 {
        self.next(24) as f64 / (1_u32 << 24) as f64
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0);
        if bound & -bound == bound {
            return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let value = bits % bound;
            if bits.wrapping_sub(value).wrapping_add(bound - 1) >= 0 {
                return value;
            }
        }
    }

    pub fn next_bool(&mut self) -> bool {
        self.next(1) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::JavaRandom;

    #[test]
    fn matches_java_random_reference_sequence() {
        let mut random = JavaRandom::new(0);
        assert_eq!(random.next_int(100), 60);
        assert_eq!(random.next_int(100), 48);
        assert_eq!(random.next_int(100), 29);
    }
}
