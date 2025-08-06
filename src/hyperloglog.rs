//! # HyperLogLog
//!
//! `hyperloglog` port from [redis's implementation](https://github.com/redis/redis/blob/4930d19e70c391750479951022e207e19111eb55/src/hyperloglog.c)
//! Some codes are borrowed from:
//! 1. https://github.com/crepererum/pdatastructs.rs/blob/3997ed50f6b6871c9e53c4c5e0f48f431405fc63/src/hyperloglog.rs
//! 2. https://github.com/apache/arrow-datafusion/blob/f203d863f5c8bc9f133f6dd9b2e34e57ac3cdddc/datafusion/physical-expr/src/aggregate/hyperloglog.rs

use crate::Hasher;
use core::hash::Hash;

/// By default, we use 2**14 registers like redis
pub const DEFAULT_P: usize = 14_usize;

/// Note: We don't make HyperLogLog as static struct by keeping `PhantomData<T>`
/// Callers should take care of its hash function to be unchanged.
/// P is the bucket number, must be [4, 18]
/// Q = 64 - P
/// Register num is 1 << P
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperLogLog<const P: usize = DEFAULT_P> {
    pub(crate) registers: Vec<u8>,
}

impl<const P: usize> Default for HyperLogLog<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const P: usize> HyperLogLog<P> {
    /// note that this method should not be invoked in untrusted environment
    pub fn new() -> Self {
        assert!(
            (P >= 4) & (P <= 18),
            "P ({}) must be larger or equal than 4 and smaller or equal than 18",
            P
        );

        Self {
            registers: vec![0; 1 << P],
        }
    }

    pub fn with_registers(registers: Vec<u8>) -> Self {
        assert_eq!(registers.len(), Self::number_registers());

        Self { registers }
    }

    /// Adds an hash to the HyperLogLog.
    /// hash value is dertermined by caller
    #[inline]
    pub fn add_hash(&mut self, hash: u64) {
        let index = (hash & Self::register_mask()) as usize;
        let one_position = ((hash >> P) | (1_u64 << Self::q())).trailing_zeros() + 1;
        self.registers[index] = self.registers[index].max(one_position as u8);
    }

    /// Adds an object to the HyperLogLog.
    /// Though we could pass different types into this method, caller should notice that
    pub fn add_object<T: ?Sized + Hash>(&mut self, obj: &T) {
        self.add_object_by_hasher::<T, ahash::AHasher>(obj);
    }

    #[inline]
    pub fn add_object_by_hasher<T: ?Sized + Hash, H: Hasher>(&mut self, obj: &T) {
        let hash = H::hll_hash(obj);
        self.add_hash(hash);
    }

    /// Merge the other [`HyperLogLog`] into this one
    pub fn merge(&mut self, other: &Self) {
        for i in 0..self.registers.len() {
            self.registers[i] = self.registers[i].max(other.registers[i]);
        }
    }

    /// Get the register histogram (each value in register index into
    /// the histogram
    #[inline]
    fn get_histogram(&self) -> [u32; 64] {
        let mut histogram = [0; 64];
        // hopefully this can be unrolled
        for r in &self.registers {
            histogram[*r as usize] += 1;
        }
        histogram
    }

    /// Guess the number of unique elements seen by the HyperLogLog.
    #[inline]
    pub fn count(&self) -> usize {
        let histogram = self.get_histogram();
        let m = Self::number_registers() as f64;
        let q = Self::q();
        let mut z = m * hll_tau((m - histogram[q + 1] as f64) / m);
        for i in histogram[1..=q].iter().rev() {
            z += *i as f64;
            z *= 0.5;
        }
        z += m * hll_sigma(histogram[0] as f64 / m);

        (0.5 / 2_f64.ln() * m * m / z).round() as usize
    }

    #[inline]
    fn q() -> usize {
        64 - P
    }

    #[inline]
    fn register_mask() -> u64 {
        Self::number_registers() as u64 - 1
    }

    #[inline]
    pub fn number_registers() -> usize {
        1 << P
    }

    #[inline]
    pub fn error_rate() -> f64 {
        1.04f64 / (Self::number_registers() as f64).sqrt()
    }

    #[inline]
    pub fn max_byte_size() -> usize {
        Self::number_registers()
    }

    #[inline]
    pub fn num_empty_registers(&self) -> usize {
        self.registers.iter().filter(|x| **x == 0).count()
    }
}

/// Helper function sigma as defined in
/// "New cardinality estimation algorithms for HyperLogLog sketches"
/// Otmar Ertl, https://arxiv.org/abs/1702.01284
#[allow(dead_code)]
#[inline]
fn hll_sigma(x: f64) -> f64 {
    if x == 1. {
        f64::INFINITY
    } else {
        let mut y = 1.0;
        let mut z = x;
        let mut x = x;
        loop {
            x *= x;
            let z_prime = z;
            z += x * y;
            y += y;

            if z_prime == z {
                break;
            }
        }
        z
    }
}

