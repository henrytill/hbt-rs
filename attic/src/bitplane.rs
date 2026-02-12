//! Shared word-level operations and constants for pos/neg bitplane encoding.

pub(crate) const BITS_LOG2: u32 = 6;
pub(crate) const BITS_MASK: usize = 2_usize.pow(BITS_LOG2) - 1;

#[inline]
pub(crate) const fn words_needed(n: usize) -> usize {
    (n + BITS_MASK) >> BITS_LOG2
}

#[inline]
pub(crate) const fn tail_mask(n: usize) -> u64 {
    let r = n & BITS_MASK;
    if r == 0 { u64::MAX } else { (1u64 << r) - 1 }
}

#[inline]
pub(crate) const fn pair(w: usize) -> std::ops::Range<usize> {
    2 * w..2 * w + 2
}

#[inline]
pub(crate) const fn not_word(pos: u64, neg: u64) -> (u64, u64) {
    (neg, pos)
}

#[inline]
pub(crate) const fn and_word(a_pos: u64, a_neg: u64, b_pos: u64, b_neg: u64) -> (u64, u64) {
    (a_pos & b_pos, a_neg | b_neg)
}

#[inline]
pub(crate) const fn or_word(a_pos: u64, a_neg: u64, b_pos: u64, b_neg: u64) -> (u64, u64) {
    (a_pos | b_pos, a_neg & b_neg)
}

#[inline]
pub(crate) const fn merge_word(a_pos: u64, a_neg: u64, b_pos: u64, b_neg: u64) -> (u64, u64) {
    (a_pos | b_pos, a_neg | b_neg)
}
