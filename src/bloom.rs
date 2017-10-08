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

#[cfg(test)]
mod tests {
    use super::*;
    use blocked::DefaultBlockedBloom;
    use standard::DefaultStandardBloom;
    use std::ops::DerefMut;

    fn bs(n: usize) -> Vec<Box<BloomFilter<usize>>> {
        let c = 16;
        let k = optimal_hashers(c);

        vec![
            Box::new(DefaultStandardBloom::new(n, c, k)),
            Box::new(DefaultBlockedBloom::new(n, c, k, 2)),
            Box::new(DefaultBlockedBloom::new(n, c, k, 16)),
        ]
    }

    fn has_standard_behavior(bf: &mut BloomFilter<usize>) -> bool {
        let n = bf.set_size();
        let c = bf.bits_per_member();
        let k = bf.hash_count();
        let fp = false_positive_probability(n, c, k);

        for i in 0..n {
            bf.mark(&i);
        }

        let false_positives =
            (n..(n * 2)).fold(0, |acc, v| if bf.check(&v) { acc + 1 } else { acc });
        let false_positive_ratio = false_positives as f64 / n as f64;

        let double_fp = fp * 2.0;

        println!("{}: {:.7} > {:.7}", n, double_fp, false_positive_ratio);

        double_fp > false_positive_ratio
    }

    #[test]
    fn test_standard_for_10k() {
        for mut b in bs(10 * 1024) {
            assert!(has_standard_behavior(b.deref_mut()));
        }
    }

    #[test]
    fn test_standard_for_100k() {
        for mut b in bs(100 * 1024) {
            assert!(has_standard_behavior(b.deref_mut()));
        }
    }
}
