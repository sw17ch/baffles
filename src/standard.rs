//! This implementation of a Standard Bloom Filter is fairly basic. If
//! I'm honest, I learned how implement bloom filters at all from the
//! paper [Cache Efficient Bloom Filters for Shared Memory Machines by
//! Tim Kaler](http://tfk.mit.edu/pdf/bloom.pdf). Their basic
//! structure, however, is not that compliated.

use rand::Rng;
use rand;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std;
use bit_array::BitArray;
use index_mask::index_mask;
use hash_until::hash_until;

pub use bloom::BloomFilter;

/// A representation of a StandardBloom filter.
///
/// ```
/// use baffles::standard::*;
///
/// let expected_set_size = 1024 * 1024;
/// let bits_per_item = 16;
/// let hashing_algos = (bits_per_item as f32 * 0.7).ceil() as usize;
///
/// let mut dbb: DefaultStandardBloom<usize> = StandardBloom::new(
///     expected_set_size,
///     bits_per_item,
///     hashing_algos);
///
/// assert!(!dbb.check(&100));
/// dbb.mark(&100);
/// assert!(dbb.check(&100));
/// ```
pub struct StandardBloom<H, T> {
    /// The number of hashing functions to use. This also happens to
    /// be the number of bits that will be set in this block for each
    /// item.
    k: usize,

    /// The hashing function seeds to use.
    seed1: u64,
    seed2: u64,

    /// The bits in this block.
    bits: BitArray,

    /// A mask to help select a random bit index.
    mask: u64,

    /// The estimated set size.
    n: usize,

    /// The number of bits per member.
    c: usize,

    _p_hasher: PhantomData<H>,
    _p_type: PhantomData<T>,
}

pub type DefaultStandardBloom<T> = StandardBloom<std::collections::hash_map::DefaultHasher, T>;

impl<H, T> fmt::Debug for StandardBloom<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StandardBloom {{ bits: {:?} }}", self.bits)
    }
}

impl<H: Hasher + Default, T: Hash> BloomFilter<T> for StandardBloom<H, T> {
    fn name(&self) -> &str {
        "standard"
    }

    fn mark(&mut self, item: &T) {
        for ix in self.hash(item) {
            self.bits.set(ix);
        }
    }

    fn check(&self, item: &T) -> bool {
        self.hash(item).iter().all(|ix| self.bits.get(*ix))
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

impl<H: Hasher + Default, T: Hash> StandardBloom<H, T> {
    /// Create a new StandardBloom filter that with an approximate set
    /// size of `n`, uses `c` bits per member, and `k` hashing
    /// functions.
    pub fn new(n: usize, c: usize, k: usize) -> Self {
        let mut rng = rand::thread_rng();
        StandardBloom::new_with_seeds(n, c, k, rng.gen::<u64>(), rng.gen::<u64>())
    }

    /// Like `new`, but allows the specification of the seeds to use
    /// for the hashers.
    pub fn new_with_seeds(
        n: usize,
        c: usize,
        k: usize,
        seed1: u64,
        seed2: u64,
    ) -> StandardBloom<H, T> {
        assert!(k > 0);
        assert!(n * c > 0);

        assert!(k <= c);

        let bits = n * c;

        let max_bit_index = bits - 1;
        StandardBloom {
            n: n,
            c: c,
            k: k,

            seed1: seed1,
            seed2: seed2,

            bits: BitArray::new(bits),
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

        let mut v = vec![0; self.k];
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
}

#[cfg(test)]
mod tests {
    use bloom::optimal_hashers;
    use super::*;

    #[test]
    fn the_basics_work() {
        let mut bb: DefaultStandardBloom<usize> =
            StandardBloom::new(1024 * 1024, 16, optimal_hashers(16));
        assert!(!bb.check(&100));
        bb.mark(&100);
        assert!(bb.check(&100));
    }
}
