//! A trait defining a bloom filter.

use std::hash::Hash;
use std::f32;

/// Get an optimal number of hashing functions to use from a given
/// number of bits per set member.
pub fn optimal_hashers(c: usize) -> usize {
    (c as f32 * 2.0f32.ln()).ceil() as usize
}

/// Calculate the probability of a false positive from the estimated
/// set size (`n`), the number of bits per item in the set (`c`), and
/// the number of hashing functions used (`k`).
pub fn false_positive_probability(n: usize, c: usize, k: usize) -> f64 {
    let e = 1.0f64.exp();
    let m = (n * c) as f64;
    let k = k as f64;

    (1f64 - e.powf((-k * n as f64) / m)).powf(k)
}

/// Bloom filters all need to support get and set operations.
pub trait BloomFilter<T: Hash> {
    /// The implementation name of the bloom filter.
    fn name(&self) -> &str {
        ""
    }

    /// Set the bits for `item` in the BloomFilter.
    fn mark(&mut self, item: &T);

    /// True if the bits for `item` in the BloomFilter are all set.
    fn check(&self, item: &T) -> bool;

    /// The estimated set size of the BloomFilter.
    fn set_size(&self) -> usize;

    /// The number of bits per member in the BloomFilter.
    fn bits_per_member(&self) -> usize;

    /// The number of hashing functions used.
    fn hash_count(&self) -> usize;
}
