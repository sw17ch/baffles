# Baffles

A collection of [Bloom
filters](https://en.wikipedia.org/wiki/Bloom_filter) written in Rust.

The name (kindly conceived of by Scott Vokes) is borrowed from [Sound
Baffles](https://en.wikipedia.org/wiki/Sound_baffle) which are used to
reduce the strength of airborne sound. Similarly, a major use of Bloom
filters has historically been to reduce the number I/O operations
required on a disk.

## Filters Provided

### Standard Bloom Filter

This is a no-surprises [Bloom
Filter](https://en.wikipedia.org/wiki/Bloom_filter).

```rust
use baffles::standard::*;

let expected_set_size = 1024 * 1024;
let bits_per_item = 16;
let hashing_algos = (bits_per_item as f32 * 0.7).ceil() as usize;

let mut dbb: DefaultStandardBloom<usize> = StandardBloom::new(
    expected_set_size,
    bits_per_item,
    hashing_algos);

assert!(!dbb.check(&100));
dbb.mark(&100);
assert!(dbb.check(&100));
```

### Blocked Bloom Filter

See [Cache Efficient Bloom Filters for Shared Memory Machines by Tim
Kaler](http://tfk.mit.edu/pdf/bloom.pdf).

```rust
use baffles::blocked::*;

let expected_set_size = 1024 * 1024;
let bits_per_item = 16;
let hashing_algos = (bits_per_item as f32 * 0.7).ceil() as usize;
let block_count = 8;

let mut dbb: DefaultBlockedBloom<usize> = BlockedBloom::new(
    expected_set_size,
    bits_per_item,
    hashing_algos,
    block_count);

assert!(!dbb.check(&100));
dbb.mark(&100);
assert!(dbb.check(&100));
```
