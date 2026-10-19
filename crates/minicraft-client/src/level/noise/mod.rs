//! Java-compatible random and level-noise algorithms.

use serde::{Deserialize, Serialize};

const MULTIPLIER: u64 = 0x5DEECE66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1_u64 << 48) - 1;

/// java.util.Random compatible generator used by Minicraft+ 2.2.4.
#[derive(Clone, Serialize, Deserialize)]
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
        if (bound as u32).is_power_of_two() {
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

    /// java.util.Random.nextInt() — full-range 32-bit value.
    pub fn next_i32(&mut self) -> i32 {
        self.next(32)
    }

    /// Random.nextLong uses two signed int draws, including sign extension of
    /// the low word (not a concatenation of two unsigned words).
    pub fn next_i64(&mut self) -> i64 {
        (i64::from(self.next(32)) << 32).wrapping_add(i64::from(self.next(32)))
    }

    pub(crate) fn state(&self) -> u64 {
        self.seed
    }

    pub fn next_double(&mut self) -> f64 {
        (((self.next(26) as u64) << 27) + self.next(27) as u64) as f64 / (1_u64 << 53) as f64
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

    #[test]
    fn next_i32_matches_java_random_next_int() {
        // java.util.Random(0).nextInt()
        assert_eq!(JavaRandom::new(0).next_i32(), -1_155_484_576);
        let mut random = JavaRandom::new(0);
        let first = random.next_i32();
        let second = random.next_i32();
        assert_ne!(first, second);
        assert_eq!(JavaRandom::new(0).next_i32(), first);
    }
}
