mod hyperloglog;

#[cfg(feature = "serde_borsh")]
mod serde;

use ahash::RandomState;
use hyperloglog::DEFAULT_P;

pub type HyperLogLog<const P: usize = DEFAULT_P> = hyperloglog::HyperLogLog<P>;

use core::hash::Hash;
pub trait Hasher {
    fn hll_hash<T: Hash>(x: T) -> u64
    where
        Self: Sized;
}

/// Fixed seed
const SEED: RandomState = RandomState::with_seeds(
    0x355e438b4b1478c7_u64,
    0xd0e8453cd135b473_u64,
    0xf7b252066a57836a_u64,
    0xb8a829e3713c09bf_u64,
);

impl Hasher for ahash::AHasher {
    fn hll_hash<T: Hash>(x: T) -> u64 {
        SEED.hash_one(x)
    }
}
