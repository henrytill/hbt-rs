//! Belnap's four-valued logic: scalar type and packed bitvector.

use std::cmp::Ordering;

/// A single Belnap truth value.
///
/// Uses `#[repr(u8)]` with discriminants encoding `(neg_bit << 1) | pos_bit`:
///
/// | pos | neg | bits   | variant   |
/// |-----|-----|--------|-----------|
/// | 0   | 0   | `0b00` | `Unknown` |
/// | 1   | 0   | `0b01` | `True`    |
/// | 0   | 1   | `0b10` | `False`   |
/// | 1   | 1   | `0b11` | `Both`    |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(strum::EnumIter))]
#[repr(u8)]
pub enum Belnap {
    Unknown = 0b00, // pos=0, neg=0
    True = 0b01,    // pos=1, neg=0
    False = 0b10,   // pos=0, neg=1
    Both = 0b11,    // pos=1, neg=1
}

const FROM_BITS: [Belnap; 4] = [
    Belnap::Unknown, // 0b00
    Belnap::True,    // 0b01
    Belnap::False,   // 0b10
    Belnap::Both,    // 0b11
];

impl From<Belnap> for u8 {
    fn from(v: Belnap) -> u8 {
        v as u8
    }
}

impl From<Belnap> for u64 {
    fn from(v: Belnap) -> u64 {
        v as u64
    }
}

impl Belnap {
    /// Returns `true` if this value carries any information (not [`Belnap::Unknown`]).
    #[must_use]
    pub const fn is_known(self) -> bool {
        self as u8 != 0
    }

    /// Returns `true` if this value is exactly [`Belnap::True`] or [`Belnap::False`].
    #[must_use]
    pub const fn is_determined(self) -> bool {
        let b = self as u8;
        (b & 1) ^ (b >> 1) != 0
    }

    /// Returns `true` if this value is [`Belnap::Both`] (contradicted).
    #[must_use]
    pub const fn is_contradicted(self) -> bool {
        self as u8 == 0b11
    }

    /// Converts to `bool` if the value is exactly [`Belnap::True`] or [`Belnap::False`].
    #[must_use]
    pub const fn to_bool(self) -> Option<bool> {
        match self {
            Belnap::True => Some(true),
            Belnap::False => Some(false),
            _ => None,
        }
    }

    /// Truth-ordering meet: logical AND.
    #[must_use]
    pub fn and(self, other: Belnap) -> Belnap {
        let (a, b) = (u8::from(self), u8::from(other));
        let r_pos = (a & 1) & (b & 1);
        let r_neg = (a >> 1) | (b >> 1);
        FROM_BITS[usize::from((r_neg << 1) | r_pos)]
    }

    /// Truth-ordering join: logical OR.
    #[must_use]
    pub fn or(self, other: Belnap) -> Belnap {
        let (a, b) = (u8::from(self), u8::from(other));
        let r_pos = (a & 1) | (b & 1);
        let r_neg = (a >> 1) & (b >> 1);
        FROM_BITS[usize::from((r_neg << 1) | r_pos)]
    }

    /// Knowledge-ordering meet: keep only information both sources agree on.
    #[must_use]
    pub fn consensus(self, other: Belnap) -> Belnap {
        FROM_BITS[usize::from(u8::from(self) & u8::from(other))]
    }

    /// Knowledge-ordering join: combine observations from independent sources.
    #[must_use]
    pub fn merge(self, other: Belnap) -> Belnap {
        FROM_BITS[usize::from(u8::from(self) | u8::from(other))]
    }

    /// Logical implication: equivalent to `(!self).or(rhs)`.
    #[must_use]
    pub fn implies(self, rhs: Belnap) -> Belnap {
        (!self).or(rhs)
    }
}

impl std::fmt::Display for Belnap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Belnap::Unknown => "Unknown",
            Belnap::True => "True",
            Belnap::False => "False",
            Belnap::Both => "Both",
        })
    }
}

impl std::ops::Not for Belnap {
    type Output = Belnap;

    fn not(self) -> Belnap {
        match self {
            Belnap::True => Belnap::False,
            Belnap::False => Belnap::True,
            Belnap::Unknown => Belnap::Unknown,
            Belnap::Both => Belnap::Both,
        }
    }
}

/// Viewed in the truth lattice: `False < {Unknown, Both} < True`.
///
/// `Unknown` and `Both` are incomparable, so `partial_cmp` returns `None`
/// for that pair.
///
/// `BitAnd` and `BitOr` are the truth-lattice meet (logical AND) and join
/// (logical OR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsTruth<T>(pub T);

impl PartialOrd for AsTruth<Belnap> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let meet = self.0.and(other.0);
        match (meet == self.0, meet == other.0) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl std::ops::BitAnd for AsTruth<Belnap> {
    type Output = AsTruth<Belnap>;

    fn bitand(self, rhs: AsTruth<Belnap>) -> AsTruth<Belnap> {
        AsTruth(self.0.and(rhs.0))
    }
}

impl std::ops::BitOr for AsTruth<Belnap> {
    type Output = AsTruth<Belnap>;

    fn bitor(self, rhs: AsTruth<Belnap>) -> AsTruth<Belnap> {
        AsTruth(self.0.or(rhs.0))
    }
}

/// Viewed in the knowledge lattice: `Unknown < {True, False} < Both`.
///
/// `True` and `False` are incomparable, so `partial_cmp` returns `None`
/// for that pair.
///
/// `BitAnd` and `BitOr` are the knowledge-lattice meet (consensus) and join
/// (merge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsKnowledge<T>(pub T);

impl PartialOrd for AsKnowledge<Belnap> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let meet = self.0.consensus(other.0);
        match (meet == self.0, meet == other.0) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl std::ops::BitAnd for AsKnowledge<Belnap> {
    type Output = AsKnowledge<Belnap>;

    fn bitand(self, rhs: AsKnowledge<Belnap>) -> AsKnowledge<Belnap> {
        AsKnowledge(self.0.consensus(rhs.0))
    }
}

impl std::ops::BitOr for AsKnowledge<Belnap> {
    type Output = AsKnowledge<Belnap>;

    fn bitor(self, rhs: AsKnowledge<Belnap>) -> AsKnowledge<Belnap> {
        AsKnowledge(self.0.merge(rhs.0))
    }
}

// -- Bitplane helpers (used by BelnapVec) --

const BITS_LOG2: u32 = 6;
const BITS_MASK: usize = (1 << BITS_LOG2) - 1;

const _: () = assert!(BITS_MASK == 63);

/// Returns the number of 64-bit word pairs needed to store `n` Belnap values.
#[inline]
const fn words_needed(n: usize) -> usize {
    (n + BITS_MASK) >> BITS_LOG2
}

const _: () = {
    assert!(words_needed(0) == 0);
    assert!(words_needed(1) == 1);
    assert!(words_needed(64) == 1);
    assert!(words_needed(65) == 2);
};

