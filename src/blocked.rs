//! The implementation for the Blocked Bloom Filter is taken from the
//! paper [Cache Efficient Bloom Filters for Shared Memory Machines by
//! Tim Kaler](http://tfk.mit.edu/pdf/bloom.pdf).
//!
//! The goal of a Blocked Bloom Filter is to achieve better
//! cache-related performance by dividing the set members evenly among
//! a number of Standard Bloom Filters that able to more-easily fit
//! into the machine cache.

use hash_until::hash_until;
use index_mask::index_mask;
use rand::Rng;
use rand;
use standard::StandardBloom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std;

pub use bloom::BloomFilter;

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
/// let mut dbb: DefaultBlockedBloom<usize> = BlockedBloom::new(
///     expected_set_size,
///     bits_per_item,
///     hashing_algos,
///     block_count);
///
/// assert!(!dbb.check(&100));
/// dbb.mark(&100);
/// assert!(dbb.check(&100));
/// ```
pub struct BlockedBloom<H, T> {
    /// The blocks in this blocked bloom filter are just StandardBloom
    /// filters.
    blocks: Vec<Option<Box<StandardBloom<H, T>>>>,

    /// The block-selection hasher seed to use.
    hasher_seed: u64,

    /// A pre-computed bit-mask that is able to represent the number
    /// of blocks in the filter. This value will probably be larger
    /// than blocks.len().
    mask: u64,

    /// The RNG used to generate differnet seeds.
    rng: rand::ThreadRng,

    /// The estimated set size.
    n: usize,

    /// The number of bits per member.
    c: usize,

    /// The number of hashing functions.
    k: usize,

    /// The number of N used for each block.
    n_per_block: usize,
}

impl<H, T> fmt::Debug for BlockedBloom<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BlockedBloom {{ blocks: {:?} }}", self.blocks)
    }
}

impl<H: Hasher + Default, T: Hash> BloomFilter<T> for BlockedBloom<H, T> {
    fn name(&self) -> &str {
        "blocked"
    }

    fn mark(&mut self, item: &T) {
        let idx = self.block_idx(item);

        if self.blocks[idx].is_none() {
            let new_block = create_block(self.n_per_block, self.c, self.k, &mut self.rng);
            self.blocks[idx] = Some(new_block);
        }

        match self.blocks[idx].as_mut() {
            Some(b) => b.mark(item),
            None => panic!("This should never happen."),
        }
    }

    fn check(&self, item: &T) -> bool {
        let idx = self.block_idx(item);

        match &self.blocks[idx] {
            &Some(ref b) => b.check(item),
            &None => false,
        }
    }

    fn set_size(&self) -> usize {
        self.n
    }

    fn bits_per_member(&self) -> usize {
        self.c
    }

    fn hash_count(&self) -> usize {
        self.k
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
    pub fn new(n: usize, c: usize, k: usize, b: usize) -> Self {
        assert!(n > 0);
        assert!(c > 0);
        assert!(k > 0);
        assert!(b > 0);

        // Ideally, N insertions divide evenly into B. The number of
        // bits we use for each B should be (N/B * C).

        let max_block_index = b - 1;

        let mut rng = rand::thread_rng();

        BlockedBloom {
            n: n,
            c: c,
            k: k,

            n_per_block: (n as f32 / b as f32).ceil() as usize,

            hasher_seed: rng.gen::<u64>(),
            mask: index_mask(max_block_index as u64),

            rng: rng,

            blocks: (0..b).map(|_| None).collect(),
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

fn create_block<H, T>(
    n_per_block: usize,
    c: usize,
    k: usize,
    rng: &mut rand::ThreadRng,
) -> Box<StandardBloom<H, T>>
where
    H: Hasher + Default,
    T: Hash,
{
    Box::new(StandardBloom::new_with_seeds(
        n_per_block,
        c,
        k,
        rng.gen::<u64>(),
        rng.gen::<u64>(),
    ))
}

#[cfg(test)]
mod tests {
    use bloom::optimal_hashers;
    use super::*;

    #[test]
    fn the_basics_work() {
        let mut bb: DefaultBlockedBloom<usize> =
            BlockedBloom::new(1024 * 1024, 16, optimal_hashers(16), 4);
        assert!(!bb.check(&100));
        bb.mark(&100);
        assert!(bb.check(&100));
    }
}
