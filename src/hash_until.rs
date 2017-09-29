use std::hash::Hasher;

/// Starting with `initial`, continue passing the output of the hasher
/// into itself until `prop` returns true for one of them.
pub fn hash_until<H: Hasher, F: Fn(u64) -> bool>(h: &mut H, initial: u64, prop: F) -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    #[test]
    fn test_hash_until() {
        let mut h: DefaultHasher = Default::default();
        let prop = |h| (0xFF & h) > 128;

        assert!(129 == hash_until(&mut h, 129, &prop));
        assert!(128 < (0xFF & hash_until(&mut h, 127, &prop)));

    }
}
