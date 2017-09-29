/// Calculate a mask suitable for representing all bits of a
/// value. (There are faster ways to do this, but we don't calculate
/// this often, so we're using the obvious approach.)
pub fn index_mask(value: u64) -> u64 {
    (1..64)
        .map(|i| (1 << i) - 1)
        .find(|m| *m >= value as u64)
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
