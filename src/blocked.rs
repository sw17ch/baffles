//! The implementation for the Blocked Bloom Filter is taken from the
//! paper [Cache Efficient Bloom Filters for Shared Memory Machines by
//! Tim Kaler](http://tfk.mit.edu/pdf/bloom.pdf).
//!
//! The goal of a Blocked Bloom Filter is to achieve better
//! cache-related performance by dividing the set members evenly among
//! a number of Standard Bloom Filters that able to more-easily fit
//! into the machine cache.

use rand::Rng;
use rand;
use std::fmt;
use std::hash::{Hash, Hasher};
use std;
use standard::StandardBloom;
use index_mask::index_mask;
use hash_until::hash_until;

pub use bloom::{BloomFilter,BloomFilterCreate};

/// A representation of a BlockedBloom filter.
///
/// ```
/// use baffles::blocked::*;
///
/// let expected_set_size = 1024 * 1024;
/// let bits_per_item = 16;
/// let hashing_algos = (bits_per_item as f32 * 0.7).ceil() as usize;
/// let block_count = 8;
///
/// let mut dbb: DefaultBlockedBloom<usize> = BlockedBloom::new_with_blocks(
///     expected_set_size,
///     bits_per_item,
///     hashing_algos,
///     block_count);
///
/// assert!(!dbb.get(&100));
/// dbb.set(&100);
/// assert!(dbb.get(&100));
/// ```
pub struct BlockedBloom<H, T> {
    /// The blocks in this blocked bloom filter are just StandardBloom
    /// filters.
    blocks: Vec<StandardBloom<H, T>>,

    /// The block-selection hasher seed to use.
    hasher_seed: u64,

    /// A pre-computed bit-mask that is able to represent the number
    /// of blocks in the filter. This value will probably be larger
    /// than blocks.len().
    mask: u64,
}

impl<H, T> fmt::Debug for BlockedBloom<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BlockedBloom {{ blocks: {:?} }}", self.blocks)
    }
}

impl<H: Hasher + Default, T: Hash> BloomFilterCreate<T> for BlockedBloom<H, T> {
    fn new(n: usize, c: usize, k: usize) -> Self {
        // The math below caps the size of each block at 64K.
        let m = n as f64 * c as f64;
        let max_block_size = 64f64 * 1024f64;
        let block_count = (m / max_block_size).ceil() as usize;

        BlockedBloom::new_with_blocks(n, c, k, block_count)
    }
}

impl<H: Hasher + Default, T: Hash> BloomFilter<T> for BlockedBloom<H, T> {
    fn set(&mut self, item: &T) {
        let idx = self.block_idx(item);
        self.blocks[idx].set(item);
    }

    fn get(&self, item: &T) -> bool {
        let idx = self.block_idx(item);
        self.blocks[idx].get(item)
    }
}


impl<H: Hasher + Default, T: Hash> BlockedBloom<H, T> {
    /// Create a new blocked bloom filter.
    ///
    /// * `n`: estimate of the number of items in the set
    /// * `c`: number of bits in the filter for each item
    /// * `k`: number of hashing algorithms to use
    /// * `b`: the number of blocks to use
    ///
    /// `n` and `c` are multiplied together to determine how many
    /// total bits will be used in the bloom filter. `b` divides the
    /// total number of bits into discrete blocks. `k` and `c` can be
    /// scaled together to effect the false-positive rate for the
    /// whole system.
    ///
    /// The false positive rate can be generalized as follows:
    ///
    /// ```
    /// fn fp(n: f64, c: f64, k: f64) -> f64 {
    ///     let e = 1.0f64.exp();
    ///     let m = n * c;
    ///     (1f64 - e.powf((-k * n) / m)).powf(k)
    /// }
    ///
    /// assert!(fp(1000.0, 16.0, 4.0) > 0.0);
    /// ```
    pub fn new_with_blocks(n: usize, c: usize, k: usize, b: usize) -> Self {
        assert!(n > 0);
        assert!(c > 0);
        assert!(k > 0);
        assert!(b > 0);

        // Ideally, N insertions divide evenly into B. The number of
        // bits we use for each B should be (N/B * C).

        let n_per_block = (n as f32 / b as f32).ceil() as usize;
        let max_block_index = b - 1;

        assert!(n_per_block >= 1);

        let mut rng = rand::thread_rng();

        BlockedBloom {
            hasher_seed: rng.gen::<u64>(),
            mask: index_mask(max_block_index as u64),

            blocks: (0..b)
                .map(|_| {
                    StandardBloom::new_with_seeds(
                        n_per_block,
                        c,
                        k,
                        rng.gen::<u64>(),
                        rng.gen::<u64>(),
                    )
                })
                .collect(),
        }
    }