/// Returns a bitmask selecting the active bits in the last word of a vector of
/// width `n`.
///
/// When `n` is a multiple of 64, the last word is fully used and all bits are
/// active, so the mask is `u64::MAX`. Otherwise only the low `n % 64` bits
/// are active.
#[inline]
const fn tail_mask(n: usize) -> u64 {
    let r = n & BITS_MASK;
    if r == 0 { u64::MAX } else { (1u64 << r) - 1 }
}

const _: () = {
    assert!(tail_mask(0) == u64::MAX);
    assert!(tail_mask(64) == u64::MAX);
    assert!(tail_mask(1) == 0b01);
    assert!(tail_mask(4) == 0b1111);
    assert!(tail_mask(63) == u64::MAX >> 1);
};

#[inline]
const fn pair(w: usize) -> std::ops::Range<usize> {
    let base = 2 * w;
    base..base + 2
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfBounds;

impl std::fmt::Display for OutOfBounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("index out of bounds")
    }
}

impl std::error::Error for OutOfBounds {}

/// Packed Belnap bitvector: two-bitplane representation.
///
/// Each bit position encodes a [`Belnap`] value using the same `(pos, neg)`
/// scheme described on that type.
///
/// Uses an interleaved layout: `[pos_0, neg_0, pos_1, neg_1, ...]`.
/// Invariant: unused high bits in the last word pair are always zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BelnapVec {
    width: usize,
    words: Vec<u64>,
}

impl BelnapVec {
    /// Creates a vector of `width` elements, all [`Belnap::Unknown`].
    #[must_use]
    pub fn new(width: usize) -> BelnapVec {
        let nw = words_needed(width);
        BelnapVec {
            width,
            words: vec![0; 2 * nw],
        }
    }

    fn mask_tail(&mut self) {
        let nw = words_needed(self.width);
        if nw > 0 {
            let m = tail_mask(self.width);
            let pn = &mut self.words[pair(nw - 1)];
            pn[0] &= m;
            pn[1] &= m;
        }
    }

    fn filled(width: usize, fill: Belnap) -> BelnapVec {
        let bits = u64::from(fill);
        let fill_pos = u64::MAX * (bits & 1);
        let fill_neg = u64::MAX * (bits >> 1);
        let nw = words_needed(width);
        let mut words = Vec::with_capacity(2 * nw);
        for _ in 0..nw {
            words.push(fill_pos);
            words.push(fill_neg);
        }
        let mut v = BelnapVec { width, words };
        v.mask_tail();
        v
    }

    #[must_use]
    pub fn all_true(width: usize) -> BelnapVec {
        BelnapVec::filled(width, Belnap::True)
    }

    #[must_use]
    pub fn all_false(width: usize) -> BelnapVec {
        BelnapVec::filled(width, Belnap::False)
    }

    #[must_use]
    pub fn all_both(width: usize) -> BelnapVec {
        BelnapVec::filled(width, Belnap::Both)
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    // Bulk resize

    pub fn truncate(&mut self, new_width: usize) {
        if new_width >= self.width {
            return;
        }
        self.width = new_width;
        let nw = words_needed(new_width);
        self.words.truncate(2 * nw);
        self.mask_tail();
    }

    pub fn resize(&mut self, new_width: usize, fill: Belnap) {
        if new_width <= self.width {
            self.truncate(new_width);
            return;
        }
        let old_nw = words_needed(self.width);
        let new_nw = words_needed(new_width);
        let bits = u64::from(fill);
        let fill_pos = u64::MAX * (bits & 1);
        let fill_neg = u64::MAX * (bits >> 1);
        // Fill remaining bits in the current last word pair
        if old_nw > 0 && self.width & BITS_MASK != 0 {
            let fill_mask = !tail_mask(self.width);
            let pn = &mut self.words[pair(old_nw - 1)];
            pn[0] |= fill_pos & fill_mask;
            pn[1] |= fill_neg & fill_mask;
        }
        self.words
            .extend(std::iter::repeat_n([fill_pos, fill_neg], new_nw - old_nw).flatten());
        self.width = new_width;
        if fill.is_known() {
            self.mask_tail();
        }
    }

    // Scalar access

    #[inline]
    #[must_use]
    fn get_unchecked(&self, i: usize) -> Belnap {
        debug_assert!(i < self.width);
        let w = i >> BITS_LOG2;
        let b = i & BITS_MASK;
        let pn = &self.words[pair(w)];
        let pos_bit = ((pn[0] >> b) & 1) as usize;
        let neg_bit = ((pn[1] >> b) & 1) as usize;
        FROM_BITS[(neg_bit << 1) | pos_bit]
    }

    /// # Errors
    ///
    /// Returns [`OutOfBounds`] if `i >= self.width()`.
    pub fn get(&self, i: usize) -> Result<Belnap, OutOfBounds> {
        if i >= self.width {
            return Err(OutOfBounds);
        }
        Ok(self.get_unchecked(i))
    }

    #[inline]
    fn set_unchecked(&mut self, i: usize, v: Belnap) {
        debug_assert!(i < self.width);
        let w = i >> BITS_LOG2;
        let b = i & BITS_MASK;
        let pn = &mut self.words[pair(w)];
        let mask = 1u64 << b;
        let v = u64::from(v);
        let pos = (v & 1) << b;
        let neg = (v >> 1) << b;
        pn[0] = (pn[0] & !mask) | pos;
        pn[1] = (pn[1] & !mask) | neg;
    }

    /// Sets the value at index `i`. If `i >= self.width()`, the vector grows
    /// to width `i + 1`, with intermediate positions filled with [`Belnap::Unknown`].
    pub fn set(&mut self, i: usize, v: Belnap) {
        if i >= self.width {
            let new_width = i + 1;
            let new_nw = words_needed(new_width);
            self.words.resize(2 * new_nw, 0u64);
            self.width = new_width;
        }
        self.set_unchecked(i, v);
    }

    // Bulk operations

    #[must_use]
    pub fn not(&self) -> BelnapVec {
        let mut words = self.words.clone();
        for pn in words.chunks_exact_mut(2) {
            pn.swap(0, 1);
        }
        BelnapVec {
            width: self.width,
            words,
        }
    }

    /// Per-plane bitwise combine. `f_pos` and `f_neg` are applied independently
    /// to the positive and negative bitplanes; missing words on the shorter
    /// operand are treated as zero (i.e. [`Belnap::Unknown`]).
    //
    // Generic over `Fn` rather than `fn(u64, u64) -> u64` so each closure inlines
    // into the inner loop instead of going through an indirect call. F and G are
    // separate type parameters because each closure literal has its own anonymous
    // type — a single parameter would force both arguments to coincide.
    fn binop<F, G>(&self, other: &BelnapVec, f_pos: F, f_neg: G) -> BelnapVec
    where
        F: Fn(u64, u64) -> u64,
        G: Fn(u64, u64) -> u64,
    {
        let width = self.width.max(other.width);
        let nw = words_needed(width);
        let mut words = vec![0u64; 2 * nw];
        for w in 0..nw {
            let (sp, sn) = self.words.get(pair(w)).map_or((0, 0), |p| (p[0], p[1]));
            let (op, on) = other.words.get(pair(w)).map_or((0, 0), |p| (p[0], p[1]));
            let out = &mut words[pair(w)];
            out[0] = f_pos(sp, op);
            out[1] = f_neg(sn, on);
        }
        BelnapVec { width, words }
    }

    #[must_use]
    pub fn and(&self, other: &BelnapVec) -> BelnapVec {
        self.binop(other, |a, b| a & b, |a, b| a | b)
    }

    #[must_use]
    pub fn or(&self, other: &BelnapVec) -> BelnapVec {
        self.binop(other, |a, b| a | b, |a, b| a & b)
    }

    /// Knowledge-ordering meet: keep only information both sources agree on.
    #[must_use]
    pub fn consensus(&self, other: &BelnapVec) -> BelnapVec {
        self.binop(other, |a, b| a & b, |a, b| a & b)
    }

    /// Knowledge-ordering join: combine observations from independent sources.
    #[must_use]
    pub fn merge(&self, other: &BelnapVec) -> BelnapVec {
        self.binop(other, |a, b| a | b, |a, b| a | b)
    }

    #[must_use]
    pub fn implies(&self, other: &BelnapVec) -> BelnapVec {
        self.not().or(other)
    }

    // Queries

    /// Returns `true` if no position is [`Belnap::Both`].
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        for pn in self.words.chunks_exact(2) {
            if pn[0] & pn[1] != 0 {
                return false;
            }
        }
        true
    }

