//! Deterministic PRNG — PCG32 with an explicit 64-bit seed + stream.
//!
//! Integer-only math, identical output on every platform/implementation.
//! This is the ONLY randomness source for puzzle generation.

/// splitmix64 — used to derive generator seeds from puzzle IDs.
pub const fn splitmix64(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Derive a generator seed from a puzzle id (stable for all time).
pub fn seed_from_id(id: u64) -> u64 {
    let mut x = id ^ 0x6A09_E667_F3BC_C90B;
    splitmix64(&mut x)
}

/// PCG32 (O'Neill), standard increment scheme.
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64, stream: u64) -> Self {
        let inc = (stream << 1) | 1;
        let mut rng = Pcg32 { state: 0, inc };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }

    /// Uniform in [0, n) via Lemire's method on a 128-bit product
    /// (unbiased, no floats, deterministic).
    pub fn range(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        let n128 = n as u128;
        let threshold = n128.wrapping_neg() % n128; // 2^64 mod n
        loop {
            let x = self.next_u64() as u128;
            let m = x * n128; // 64×64 → 128
            let low = (m & 0xFFFF_FFFF_FFFF_FFFF) as u128;
            if low >= threshold {
                return (m >> 64) as usize;
            }
        }
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.range(i + 1);
            slice.swap(i, j);
        }
    }

    pub fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[self.range(slice.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_across_instances() {
        let a: Vec<u32> = {
            let mut r = Pcg32::new(42, 7);
            (0..100).map(|_| r.next_u32()).collect()
        };
        let b: Vec<u32> = {
            let mut r = Pcg32::new(42, 7);
            (0..100).map(|_| r.next_u32()).collect()
        };
        assert_eq!(a, b);
    }

    #[test]
    fn different_streams_differ() {
        let mut a = Pcg32::new(42, 1);
        let mut b = Pcg32::new(42, 2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn range_in_bounds() {
        let mut r = Pcg32::new(1, 1);
        for _ in 0..10_000 {
            assert!(r.range(13) < 13);
        }
    }

    #[test]
    fn seed_from_id_stable() {
        assert_eq!(seed_from_id(25), seed_from_id(25));
        assert_ne!(seed_from_id(25), seed_from_id(26));
    }
}
