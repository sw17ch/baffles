extern crate rand;

use std::hash::{Hash,Hasher};
use std::fmt;
use std::marker::PhantomData;

use rand::Rng;

mod bit_array;

struct Block<H,T> {
    /// The number of hashing functions to use. This also happens to
    /// be the number of bits that will be set in this block for each
    /// item.
    k: usize,

    /// The hashing function seeds to use.
    seed1: u64,
    seed2: u64,

    /// The bits in this block.
    bits: bit_array::BitArray,

    /// A mask to help select a random bit index.
    mask: u64,

    _p_hasher: PhantomData<H>,
    _p_type: PhantomData<T>,
}

impl<H,T> fmt::Debug for Block<H,T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Block {{ bits: {:?} }}", self.bits)
    }
}
impl<H: Hasher + Default, T: Hash> Block<H,T> {
    /// Create a new block that will use `k` hashing functions,
    /// `seed1` and `seed2` to derive those hashing functions, and
    /// space for `bits` bits.
    fn new(k: usize, seed1: u64, seed2: u64, bits: usize) -> Block<H,T> {
        assert!(k > 0);
        assert!(bits > 0);

        let max_bit_index = bits - 1;
        Block {
            k: k,

            seed1: seed1,
            seed2: seed2,

            bits: bit_array::BitArray::new(bits),
            mask: index_mask(max_bit_index as u64),

            _p_hasher: PhantomData,
            _p_type: PhantomData,
        }
    }

    /// Create a list of bit indicies representing the bloom filter
    /// hash for `item`.
    fn hash(&self, item: &T) -> Vec<usize> {
        let mut h1: H = Default::default();
        let mut h2: H = Default::default();
        h1.write_u64(self.seed1);
        h2.write_u64(self.seed2);

        item.hash(&mut h1);
        item.hash(&mut h2);

        let ih1 = h1.finish();
        let ih2 = h2.finish();

        let mut v = vec![0;self.k];
        for i in 0..self.k {
            // A. Kirsch and M. Mitzenmacher describe a way to
            // generate multiple hashes without having to recompute
            // every time in their paper "Less Hashing, Same
            // Performance: Building a Better Bloom Filter" published
            // September 2008. It's generalized below as:
            //
            //    hi = h1 + (i * h2)
            //
            // Their paper identifies that this mechanism allows us to
            // calculate two hashes once, and derive any number of
            // hashes from those initial two without losing entropy in
            // each successive hash.
            //
            // We generate this k_and_m hash and then test whether or
            // not it's a suitable candidate for producing a random
            // bit index. In order to treat all indicies fairly, the
            // hash is recalculated until masking off the top bits of
            // the hash produces a number that's less than or equal to
            // the number of bits in the block.

            // The value for the i'th hash.
            let k_and_m = ih1.wrapping_add((i as u64).wrapping_mul(ih2));

            // The hasher used for looping.
            let mut h3: H = Default::default();

            // This will be true when the hash can be used to produce
            // a random bit index.
            let prop = |h| (self.mask & h) <= (self.bits.width() - 1) as u64;

            // This hash, when masked, will give us a usable bit
            // index.
            let usable_hash = hash_until(&mut h3, k_and_m, prop);

            // Store the bit index into the vector.
            v[i] = (self.mask & usable_hash) as usize;
        }

        v
    }

    /// The bits for `item` in the block.
    fn set(&mut self, item: &T) {
        for ix in self.hash(item) {
            self.bits.set(ix);
        }
    }

    /// True if the bits for `item` are already set in the block.
    fn get(&self, item: &T) -> bool {
        self.hash(item).iter().all(|ix| self.bits.get(*ix))
    }
}

/// A representation of a BlockedBloom filter.
///
/// ```
/// use baffles::{BlockedBloom,DefaultBlockedBloom};
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
/// assert!(!dbb.get(&100));
/// dbb.set(&100);
/// assert!(dbb.get(&100));
/// ```
pub struct BlockedBloom<H,T> {
    /// The blocks in this blocked bloom filter.
    blocks: Vec<Block<H,T>>,

    /// The block-selection hasher seed to use.
    hasher_seed: u64,

    /// A pre-computed bit-mask that is able to represent the number
    /// of blocks in the filter. This value will probably be larger
    /// than blocks.len().
    mask: u64
}

pub type DefaultBlockedBloom<T> = BlockedBloom<std::collections::hash_map::DefaultHasher,T>;

impl<H,T> fmt::Debug for BlockedBloom<H,T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BlockedBloom {{ blocks: {:?} }}", self.blocks)
    }
}