    /// Returns `true` if `active(pn)` covers all bits in every word pair.
    /// Full words must equal `u64::MAX`; the last word must equal `tail_mask`.
    fn all_words<F>(&self, active: F) -> bool
    where
        F: Fn(&[u64]) -> u64,
    {
        let nw = words_needed(self.width);
        if nw == 0 {
            return true;
        }
        for pn in self.words[..2 * (nw - 1)].chunks_exact(2) {
            if active(pn) != u64::MAX {
                return false;
            }
        }
        active(&self.words[pair(nw - 1)]) == tail_mask(self.width)
    }

    /// Returns `true` if every position is [`Belnap::True`] or [`Belnap::False`].
    #[must_use]
    pub fn is_all_determined(&self) -> bool {
        self.all_words(|pn| pn[0] ^ pn[1])
    }

    #[must_use]
    pub fn is_all_true(&self) -> bool {
        self.all_words(|pn| pn[0] & !pn[1])
    }

    #[must_use]
    pub fn is_all_false(&self) -> bool {
        self.all_words(|pn| !pn[0] & pn[1])
    }

    #[must_use]
    pub fn count_true(&self) -> usize {
        self.words
            .chunks_exact(2)
            .map(|pn| (pn[0] & !pn[1]).count_ones() as usize)
            .sum()
    }

    #[must_use]
    pub fn count_false(&self) -> usize {
        self.words
            .chunks_exact(2)
            .map(|pn| (!pn[0] & pn[1]).count_ones() as usize)
            .sum()
    }

    #[must_use]
    pub fn count_both(&self) -> usize {
        self.words
            .chunks_exact(2)
            .map(|pn| (pn[0] & pn[1]).count_ones() as usize)
            .sum()
    }

    #[must_use]
    pub fn count_unknown(&self) -> usize {
        let known: usize = self
            .words
            .chunks_exact(2)
            .map(|pn| (pn[0] | pn[1]).count_ones() as usize)
            .sum();
        self.width - known
    }

    /// Returns the index of the first occurrence of `needle`, or `None` if absent.
    #[must_use]
    pub fn find_first(&self, needle: Belnap) -> Option<usize> {
        let nw = words_needed(self.width);
        if nw == 0 {
            return None;
        }
        let bits = u8::from(needle);
        let want_pos = (bits & 1) != 0;
        let want_neg = (bits >> 1) != 0;
        let last = nw - 1;
        let tail = tail_mask(self.width);
        for w in 0..nw {
            let pn = &self.words[pair(w)];
            let pos_match = if want_pos { pn[0] } else { !pn[0] };
            let neg_match = if want_neg { pn[1] } else { !pn[1] };
            let mut m = pos_match & neg_match;
            // Mask the last word to suppress garbage past `width`. For non-Unknown
            // needles the invariant already keeps those bits at 0, but Unknown
            // matches `(0, 0)` and would otherwise hit the padding.
            if w == last {
                m &= tail;
            }
            if m != 0 {
                return Some(w * 64 + m.trailing_zeros() as usize);
            }
        }
        None
    }

    /// Returns an iterator over all elements in index order.
    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        Iter { vec: self, next: 0 }
    }
}

/// Iterator over a [`BelnapVec`]'s elements in index order.
pub struct Iter<'a> {
    vec: &'a BelnapVec,
    next: usize,
}