    /// Determine a block index from an item. The block index for a
    /// given item will always be the same.
    fn block_idx(&self, item: &T) -> usize {
        // We create a hash for the item by calculating hashes for the
        // item until one of those hashes is usable as a block index
        // after masking off the top bits.

        // A hasher with the block-picking seed.
        let mut h: H = Default::default();
        h.write_u64(self.hasher_seed);

        // Incorporate the item value into the hash.
        item.hash(&mut h);

        // The initial hash of the item.
        let initial = h.finish();

        // A property to test that a given hash is able to represent a
        // block index.
        let prop = |v| (self.mask & v) <= (self.blocks.len() - 1) as u64;

        // A hash that's able to represent a block index after masking
        // off the top bits.
        let usable_hash = hash_until(&mut h, initial, prop);

        (usable_hash & self.mask) as usize
    }
}

/// A BlockedBloom filter that uses the DefaultHasher.
pub type DefaultBlockedBloom<T> = BlockedBloom<std::collections::hash_map::DefaultHasher, T>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    // Expected set size.
    const N: usize = 128 * 1024;

    // Bits per element.
    const C: usize = 16;

    // Number of hashing algorithms to use.
    fn k() -> usize {
        (C as f32 * 0.7).ceil() as usize
    }

    // Natural logarithm.
    fn e() -> f64 {
        1.0f64.exp()
    }

    // Number of bits in the bloom filter.
    const M: usize = N * C;

    // Expected false positive rate.
    fn fp() -> f64 {
        let n = N as f64;
        let k = k() as f64;
        let m = M as f64;

        (1f64 - e().powf((-k * n) / m)).powf(k)
    }

    fn insert_n(bb: &mut DefaultBlockedBloom<usize>, n: usize) {
        for i in 0..n {
            bb.set(&i);
        }
    }

    fn test_n_to_m(bb: &DefaultBlockedBloom<usize>, n: usize, m: usize) -> usize {
        (n..m).fold(0, |acc, v| if bb.get(&v) { acc + 1 } else { acc })
    }

    #[test]
    fn it_should_have_standard_behavior_for_block_count_1() {
        let mut bb: DefaultBlockedBloom<usize> = BlockedBloom::new_with_blocks(N, C, k(), 1);
        insert_n(&mut bb, N);

        let fpos = test_n_to_m(&bb, N, N * 2) as f64;
        let n = N as f64;
        let false_positive_rate = fpos / n;

        println!(
            "false positive rate: {:.7}. expected {:.7}.",
            false_positive_rate,
            fp()
        );

        assert!(fp() * 2.0 > false_positive_rate);
    }

    #[test]
    fn it_should_have_standard_behavior_for_block_count_16() {
        let mut bb: BlockedBloom<DefaultHasher, usize> =
            BlockedBloom::new_with_blocks(N, C, k(), 16);
        insert_n(&mut bb, N);

        let fpos = test_n_to_m(&bb, N, N * 2) as f64;
        let n = N as f64;
        let false_positive_rate = fpos / n;

        println!(
            "false positive rate: {:.7}. expected {:.7}.",
            false_positive_rate,
            fp()
        );

        assert!(fp() * 2.0 > false_positive_rate);
    }

    #[test]
    fn it_should_have_standard_behavior_for_block_count_500() {
        let mut bb: BlockedBloom<DefaultHasher, usize> =
            BlockedBloom::new_with_blocks(N, C, k(), 500);
        insert_n(&mut bb, N);

        let fpos = test_n_to_m(&bb, N, N * 2) as f64;
        let n = N as f64;
        let false_positive_rate = fpos / n;

        println!(
            "false positive rate: {:.7}. expected {:.7}.",
            false_positive_rate,
            fp()
        );

        assert!(fp() * 2.0 > false_positive_rate);
    }
}
