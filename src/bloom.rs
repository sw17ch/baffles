//! A trait defining a bloom filter.

use std::hash::Hash;
use std::f32;


/// Get an optimal number of hashing functions to use from a given
/// number of bits per set member.
pub fn optimal_hashers(c: usize) -> usize {
    (c as f32 * 2.0f32.ln()).ceil() as usize
}

/// Bloom filters all need to support get and set operations.
pub trait BloomFilter<T: Hash> {
    /// Set the bits for `item` in the BloomFilter.
    fn mark(&mut self, item: &T);

    /// True if the bits for `item` in the BloomFilter are all set.
    fn check(&self, item: &T) -> bool;

}

#[cfg(test)]
mod tests {
    use super::*;
    use blocked::DefaultBlockedBloom;
    use standard::DefaultStandardBloom;

    fn ns() -> Vec<usize> {
        vec![1, 10, 1000, 10_000, 100_000]
            .iter()
            .map(|i| i * 1024usize)
            .collect()
    }

    fn bs<T: 'static + Hash>() -> Vec<Box<BloomFilter<T>>> {
        let c = 16;
        let k = optimal_hashers(c);

        let mut bs: Vec<Box<BloomFilter<T>>> = vec![];

        for n in ns() {
            bs.push(Box::new(DefaultStandardBloom::new(n, c, k)));
            bs.push(Box::new(DefaultBlockedBloom::new(n, c, k, 2)));
        }

        bs
    }

    #[test]
    fn test_all() {
        for mut b in bs() {
            let n = 1000;

            for i in 0..n {
                assert!(!b.check(&i));
            }
            for i in 0..n {
                b.mark(&i);
            }
            for i in 0..n {
                assert!(b.check(&i));
            }
        }
    }
}