impl<H: Hasher + Default, T: Hash> BlockedBloom<H,T> {
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
    /// ```
    pub fn new(n: usize, c: usize, k: usize, b: usize) -> Self {
        assert!(n > 0);
        assert!(c > 0);
        assert!(k > 0);
        assert!(b > 0);

        // Ideally, N insertions divide evenly into B. The number of
        // bits we use for each B should be (N/B * C).

        let n_per_block = n as f32 / b as f32;
        let bits_per_block = (n_per_block * c as f32).ceil() as usize;
        let max_block_index = b - 1;

        assert!(bits_per_block >= 1);

        let mut rng = rand::thread_rng();

        BlockedBloom {
            hasher_seed: rng.gen::<u64>(),
            mask: index_mask(max_block_index as u64),

            blocks: (0..b).map (|_| {
                Block::new(k, rng.gen::<u64>(), rng.gen::<u64>(), bits_per_block)
            }).collect(),
        }
    }

    /// Set the bits for `item` in the filter.
    pub fn set(&mut self, item: &T) {
        // Set the bits for the item in the specified block.
        let idx = self.block_idx(item);
        self.blocks[idx].set(item);
    }

    /// True if the bits for `item` are already set in the filter.
    pub fn get(&self, item: &T) -> bool{
        // Set the bits for the item in the specified block.
        let idx = self.block_idx(item);
        self.blocks[idx].get(item)
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

/// Starting with `initial`, continue passing the output of the hasher
/// into itself until `prop` returns true for one of them.
fn hash_until<H: Hasher, F: Fn(u64) -> bool>(h: &mut H, initial: u64, prop: F) -> u64 {
    if prop(initial) {
        // If the initial hash value already meets our constraint, it
        // is our result. We don't need to do any more work.
        initial
    } else {
        // If the initial hash value does not meet our constraint, then
        // we'll create a new hasher and seed it with our initial value.
        h.write_u64(initial);
        let mut r = h.finish();

        loop {
            // Now we'll keep feeding the result of the hash back into
            // the hasher until we get a value that fits our
            // constraint.
            if prop(r) {
                break;
            } else {
                h.write_u64(r);
                r = h.finish();
            }
        }

        r
    }
}

/// Calculate a mask suitable for representing all bits of a
/// value. (There are faster ways to do this, but we don't calculate
/// this often, so we're using the obvious approach.)
fn index_mask(value: u64) -> u64 {
    (1..64)
        .map(|i| { (1 << i) - 1 })
        .find(|m| { *m >= value as u64 })
        .unwrap_or(u64::max_value())
}

#[test]
fn test_index_mask() {
    assert!(u64::max_value() == index_mask(1 << 63));
    assert!(u64::max_value() == index_mask(10 + (1 << 63)));
    assert!((1 << 62) - 1 == index_mask((1 << 61) + 424242));
    assert!(1 == index_mask(0));
    assert!(1 == index_mask(1));
    assert!(3 == index_mask(2));
}

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
        (n..m).fold(0, |acc, v| {
            if bb.get(&v) {
                acc + 1
            } else {
                acc
            }
        })
    }

    #[test]
    fn it_should_have_standard_behavior_for_block_count_1() {
        let mut bb: DefaultBlockedBloom<usize> = BlockedBloom::new(N,C,k(),1);
        insert_n(&mut bb, N);

        let fpos = test_n_to_m(&bb, N, N * 2) as f64;
        let n = N as f64;
        let false_positive_rate = fpos / n;

        println!("false positive rate: {:.7}. expected {:.7}.",
                 false_positive_rate, fp());

        assert!(fp() * 2.0 > false_positive_rate);
    }

    #[test]
    fn it_should_have_standard_behavior_for_block_count_16() {
        let mut bb: BlockedBloom<DefaultHasher,usize> = BlockedBloom::new(N,C,k(),16);
        insert_n(&mut bb, N);

        let fpos = test_n_to_m(&bb, N, N * 2) as f64;
        let n = N as f64;
        let false_positive_rate = fpos / n;

        println!("false positive rate: {:.7}. expected {:.7}.",
                 false_positive_rate, fp());

        assert!(fp() * 2.0 > false_positive_rate);
    }

    #[test]
    fn it_should_have_standard_behavior_for_block_count_500() {
        let mut bb: BlockedBloom<DefaultHasher,usize> = BlockedBloom::new(N,C,k(),500);
        insert_n(&mut bb, N);

        let fpos = test_n_to_m(&bb, N, N * 2) as f64;
        let n = N as f64;
        let false_positive_rate = fpos / n;

        println!("false positive rate: {:.7}. expected {:.7}.",
                 false_positive_rate, fp());

        assert!(fp() * 2.0 > false_positive_rate);
    }
}