/// Helper function tau as defined in
/// "New cardinality estimation algorithms for HyperLogLog sketches"
/// Otmar Ertl, https://arxiv.org/abs/1702.01284
#[inline]
fn hll_tau(x: f64) -> f64 {
    if x == 0.0 || x == 1.0 {
        0.0
    } else {
        let mut y = 1.0;
        let mut z = 1.0 - x;
        let mut x = x;
        loop {
            x = x.sqrt();
            let z_prime = z;
            y *= 0.5;
            z -= (1.0 - x).powi(2) * y;
            if z_prime == z {
                break;
            }
        }
        z / 3.0
    }
}

#[cfg(test)]
mod tests {
    use crate::HyperLogLog;

    const P: usize = 14;
    const NUM_REGISTERS: usize = 1 << P;

    fn compare_with_delta(got: usize, expected: usize) {
        let expected = expected as f64;
        let diff = (got as f64) - expected;
        let diff = diff.abs() / expected;
        // times 6 because we want the tests to be stable
        // so we allow a rather large margin of error
        // this is adopted from redis's unit test version as well
        let margin = 1.04 / ((NUM_REGISTERS as f64).sqrt()) * 6.0;
        assert!(
            diff <= margin,
            "{} is not near {} percent of {} which is ({}, {})",
            got,
            margin,
            expected,
            expected * (1.0 - margin),
            expected * (1.0 + margin)
        );
    }

    macro_rules! sized_number_test {
        ($SIZE: expr, $T: tt) => {{
            let mut hll = HyperLogLog::<P>::new();
            for i in 0..$SIZE {
                hll.add_object(&(i as $T));
            }
            compare_with_delta(hll.count(), $SIZE);
        }};
    }

    macro_rules! typed_large_number_test {
        ($SIZE: expr) => {{
            sized_number_test!($SIZE, u64);
            sized_number_test!($SIZE, u128);
            sized_number_test!($SIZE, i64);
            sized_number_test!($SIZE, i128);
        }};
    }

    macro_rules! typed_number_test {
        ($SIZE: expr) => {{
            sized_number_test!($SIZE, u16);
            sized_number_test!($SIZE, u32);
            sized_number_test!($SIZE, i16);
            sized_number_test!($SIZE, i32);
            typed_large_number_test!($SIZE);
        }};
    }

    #[test]
    fn test_empty() {
        let hll = HyperLogLog::<P>::new();
        assert_eq!(hll.count(), 0);
    }

    #[test]
    fn test_one() {
        let mut hll = HyperLogLog::<P>::new();
        hll.add_hash(1);
        assert_eq!(hll.count(), 1);
    }

    #[test]
    fn test_number_100() {
        typed_number_test!(100);
    }

    #[test]
    fn test_number_1k() {
        typed_number_test!(1_000);
    }

    #[test]
    fn test_number_10k() {
        typed_number_test!(10_000);
    }

    #[test]
    fn test_number_100k() {
        typed_large_number_test!(100_000);
    }

    #[test]
    fn test_number_1m() {
        typed_large_number_test!(1_000_000);
    }

    #[test]
    fn test_empty_merge() {
        let mut hll = HyperLogLog::<P>::new();
        hll.merge(&HyperLogLog::<P>::new());
        assert_eq!(hll.count(), 0);
    }

    #[test]
    fn test_merge_overlapped() {
        let mut hll = HyperLogLog::<P>::new();
        for i in 0..1000 {
            hll.add_object(&i);
        }

        let other = HyperLogLog::<P>::new();
        for i in 0..1000 {
            hll.add_object(&i);
        }

        hll.merge(&other);
        compare_with_delta(hll.count(), 1000);
    }

    #[test]
    fn test_repetition() {
        let mut hll = HyperLogLog::<P>::new();
        for i in 0..1_000_000 {
            hll.add_object(&(i % 1000));
        }
        compare_with_delta(hll.count(), 1000);
    }

    macro_rules! custom_hasher_test {
        ($SIZE: expr, $H: ty, $T: tt) => {{
            let mut hll = HyperLogLog::<P>::new();
            for i in 0..$SIZE {
                hll.add_object_by_hasher::<$T, $H>(&(i as $T));
            }
            compare_with_delta(hll.count(), $SIZE);
        }};
    }

    #[test]
    fn test_xxhash_hll() {
        use core::hash::{BuildHasher, Hash};
        #[derive(Default)]
        struct XXH3;
        impl crate::Hasher for XXH3 {
            fn hll_hash<T: Hash>(x: T) -> u64 {
                let builder = xxhash_rust::xxh3::Xxh3Builder::default();
                builder.hash_one(x)
            }
        }

        #[derive(Default)]
        struct XXH3WithSeed;
        const SEED: u64 = 0x1234_5678_u64;

        impl crate::Hasher for XXH3WithSeed {
            fn hll_hash<T: Hash>(x: T) -> u64 {
                let builder = xxhash_rust::xxh3::Xxh3Builder::default().with_seed(SEED);
                builder.hash_one(x)
            }
        }

        custom_hasher_test!(1000, XXH3, u16);
        custom_hasher_test!(1000, XXH3, i32);
        custom_hasher_test!(1000, XXH3, i64);

        custom_hasher_test!(1000, XXH3WithSeed, u16);
        custom_hasher_test!(1000, XXH3WithSeed, i32);
        custom_hasher_test!(1000, XXH3WithSeed, i64);
    }
}
