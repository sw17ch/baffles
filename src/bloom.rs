//! A trait defining a bloom filter.

use std::hash::Hash;

/// Bloom filters can be created using a few basic methods. For now,
/// we just allow creation using `n`, `c`, and `k`.
pub trait BloomFilterCreate<T: Hash> {
    /// Create a new BloomFilter with estimated set size `k`, using
    /// `c` bits per element, and using `k` hashing functions.
    fn new(n: usize, c: usize, k: usize) -> Self;
}

/// Bloom filters all need to support get and set operations.
pub trait BloomFilter<T: Hash> {
    /// Set the bits for `item` in the BloomFilter.
    fn set(&mut self, item: &T);

    /// True if the bits for `item` in the BloomFilter are all set.
    fn get(&self, item: &T) -> bool;
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

    fn k(c: f32) -> usize {
        (c * 0.7).ceil() as usize
    }

    fn bs<T: 'static + Hash>() -> Vec<Box<BloomFilter<T>>> {
        let c = 16;
        let k = k(c as f32);

        let mut bs: Vec<Box<BloomFilter<T>>> = vec![];

        for n in ns() {
            bs.push(Box::new(DefaultStandardBloom::new(n, c, k)));
            bs.push(Box::new(DefaultBlockedBloom::new(n, c, k)));
        }

        bs
    }

    #[test]
    fn test_all() {
        for mut b in bs() {
            let n = 1000;

            for i in 0..n {
                assert!(!b.get(&i));
            }
            for i in 0..n {
                b.set(&i);
            }
            for i in 0..n {
                assert!(b.get(&i));
            }
        }
    }
}
