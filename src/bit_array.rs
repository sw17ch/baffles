use std;
use std::fmt;

type Word = u64;

pub struct BitArray {
    bits: usize,
    backing: Vec<Word>,
}

impl fmt::Debug for BitArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BitArray {{ bits: ")?;
        for w in &self.backing {
            write!(f, "{:#016X} ", w)?;
        }

        write!(f, " }}")
    }
}

fn bits_in_word() -> usize {
    8 * std::mem::size_of::<Word>()
}

fn word_index_for_bit(bit: usize) -> usize {
    bit / bits_in_word()
}

impl BitArray {
    pub fn new(bit_count: usize) -> BitArray {
        assert!(bit_count > 0);

        let max_index = bit_count - 1;
        let words_needed_for_bits = word_index_for_bit(max_index) + 1;
        BitArray {
            bits: bit_count,
            backing: vec![0; words_needed_for_bits],
        }
    }

    pub fn set_to(&mut self, bit: usize, state: bool) {
        assert!(bit < self.bits);
        let word_ix = word_index_for_bit(bit);
        let bit_ix = bit % bits_in_word();
        let set_mask = 1 << bit_ix;

        if state {
            self.backing[word_ix] |= set_mask;
        } else {
            self.backing[word_ix] &= !set_mask;
        }
    }

    #[allow(dead_code)]
    pub fn set(&mut self, bit: usize) {
        self.set_to(bit, true)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self, bit: usize) {
        self.set_to(bit, false)
    }

    pub fn get(&self, bit: usize) -> bool {
        assert!(bit < self.bits);
        let word_ix = word_index_for_bit(bit);
        let bit_ix = bit % bits_in_word();
        let set_mask = 1 << bit_ix;

        set_mask == self.backing[word_ix] & set_mask
    }

    pub fn width(&self) -> usize {
        self.bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_index_for_bit() {
        assert!(word_index_for_bit(0) == 0);
        assert!(word_index_for_bit(10) == 0);
        assert!(word_index_for_bit(63) == 0);
        assert!(word_index_for_bit(64) == 1);
        assert!(word_index_for_bit(70) == 1);
    }

    #[test]
    #[should_panic]
    fn test_set_out_of_range() {
        let mut ba = BitArray::new(1);
        ba.set_to(1, true);
    }

    #[test]
    #[should_panic]
    fn test_get_out_of_range() {
        let ba = BitArray::new(1);
        ba.get(1);
    }

    #[test]
    fn test_set_and_get() {
        let mut ba = BitArray::new(1);

        assert!(!ba.get(0));
        ba.set(0);
        assert!(ba.get(0));
        ba.clear(0);
        assert!(!ba.get(0));
    }
}