impl Iterator for Iter<'_> {
    type Item = Belnap;

    fn next(&mut self) -> Option<Belnap> {
        if self.next >= self.vec.width {
            return None;
        }
        let v = self.vec.get_unchecked(self.next);
        self.next += 1;
        Some(v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.width - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl<'a> IntoIterator for &'a BelnapVec {
    type Item = Belnap;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl From<&[Belnap]> for BelnapVec {
    fn from(xs: &[Belnap]) -> BelnapVec {
        let mut v = BelnapVec::new(xs.len());
        for (i, &x) in xs.iter().enumerate() {
            v.set_unchecked(i, x);
        }
        v
    }
}

impl std::ops::Not for &BelnapVec {
    type Output = BelnapVec;

    fn not(self) -> BelnapVec {
        BelnapVec::not(self)
    }
}

impl std::ops::Not for BelnapVec {
    type Output = BelnapVec;

    fn not(self) -> BelnapVec {
        BelnapVec::not(&self)
    }
}

macro_rules! impl_lattice_binop {
    ($wrapper:ident, $trait:ident, $method:ident, $inherent:ident) => {
        impl std::ops::$trait for $wrapper<&BelnapVec> {
            type Output = $wrapper<BelnapVec>;

            fn $method(self, rhs: $wrapper<&BelnapVec>) -> $wrapper<BelnapVec> {
                $wrapper(self.0.$inherent(rhs.0))
            }
        }

        impl std::ops::$trait<$wrapper<&BelnapVec>> for $wrapper<BelnapVec> {
            type Output = $wrapper<BelnapVec>;

            fn $method(self, rhs: $wrapper<&BelnapVec>) -> $wrapper<BelnapVec> {
                $wrapper(self.0.$inherent(rhs.0))
            }
        }

        impl std::ops::$trait<$wrapper<BelnapVec>> for $wrapper<&BelnapVec> {
            type Output = $wrapper<BelnapVec>;

            fn $method(self, rhs: $wrapper<BelnapVec>) -> $wrapper<BelnapVec> {
                $wrapper(self.0.$inherent(&rhs.0))
            }
        }

        impl std::ops::$trait for $wrapper<BelnapVec> {
            type Output = $wrapper<BelnapVec>;

            fn $method(self, rhs: $wrapper<BelnapVec>) -> $wrapper<BelnapVec> {
                $wrapper(self.0.$inherent(&rhs.0))
            }
        }
    };
}

impl_lattice_binop!(AsTruth, BitAnd, bitand, and);
impl_lattice_binop!(AsTruth, BitOr, bitor, or);
impl_lattice_binop!(AsKnowledge, BitAnd, bitand, consensus);
impl_lattice_binop!(AsKnowledge, BitOr, bitor, merge);

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn scalar_not_truth_table() {
        use Belnap::*;
        let expected: [Belnap; 4] = [Unknown, False, True, Both];
        for (i, a) in Belnap::iter().enumerate() {
            assert_eq!(!a, expected[i], "!{a:?}");
        }
    }

    #[test]
    fn scalar_and_truth_table() {
        use Belnap::*;
        #[rustfmt::skip]
        let expected: [[Belnap; 4]; 4] = [
            /*      U         T        F      B    */
            /* U */ [Unknown, Unknown, False, False],
            /* T */ [Unknown, True,    False, Both ],
            /* F */ [False,   False,   False, False],
            /* B */ [False,   Both,    False, Both ],
        ];
        for (i, a) in Belnap::iter().enumerate() {
            for (j, b) in Belnap::iter().enumerate() {
                assert_eq!(a.and(b), expected[i][j], "{a:?}.and({b:?})");
            }
        }
    }

    #[test]
    fn scalar_or_truth_table() {
        use Belnap::*;
        #[rustfmt::skip]
        let expected: [[Belnap; 4]; 4] = [
            /*      U         T     F        B   */
            /* U */ [Unknown, True, Unknown, True],
            /* T */ [True,    True, True,    True],
            /* F */ [Unknown, True, False,   Both],
            /* B */ [True,    True, Both,    Both],
        ];
        for (i, a) in Belnap::iter().enumerate() {
            for (j, b) in Belnap::iter().enumerate() {
                assert_eq!(a.or(b), expected[i][j], "{a:?}.or({b:?})");
            }
        }
    }

    #[test]
    fn scalar_newtype_operators_agree() {
        // BitAnd/BitOr on AsTruth dispatch to the truth lattice (and / or).
        // BitAnd/BitOr on AsKnowledge dispatch to the knowledge lattice
        // (consensus / merge).
        for a in Belnap::iter() {
            for b in Belnap::iter() {
                assert_eq!(AsTruth(a) & AsTruth(b), AsTruth(a.and(b)));
                assert_eq!(AsTruth(a) | AsTruth(b), AsTruth(a.or(b)));
                assert_eq!(AsKnowledge(a) & AsKnowledge(b), AsKnowledge(a.consensus(b)));
                assert_eq!(AsKnowledge(a) | AsKnowledge(b), AsKnowledge(a.merge(b)));
            }
        }
    }

    #[test]
    fn scalar_merge_truth_table() {
        use Belnap::*;
        #[rustfmt::skip]
        let expected: [[Belnap; 4]; 4] = [
            /*      U         T      F      B   */
            /* U */ [Unknown, True,  False, Both],
            /* T */ [True,    True,  Both,  Both],
            /* F */ [False,   Both,  False, Both],
            /* B */ [Both,    Both,  Both,  Both],
        ];
        for (i, a) in Belnap::iter().enumerate() {
            for (j, b) in Belnap::iter().enumerate() {
                assert_eq!(a.merge(b), expected[i][j], "{a:?}.merge({b:?})");
            }
        }
    }

    #[test]
    fn scalar_consensus_truth_table() {
        use Belnap::*;
        #[rustfmt::skip]
        let expected: [[Belnap; 4]; 4] = [
            /*      U         T        F        B      */
            /* U */ [Unknown, Unknown, Unknown, Unknown],
            /* T */ [Unknown, True,    Unknown, True   ],
            /* F */ [Unknown, Unknown, False,   False  ],
            /* B */ [Unknown, True,    False,   Both   ],
        ];
        for (i, a) in Belnap::iter().enumerate() {
            for (j, b) in Belnap::iter().enumerate() {
                assert_eq!(a.consensus(b), expected[i][j], "{a:?}.consensus({b:?})");
            }
        }
    }

    #[test]
    fn scalar_implies_truth_table() {
        use Belnap::*;
        #[rustfmt::skip]
        let expected: [[Belnap; 4]; 4] = [
            /*      U         T     F        B   */
            /* U */ [Unknown, True, Unknown, True],
            /* T */ [Unknown, True, False,   Both],
            /* F */ [True,    True, True,    True],
            /* B */ [True,    True, Both,    Both],
        ];
        for (i, a) in Belnap::iter().enumerate() {
            for (j, b) in Belnap::iter().enumerate() {
                assert_eq!(a.implies(b), expected[i][j], "{a:?}.implies({b:?})");
            }
        }
    }

    #[test]
    fn scalar_display() {
        assert_eq!(Belnap::Unknown.to_string(), "Unknown");
        assert_eq!(Belnap::True.to_string(), "True");
        assert_eq!(Belnap::False.to_string(), "False");
        assert_eq!(Belnap::Both.to_string(), "Both");
    }

    #[test]
    fn scalar_leq_truth_table() {
        // Truth order: False < {Unknown, Both} < True; Unknown and Both incomparable.
        #[rustfmt::skip]
        let expected: [[bool; 4]; 4] = [
            /*      U       T      F      B    */
            /* U */ [true,  true,  false, false],
            /* T */ [false, true,  false, false],
            /* F */ [true,  true,  true,  true ],
            /* B */ [false, true,  false, true ],
        ];
        for (i, a) in Belnap::iter().enumerate() {
            for (j, b) in Belnap::iter().enumerate() {
                assert_eq!(
                    AsTruth(a) <= AsTruth(b),
                    expected[i][j],
                    "AsTruth({a:?}) <= AsTruth({b:?})"
                );
            }
        }
    }

    #[test]
    fn scalar_leq_knowledge_table() {
        // Knowledge order: Unknown < {True, False} < Both; True and False incomparable.
        #[rustfmt::skip]
        let expected: [[bool; 4]; 4] = [
            /*      U       T      F      B    */
            /* U */ [true,  true,  true,  true ],
            /* T */ [false, true,  false, true ],
            /* F */ [false, false, true,  true ],
            /* B */ [false, false, false, true ],
        ];
        for (i, a) in Belnap::iter().enumerate() {
            for (j, b) in Belnap::iter().enumerate() {
                assert_eq!(
                    AsKnowledge(a) <= AsKnowledge(b),
                    expected[i][j],
                    "AsKnowledge({a:?}) <= AsKnowledge({b:?})"
                );
            }
        }
    }

    #[test]
    fn scalar_partial_cmp_incomparable() {
        // Truth lattice: Unknown and Both are incomparable.
        assert_eq!(
            AsTruth(Belnap::Unknown).partial_cmp(&AsTruth(Belnap::Both)),
            None
        );
        assert_eq!(
            AsTruth(Belnap::Both).partial_cmp(&AsTruth(Belnap::Unknown)),
            None
        );
        // Knowledge lattice: True and False are incomparable.
        assert_eq!(
            AsKnowledge(Belnap::True).partial_cmp(&AsKnowledge(Belnap::False)),
            None
        );
        assert_eq!(
            AsKnowledge(Belnap::False).partial_cmp(&AsKnowledge(Belnap::True)),
            None
        );
    }

    #[test]
    fn scalar_queries() {
        use Belnap::*;
        assert!(!Unknown.is_known());
        assert!(True.is_known());
        assert!(False.is_known());
        assert!(Both.is_known());

        assert!(!Unknown.is_determined());
        assert!(True.is_determined());
        assert!(False.is_determined());
        assert!(!Both.is_determined());

        assert!(!Unknown.is_contradicted());
        assert!(!True.is_contradicted());
        assert!(!False.is_contradicted());
        assert!(Both.is_contradicted());

        assert_eq!(Unknown.to_bool(), None);
        assert_eq!(True.to_bool(), Some(true));
        assert_eq!(False.to_bool(), Some(false));
        assert_eq!(Both.to_bool(), None);
    }

    #[test]
    fn vec_get_set_all_four() {
        let mut v = BelnapVec::new(4);
        v.set(0, Belnap::Unknown);
        v.set(1, Belnap::True);
        v.set(2, Belnap::False);
        v.set(3, Belnap::Both);
        assert_eq!(v.get(0).unwrap(), Belnap::Unknown);
        assert_eq!(v.get(1).unwrap(), Belnap::True);
        assert_eq!(v.get(2).unwrap(), Belnap::False);
        assert_eq!(v.get(3).unwrap(), Belnap::Both);
    }

    #[test]
    fn vec_bulk_and() {
        let a = BelnapVec::all_true(64);
        let b = BelnapVec::all_false(64);
        let c = a.and(&b);
        assert!(c.is_all_false());
    }

    #[test]
    fn vec_bulk_or() {
        let a = BelnapVec::all_false(64);
        let b = BelnapVec::all_true(64);
        let c = a.or(&b);
        assert!(c.is_all_true());
    }

    #[test]
    fn vec_bulk_not() {
        let a = BelnapVec::all_true(100);
        let b = a.not();
        assert!(b.is_all_false());
        let c = b.not();
        assert!(c.is_all_true());
    }

    #[test]
    fn vec_bulk_merge() {
        let a = BelnapVec::all_true(64);
        let b = BelnapVec::all_false(64);
        let c = a.merge(&b);
        // Merging True and False should give Both everywhere
        assert_eq!(c.count_both(), 64);
        assert_eq!(c.count_true(), 0);
        assert_eq!(c.count_false(), 0);
        assert_eq!(c.count_unknown(), 0);
    }

    #[test]
    fn vec_bulk_consensus() {
        let a = BelnapVec::all_true(64);
        let b = BelnapVec::all_false(64);
        let c = a.consensus(&b);
        // Consensus of True and False should give Unknown everywhere
        assert_eq!(c.count_unknown(), 64);
        assert_eq!(c.count_true(), 0);
        assert_eq!(c.count_false(), 0);
        assert_eq!(c.count_both(), 0);
    }

    #[test]
    fn vec_is_consistent() {
        let a = BelnapVec::all_true(64);
        assert!(a.is_consistent());

        let mut b = BelnapVec::new(10);
        b.set(0, Belnap::True);
        b.set(1, Belnap::False);
        assert!(b.is_consistent());

        b.set(2, Belnap::Both);
        assert!(!b.is_consistent());
    }

    #[test]
    fn vec_is_all_determined() {
        let mut v = BelnapVec::new(4);
        v.set(0, Belnap::True);
        v.set(1, Belnap::False);
        v.set(2, Belnap::True);
        v.set(3, Belnap::False);
        assert!(v.is_all_determined());

        v.set(3, Belnap::Unknown);
        assert!(!v.is_all_determined());

        v.set(3, Belnap::Both);
        assert!(!v.is_all_determined());
    }

    #[test]
    fn vec_counts() {
        let mut v = BelnapVec::new(10);
        v.set(0, Belnap::True);
        v.set(1, Belnap::True);
        v.set(2, Belnap::False);
        v.set(3, Belnap::Both);
        assert_eq!(v.count_true(), 2);
        assert_eq!(v.count_false(), 1);
        assert_eq!(v.count_both(), 1);
        assert_eq!(v.count_unknown(), 6);
    }

    #[test]
    fn vec_word_boundaries() {
        // Element 63 is bit 63 (sign bit) of word-pair 0.
        let mut v = BelnapVec::new(65);
        v.set(63, Belnap::Both);
        assert_eq!(v.get(63), Ok(Belnap::Both));
        assert_eq!(v.get(62), Ok(Belnap::Unknown));
        assert_eq!(v.get(64), Ok(Belnap::Unknown));

        // Element 64 is bit 0 of word-pair 1.
        v.set(64, Belnap::True);
        assert_eq!(v.get(64), Ok(Belnap::True));
        assert_eq!(v.get(63), Ok(Belnap::Both));
    }

    #[test]
    fn vec_width_63() {
        // width=63 exercises r=63 in tail_mask: the largest non-aligned width.
        let v = BelnapVec::all_true(63);
        assert!(v.is_all_true());
        assert!(v.is_all_determined());
        assert!(v.is_consistent());
        assert_eq!(v.get(62), Ok(Belnap::True));

        let merged = v.merge(&BelnapVec::all_false(63));
        assert_eq!(merged.count_both(), 63);
    }

    #[test]
    fn vec_auto_grow() {
        let mut v = BelnapVec::new(10);
        v.set(100, Belnap::Both);
        assert_eq!(v.width(), 101);
        assert_eq!(v.get(100), Ok(Belnap::Both));
        assert_eq!(v.get(50), Ok(Belnap::Unknown));
        assert_eq!(v.get(200), Err(OutOfBounds));
    }

    #[test]
    fn vec_resize() {
        // grow with Unknown fill
        let mut v = BelnapVec::all_true(10);
        v.resize(100, Belnap::Unknown);
        assert_eq!(v.width(), 100);
        assert_eq!(v.count_true(), 10);
        assert_eq!(v.count_unknown(), 90);

        // grow with Both fill
        let mut v = BelnapVec::all_true(10);
        v.resize(100, Belnap::Both);
        assert_eq!(v.width(), 100);
        assert_eq!(v.count_true(), 10);
        assert_eq!(v.count_both(), 90);

        // grow with False fill
        let mut v = BelnapVec::all_true(10);
        v.resize(100, Belnap::False);
        assert_eq!(v.width(), 100);
        assert_eq!(v.count_true(), 10);
        assert_eq!(v.count_false(), 90);

        // grow with True fill
        let mut v = BelnapVec::new(10);
        v.resize(100, Belnap::True);
        assert_eq!(v.width(), 100);
        assert_eq!(v.count_unknown(), 10);
        assert_eq!(v.count_true(), 90);

        // grow across word boundary
        let mut v = BelnapVec::all_false(60);
        v.resize(200, Belnap::True);
        assert_eq!(v.width(), 200);
        assert_eq!(v.count_false(), 60);
        assert_eq!(v.count_true(), 140);

        // shrink
        let mut v = BelnapVec::all_true(100);
        v.resize(10, Belnap::False);
        assert_eq!(v.width(), 10);
        assert!(v.is_all_true());

        // grow from empty
        let mut v = BelnapVec::new(0);
        v.resize(64, Belnap::True);
        assert_eq!(v.width(), 64);
        assert!(v.is_all_true());

        let mut v = BelnapVec::new(0);
        v.resize(100, Belnap::False);
        assert_eq!(v.width(), 100);
        assert!(v.is_all_false());
    }

    #[test]
    fn vec_truncate() {
        let mut v = BelnapVec::all_true(100);
        v.truncate(100);
        assert_eq!(v.width(), 100);
        assert!(v.is_all_true());

        let mut v = BelnapVec::all_true(200);
        v.truncate(65);
        assert_eq!(v.width(), 65);
        assert!(v.is_all_true());
        assert_eq!(v.count_true(), 65);
    }

    #[test]
    fn vec_and_different_widths() {
        let mut short = BelnapVec::new(10);
        short.set(0, Belnap::True);
        short.set(1, Belnap::False);
        short.set(2, Belnap::Both);

        let mut long = BelnapVec::new(100);
        long.set(0, Belnap::True);
        long.set(1, Belnap::True);
        long.set(2, Belnap::True);
        long.set(99, Belnap::True);

        let ab = short.and(&long);
        let ba = long.and(&short);
        assert_eq!(ab.width(), 100);
        assert_eq!(ba.width(), 100);
        assert_eq!(ab, ba);

        // True & True = True
        assert_eq!(ab.get(0).unwrap(), Belnap::True);
        // False & True = False
        assert_eq!(ab.get(1).unwrap(), Belnap::False);
        // Both & True = Both
        assert_eq!(ab.get(2).unwrap(), Belnap::Both);
        // Unknown (short) & True (long) = Unknown
        assert_eq!(ab.get(99).unwrap(), Belnap::Unknown);
        // Beyond short: Unknown & Unknown = Unknown
        assert_eq!(ab.get(50).unwrap(), Belnap::Unknown);
    }

    #[test]
    fn vec_or_different_widths() {
        let mut short = BelnapVec::new(10);
        short.set(0, Belnap::True);
        short.set(1, Belnap::False);
        short.set(2, Belnap::Both);

        let mut long = BelnapVec::new(100);
        long.set(0, Belnap::False);
        long.set(1, Belnap::True);
        long.set(2, Belnap::False);
        long.set(99, Belnap::False);

        let ab = short.or(&long);
        let ba = long.or(&short);
        assert_eq!(ab.width(), 100);
        assert_eq!(ba.width(), 100);
        assert_eq!(ab, ba);

        // True | False = True
        assert_eq!(ab.get(0).unwrap(), Belnap::True);
        // False | True = True
        assert_eq!(ab.get(1).unwrap(), Belnap::True);
        // Both | False = Both
        assert_eq!(ab.get(2).unwrap(), Belnap::Both);
        // Unknown (short) | False (long) = Unknown
        assert_eq!(ab.get(99).unwrap(), Belnap::Unknown);
        // Beyond short: Unknown | Unknown = Unknown
        assert_eq!(ab.get(50).unwrap(), Belnap::Unknown);
    }

    #[test]
    fn vec_merge_different_widths() {
        let mut short = BelnapVec::new(10);
        short.set(0, Belnap::True);
        short.set(1, Belnap::False);

        let mut long = BelnapVec::new(100);
        long.set(0, Belnap::False);
        long.set(1, Belnap::True);
        long.set(99, Belnap::True);

        let ab = short.merge(&long);
        let ba = long.merge(&short);
        assert_eq!(ab.width(), 100);
        assert_eq!(ba.width(), 100);
        assert_eq!(ab, ba);

        // True merge False = Both
        assert_eq!(ab.get(0).unwrap(), Belnap::Both);
        // False merge True = Both
        assert_eq!(ab.get(1).unwrap(), Belnap::Both);
        // Unknown (short) merge True (long) = True
        assert_eq!(ab.get(99).unwrap(), Belnap::True);
        // Beyond short: Unknown merge Unknown = Unknown
        assert_eq!(ab.get(50).unwrap(), Belnap::Unknown);
    }

    #[test]
    fn vec_consensus_different_widths() {
        let mut short = BelnapVec::new(10);
        short.set(0, Belnap::Both);
        short.set(1, Belnap::True);

        let mut long = BelnapVec::new(100);
        long.set(0, Belnap::True);
        long.set(1, Belnap::True);
        long.set(99, Belnap::True);

        let ab = short.consensus(&long);
        let ba = long.consensus(&short);
        assert_eq!(ab.width(), 100);
        assert_eq!(ba.width(), 100);
        assert_eq!(ab, ba);

        // Both consensus True = True
        assert_eq!(ab.get(0).unwrap(), Belnap::True);
        // True consensus True = True
        assert_eq!(ab.get(1).unwrap(), Belnap::True);
        // Unknown (short) consensus True (long) = Unknown
        assert_eq!(ab.get(99).unwrap(), Belnap::Unknown);
        // Beyond short: Unknown consensus Unknown = Unknown
        assert_eq!(ab.get(50).unwrap(), Belnap::Unknown);
    }

    #[test]
    fn vec_from_slice_iter_roundtrip() {
        // Empty.
        assert_eq!(BelnapVec::new(0).iter().count(), 0);
        let empty: &[Belnap] = &[];
        assert_eq!(BelnapVec::from(empty).width(), 0);

        // 4 elements covering all variants.
        let xs = [Belnap::Unknown, Belnap::True, Belnap::False, Belnap::Both];
        let collected: Vec<_> = BelnapVec::from(&xs[..]).iter().collect();
        assert_eq!(collected, xs);

        // 64 elements: exactly one full word-pair.
        let xs64 = [Belnap::True; 64];
        let collected: Vec<_> = BelnapVec::all_true(64).iter().collect();
        assert_eq!(collected, xs64);

        // 65 elements: last element straddles into word-pair 1.
        let mut xs65 = [Belnap::True; 65];
        xs65[64] = Belnap::False;
        let collected: Vec<_> = BelnapVec::from(&xs65[..]).iter().collect();
        assert_eq!(collected, xs65);
    }

    #[test]
    fn vec_iter_indexed_and_early_termination() {
        let xs = [Belnap::Unknown, Belnap::True, Belnap::False, Belnap::Both];
        let v = BelnapVec::from(&xs[..]);

        let indexed: Vec<_> = v.iter().enumerate().collect();
        assert_eq!(
            indexed,
            vec![
                (0, Belnap::Unknown),
                (1, Belnap::True),
                (2, Belnap::False),
                (3, Belnap::Both),
            ]
        );

        // Early termination via take.
        let first_two: Vec<_> = v.iter().take(2).collect();
        assert_eq!(first_two, vec![Belnap::Unknown, Belnap::True]);

        // ExactSizeIterator.
        assert_eq!(v.iter().len(), 4);

        // IntoIterator for &BelnapVec.
        let collected: Vec<_> = (&v).into_iter().collect();
        assert_eq!(collected, xs);
    }

    #[test]
    fn vec_find_first() {
        let xs = [Belnap::False, Belnap::False, Belnap::True, Belnap::Both];
        let v = BelnapVec::from(&xs[..]);
        assert_eq!(v.find_first(Belnap::True), Some(2));
        assert_eq!(v.find_first(Belnap::False), Some(0));
        assert_eq!(v.find_first(Belnap::Both), Some(3));
        assert_eq!(v.find_first(Belnap::Unknown), None);

        // Empty vec.
        assert_eq!(BelnapVec::new(0).find_first(Belnap::True), None);

        // Match at word boundary (index 64, word-pair 1).
        let mut xs = [Belnap::False; 65];
        xs[64] = Belnap::True;
        let v = BelnapVec::from(&xs[..]);
        assert_eq!(v.find_first(Belnap::True), Some(64));

        // Tail-mask must not produce a false hit on garbage bits past width.
        assert_eq!(BelnapVec::all_true(63).find_first(Belnap::Unknown), None);
    }

    #[test]
    fn vec_equal() {
        let a = BelnapVec::from(&[Belnap::True, Belnap::False, Belnap::Both][..]);
        let b = BelnapVec::from(&[Belnap::True, Belnap::False, Belnap::Both][..]);
        assert_eq!(a, b);

        let c = BelnapVec::from(&[Belnap::True, Belnap::False, Belnap::Unknown][..]);
        assert_ne!(a, c);

        // Different widths are not equal.
        let d = BelnapVec::from(&[Belnap::True, Belnap::False][..]);
        assert_ne!(a, d);

        assert_eq!(BelnapVec::new(0), BelnapVec::new(0));
    }

    #[test]
    fn vec_newtype_operators_agree() {
        let xs = [Belnap::Unknown, Belnap::True, Belnap::False, Belnap::Both];
        let ys = [Belnap::True, Belnap::Both, Belnap::Unknown, Belnap::False];
        let v = BelnapVec::from(&xs[..]);
        let w = BelnapVec::from(&ys[..]);

        // Borrowed operands.
        assert_eq!((AsTruth(&v) & AsTruth(&w)).0, v.and(&w));
        assert_eq!((AsTruth(&v) | AsTruth(&w)).0, v.or(&w));
        assert_eq!((AsKnowledge(&v) & AsKnowledge(&w)).0, v.consensus(&w));
        assert_eq!((AsKnowledge(&v) | AsKnowledge(&w)).0, v.merge(&w));

        // Owned LHS, borrowed RHS.
        assert_eq!((AsTruth(v.clone()) & AsTruth(&w)).0, v.and(&w));
        // Borrowed LHS, owned RHS.
        assert_eq!((AsTruth(&v) | AsTruth(w.clone())).0, v.or(&w));
        // Owned both.
        assert_eq!(
            (AsKnowledge(v.clone()) & AsKnowledge(w.clone())).0,
            v.consensus(&w)
        );
    }

    #[test]
    fn vec_implies_different_widths() {
        let short = BelnapVec::all_true(10);
        let long = BelnapVec::all_true(100);
        let result = short.implies(&long);
        assert_eq!(result.width(), 100);
        // True -> True = True for first 10
        assert_eq!(result.get(0).unwrap(), Belnap::True);
        // Unknown -> True = True for positions beyond short
        assert_eq!(result.get(50).unwrap(), Belnap::True);
    }

    mod props {
        use proptest::prelude::*;

        use super::*;

        const MAX_N: usize = 200;

        fn arb_belnap() -> impl Strategy<Value = Belnap> {
            prop_oneof![
                Just(Belnap::Unknown),
                Just(Belnap::True),
                Just(Belnap::False),
                Just(Belnap::Both),
            ]
        }

        fn arb_xs() -> impl Strategy<Value = Vec<Belnap>> {
            prop::collection::vec(arb_belnap(), 0..=MAX_N)
        }

        fn arb_xs2() -> impl Strategy<Value = (Vec<Belnap>, Vec<Belnap>)> {
            (0usize..=MAX_N).prop_flat_map(|n| {
                (
                    prop::collection::vec(arb_belnap(), n),
                    prop::collection::vec(arb_belnap(), n),
                )
            })
        }

        fn arb_xs3() -> impl Strategy<Value = (Vec<Belnap>, Vec<Belnap>, Vec<Belnap>)> {
            (0usize..=MAX_N).prop_flat_map(|n| {
                (
                    prop::collection::vec(arb_belnap(), n),
                    prop::collection::vec(arb_belnap(), n),
                    prop::collection::vec(arb_belnap(), n),
                )
            })
        }

        fn arb_get_set() -> impl Strategy<Value = (Vec<Belnap>, usize, Belnap)> {
            (1usize..=MAX_N)
                .prop_flat_map(|n| (prop::collection::vec(arb_belnap(), n), 0..n, arb_belnap()))
        }

        proptest! {
            // -- Lattice laws for | (or) --

            #[test]
            fn or_commutativity((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.or(&b), b.or(&a));
            }

            #[test]
            fn or_associativity((xs, ys, zs) in arb_xs3()) {
                let a = BelnapVec::from(&xs[..]);
                let b = BelnapVec::from(&ys[..]);
                let c = BelnapVec::from(&zs[..]);
                prop_assert_eq!(a.or(&b).or(&c), a.or(&b.or(&c)));
            }

            #[test]
            fn or_idempotency(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.or(&a), a.clone());
            }

            // -- Lattice laws for & (and) --

            #[test]
            fn and_commutativity((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.and(&b), b.and(&a));
            }

            #[test]
            fn and_associativity((xs, ys, zs) in arb_xs3()) {
                let a = BelnapVec::from(&xs[..]);
                let b = BelnapVec::from(&ys[..]);
                let c = BelnapVec::from(&zs[..]);
                prop_assert_eq!(a.and(&b).and(&c), a.and(&b.and(&c)));
            }

            #[test]
            fn and_idempotency(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.and(&a), a.clone());
            }

            // -- Absorption / distributivity --

            #[test]
            fn absorption_or_and((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.or(&a.and(&b)), a.clone());
            }

            #[test]
            fn absorption_and_or((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.and(&a.or(&b)), a.clone());
            }

            #[test]
            fn and_distributes_over_or((xs, ys, zs) in arb_xs3()) {
                let a = BelnapVec::from(&xs[..]);
                let b = BelnapVec::from(&ys[..]);
                let c = BelnapVec::from(&zs[..]);
                prop_assert_eq!(a.and(&b.or(&c)), a.and(&b).or(&a.and(&c)));
            }

            #[test]
            fn or_distributes_over_and((xs, ys, zs) in arb_xs3()) {
                let a = BelnapVec::from(&xs[..]);
                let b = BelnapVec::from(&ys[..]);
                let c = BelnapVec::from(&zs[..]);
                prop_assert_eq!(a.or(&b.and(&c)), a.or(&b).and(&a.or(&c)));
            }

            // -- Identities and annihilators --

            #[test]
            fn or_false_identity(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.or(&BelnapVec::all_false(xs.len())), a.clone());
            }

            #[test]
            fn and_true_identity(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.and(&BelnapVec::all_true(xs.len())), a.clone());
            }

            #[test]
            fn or_true_annihilator(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.or(&BelnapVec::all_true(xs.len())), BelnapVec::all_true(xs.len()));
            }

            #[test]
            fn and_false_annihilator(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.and(&BelnapVec::all_false(xs.len())), BelnapVec::all_false(xs.len()));
            }

            // -- Negation --

            #[test]
            fn implies_definition((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.implies(&b), a.not().or(&b));
            }

            #[test]
            fn not_involutive(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.not().not(), a.clone());
            }

            #[test]
            fn de_morgan_and((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.and(&b).not(), a.not().or(&b.not()));
            }

            #[test]
            fn de_morgan_or((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.or(&b).not(), a.not().and(&b.not()));
            }

            // -- Merge (knowledge join) --

            #[test]
            fn merge_commutativity((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.merge(&b), b.merge(&a));
            }

            #[test]
            fn merge_associativity((xs, ys, zs) in arb_xs3()) {
                let a = BelnapVec::from(&xs[..]);
                let b = BelnapVec::from(&ys[..]);
                let c = BelnapVec::from(&zs[..]);
                prop_assert_eq!(a.merge(&b).merge(&c), a.merge(&b.merge(&c)));
            }

            #[test]
            fn merge_idempotency(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.merge(&a), a.clone());
            }

            #[test]
            fn merge_unknown_identity(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.merge(&BelnapVec::new(xs.len())), a.clone());
            }

            // -- Consensus (knowledge meet) --

            #[test]
            fn consensus_commutativity((xs, ys) in arb_xs2()) {
                let (a, b) = (BelnapVec::from(&xs[..]), BelnapVec::from(&ys[..]));
                prop_assert_eq!(a.consensus(&b), b.consensus(&a));
            }

            #[test]
            fn consensus_associativity((xs, ys, zs) in arb_xs3()) {
                let a = BelnapVec::from(&xs[..]);
                let b = BelnapVec::from(&ys[..]);
                let c = BelnapVec::from(&zs[..]);
                prop_assert_eq!(a.consensus(&b).consensus(&c), a.consensus(&b.consensus(&c)));
            }

            #[test]
            fn consensus_idempotency(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.consensus(&a), a.clone());
            }

            #[test]
            fn consensus_both_identity(xs in arb_xs()) {
                let a = BelnapVec::from(&xs[..]);
                prop_assert_eq!(a.consensus(&BelnapVec::all_both(xs.len())), a.clone());
            }

            // -- Count invariants --

            #[test]
            fn counts_sum_to_width(xs in arb_xs()) {
                let v = BelnapVec::from(&xs[..]);
                prop_assert_eq!(
                    v.count_true() + v.count_false() + v.count_both() + v.count_unknown(),
                    xs.len()
                );
            }

            #[test]
            fn is_consistent_iff_no_both(xs in arb_xs()) {
                let v = BelnapVec::from(&xs[..]);
                prop_assert_eq!(v.is_consistent(), v.count_both() == 0);
            }

            #[test]
            fn is_all_determined_iff(xs in arb_xs()) {
                let v = BelnapVec::from(&xs[..]);
                prop_assert_eq!(
                    v.is_all_determined(),
                    v.count_unknown() == 0 && v.count_both() == 0
                );
            }

            #[test]
            fn is_all_true_iff(xs in arb_xs()) {
                let v = BelnapVec::from(&xs[..]);
                prop_assert_eq!(v.is_all_true(), v.count_true() == xs.len());
            }

            #[test]
            fn is_all_false_iff(xs in arb_xs()) {
                let v = BelnapVec::from(&xs[..]);
                prop_assert_eq!(v.is_all_false(), v.count_false() == xs.len());
            }

            // -- Slice round-trip and iter consistency --

            #[test]
            fn from_slice_iter_roundtrip(xs in arb_xs()) {
                let collected: Vec<_> = BelnapVec::from(&xs[..]).iter().collect();
                prop_assert_eq!(collected, xs);
            }

            #[test]
            fn iter_matches_get(xs in arb_xs()) {
                let v = BelnapVec::from(&xs[..]);
                for (i, val) in v.iter().enumerate() {
                    prop_assert_eq!(val, v.get(i).unwrap());
                }
            }

            // -- find_first --

            #[test]
            fn find_first_returns_match((needle, xs) in (arb_belnap(), arb_xs())) {
                let v = BelnapVec::from(&xs[..]);
                if let Some(i) = v.find_first(needle) {
                    prop_assert_eq!(v.get(i).unwrap(), needle);
                }
            }

            #[test]
            fn find_first_is_leftmost((needle, xs) in (arb_belnap(), arb_xs())) {
                let v = BelnapVec::from(&xs[..]);
                if let Some(i) = v.find_first(needle) {
                    for j in 0..i {
                        prop_assert_ne!(v.get(j).unwrap(), needle);
                    }
                }
            }

            #[test]
            fn find_first_none_iff_count_zero(xs in arb_xs()) {
                let v = BelnapVec::from(&xs[..]);
                for (needle, count) in [
                    (Belnap::True, v.count_true()),
                    (Belnap::False, v.count_false()),
                    (Belnap::Both, v.count_both()),
                    (Belnap::Unknown, v.count_unknown()),
                ] {
                    prop_assert_eq!(v.find_first(needle).is_some(), count > 0);
                }
            }

            // -- get/set --

            #[test]
            fn get_after_set((xs, i, val) in arb_get_set()) {
                let mut v = BelnapVec::from(&xs[..]);
                v.set(i, val);
                prop_assert_eq!(v.get(i).unwrap(), val);
            }
        }
    }
}
