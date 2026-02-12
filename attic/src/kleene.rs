//! Kleene's three-valued logic: scalar type and packed bitvector.

use crate::bitplane::{self, BITS_LOG2, BITS_MASK, pair, tail_mask, words_needed};

// We use unchecked casts to convert u64 (and u32) to usize.
const _: () = assert!(std::mem::size_of::<usize>() >= std::mem::size_of::<u64>());

/// A single Kleene truth value.
///
/// Uses `#[repr(u8)]` with discriminants encoding `(neg_bit << 1) | pos_bit`:
///
/// | pos | neg | bits   | variant   |
/// |-----|-----|--------|-----------|
/// | 0   | 0   | `0b00` | `Unknown` |
/// | 1   | 0   | `0b01` | `True`    |
/// | 0   | 1   | `0b10` | `False`   |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kleene {
    Unknown = 0b00, // pos=0, neg=0
    True = 0b01,    // pos=1, neg=0
    False = 0b10,   // pos=0, neg=1
}

const FROM_BITS: [Kleene; 4] = [
    Kleene::Unknown, // 0b00: pos=0, neg=0
    Kleene::True,    // 0b01: pos=1, neg=0
    Kleene::False,   // 0b10: pos=0, neg=1
    Kleene::Unknown, // 0b11: impossible by invariant (pos=1, neg=1)
];

impl Kleene {
    #[inline]
    #[must_use]
    pub const fn is_known(self) -> bool {
        self as u8 != 0
    }

    #[inline]
    #[must_use]
    pub const fn to_bool_unchecked(self) -> bool {
        self as u8 & 1 != 0
    }

    #[must_use]
    pub const fn to_bool(self) -> Option<bool> {
        if self.is_known() {
            Some(self.to_bool_unchecked())
        } else {
            None
        }
    }
}

impl std::ops::Not for Kleene {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Kleene::True => Kleene::False,
            Kleene::False => Kleene::True,
            Kleene::Unknown => Kleene::Unknown,
        }
    }
}

impl std::ops::BitAnd for Kleene {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Kleene::True, b) => b,
            (Kleene::False, _) | (Kleene::Unknown, Kleene::False) => Kleene::False,
            (Kleene::Unknown, _) => Kleene::Unknown,
        }
    }
}

impl std::ops::BitOr for Kleene {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Kleene::False, b) => b,
            (Kleene::True, _) | (Kleene::Unknown, Kleene::True) => Kleene::True,
            (Kleene::Unknown, _) => Kleene::Unknown,
        }
    }
}

impl Kleene {
    #[must_use]
    pub fn implies(self, rhs: Self) -> Self {
        (!self) | rhs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfBounds;

impl std::fmt::Display for OutOfBounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("index out of bounds")
    }
}

impl std::error::Error for OutOfBounds {}

/// Packed Kleene bitvector: two-bitplane representation.
///
/// Encoding per bit position:
/// - pos=0, neg=0 → Unknown
/// - pos=1, neg=0 → True
/// - pos=0, neg=1 → False
///
/// Uses an interleaved layout: `[pos_0, neg_0, pos_1, neg_1, ...]`.
/// Invariant: within every pair, `pos & neg == 0` (no position tells both).
/// Unused high bits in the last word pair are always zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KleeneVec {
    width: usize,
    words: Vec<u64>,
}

impl KleeneVec {
    /// Create a vector of `width` elements, all Unknown.
    #[must_use]
    pub fn new(width: usize) -> Self {
        let nw = words_needed(width);
        Self {
            width,
            words: vec![0; 2 * nw],
        }
    }

    #[must_use]
    pub fn all_true(width: usize) -> Self {
        let nw = words_needed(width);
        let mut words = Vec::with_capacity(2 * nw);
        for _ in 0..nw {
            words.push(u64::MAX); // pos
            words.push(0); // neg
        }
        let mut v = Self { width, words };
        v.mask_tail();
        v
    }

    #[must_use]
    pub fn all_false(width: usize) -> Self {
        let nw = words_needed(width);
        let mut words = Vec::with_capacity(2 * nw);
        for _ in 0..nw {
            words.push(0); // pos
            words.push(u64::MAX); // neg
        }
        let mut v = Self { width, words };
        v.mask_tail();
        v
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn words_raw(&self) -> &[u64] {
        &self.words
    }

    pub(crate) fn from_raw_parts(width: usize, words: Vec<u64>) -> Self {
        let nw = words_needed(width);
        assert_eq!(words.len(), 2 * nw);
        // Verify Kleene invariant: pos & neg == 0 in every pair
        debug_assert!(words.chunks_exact(2).all(|pn| pn[0] & pn[1] == 0));
        Self { width, words }
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

    pub fn resize(&mut self, new_width: usize, fill: Kleene) {
        if new_width <= self.width {
            self.truncate(new_width);
            return;
        }
        let old_width = self.width;
        let old_nw = words_needed(old_width);
        let new_nw = words_needed(new_width);
        let (fill_pos, fill_neg): (u64, u64) = match fill {
            Kleene::Unknown => (0, 0),
            Kleene::True => (u64::MAX, 0),
            Kleene::False => (0, u64::MAX),
        };
        // Fill remaining bits in the current last word pair
        if old_nw > 0 && old_width & BITS_MASK != 0 {
            let high_mask = !tail_mask(old_width);
            let pn = &mut self.words[pair(old_nw - 1)];
            pn[0] |= fill_pos & high_mask;
            pn[1] |= fill_neg & high_mask;
        }
        // Grow by pushing interleaved pairs
        self.words.reserve(2 * (new_nw - old_nw));
        for _ in old_nw..new_nw {
            self.words.push(fill_pos);
            self.words.push(fill_neg);
        }
        self.width = new_width;
        if fill.is_known() {
            self.mask_tail();
        }
    }

    // Scalar access

    #[must_use]
    fn get_unchecked(&self, i: usize) -> Kleene {
        debug_assert!(i < self.width);
        let w = i >> BITS_LOG2;
        let b = i & BITS_MASK;
        let pn = &self.words[pair(w)];
        let pos_bit = ((pn[0] >> b) & 1) as usize;
        let neg_bit = ((pn[1] >> b) & 1) as usize;
        FROM_BITS[neg_bit << 1 | pos_bit]
    }

    /// # Errors
    ///
    /// Returns `OutOfBounds` if `i >= self.width()`.
    pub fn get(&self, i: usize) -> Result<Kleene, OutOfBounds> {
        if i >= self.width {
            return Err(OutOfBounds);
        }
        Ok(self.get_unchecked(i))
    }

    fn set_unchecked(&mut self, i: usize, v: Kleene) {
        debug_assert!(i < self.width);
        let w = i >> BITS_LOG2;
        let b = i & BITS_MASK;
        let pn = &mut self.words[pair(w)];
        match v {
            Kleene::True => {
                pn[0] |= 1u64 << b; // set pos
                pn[1] &= !(1u64 << b); // clear neg
            }
            Kleene::False => {
                pn[0] &= !(1u64 << b); // clear pos
                pn[1] |= 1u64 << b; // set neg
            }
            Kleene::Unknown => {
                pn[0] &= !(1u64 << b); // clear pos
                pn[1] &= !(1u64 << b); // clear neg
            }
        }
        debug_assert!(pn[0] & pn[1] == 0);
    }

    pub fn set(&mut self, i: usize, v: Kleene) {
        if i >= self.width {
            let new_width = i + 1;
            let new_nw = words_needed(new_width);
            let old_nw = words_needed(self.width);
            self.words.reserve(2 * (new_nw - old_nw));
            for _ in old_nw..new_nw {
                self.words.push(0);
                self.words.push(0);
            }
            self.width = new_width;
        }
        self.set_unchecked(i, v);
    }

    // Bulk operations

    fn check_width(&self, other: &Self) {
        assert_eq!(self.width, other.width, "KleeneVec: width mismatch");
    }

    #[must_use]
    pub fn not(&self) -> Self {
        let words = self
            .words
            .chunks_exact(2)
            .flat_map(|pn| {
                let (p, n) = bitplane::not_word(pn[0], pn[1]);
                [p, n]
            })
            .collect();
        Self {
            width: self.width,
            words,
        }
    }

    #[must_use]
    pub fn and(&self, other: &Self) -> Self {
        self.check_width(other);
        let words = self
            .words
            .chunks_exact(2)
            .zip(other.words.chunks_exact(2))
            .flat_map(|(a, b)| {
                let (p, n) = bitplane::and_word(a[0], a[1], b[0], b[1]);
                [p, n]
            })
            .collect();
        Self {
            width: self.width,
            words,
        }
    }

    #[must_use]
    pub fn or(&self, other: &Self) -> Self {
        self.check_width(other);
        let words = self
            .words
            .chunks_exact(2)
            .zip(other.words.chunks_exact(2))
            .flat_map(|(a, b)| {
                let (p, n) = bitplane::or_word(a[0], a[1], b[0], b[1]);
                [p, n]
            })
            .collect();
        Self {
            width: self.width,
            words,
        }
    }

    #[must_use]
    pub fn implies(&self, other: &Self) -> Self {
        self.not().or(other)
    }

    // Queries

    #[must_use]
    pub fn is_all_known(&self) -> bool {
        let nw = words_needed(self.width);
        if nw == 0 {
            return true;
        }
        let m = tail_mask(self.width);
        self.words
            .chunks_exact(2)
            .take(nw - 1)
            .all(|pn| pn[0] | pn[1] == u64::MAX)
            && {
                let pn = &self.words[pair(nw - 1)];
                pn[0] | pn[1] == m
            }
    }

    #[must_use]
    pub fn is_all_true(&self) -> bool {
        let nw = words_needed(self.width);
        if nw == 0 {
            return true;
        }
        let m = tail_mask(self.width);
        self.words
            .chunks_exact(2)
            .take(nw - 1)
            .all(|pn| pn[0] == u64::MAX && pn[1] == 0)
            && {
                let pn = &self.words[pair(nw - 1)];
                pn[0] == m && pn[1] == 0
            }
    }

    #[must_use]
    pub fn is_all_false(&self) -> bool {
        let nw = words_needed(self.width);
        if nw == 0 {
            return true;
        }
        let m = tail_mask(self.width);
        self.words
            .chunks_exact(2)
            .take(nw - 1)
            .all(|pn| pn[0] == 0 && pn[1] == u64::MAX)
            && {
                let pn = &self.words[pair(nw - 1)];
                pn[0] == 0 && pn[1] == m
            }
    }

    #[must_use]
    pub fn count_true(&self) -> usize {
        self.words
            .chunks_exact(2)
            .map(|pn| pn[0].count_ones() as usize)
            .sum()
    }

    #[must_use]
    pub fn count_false(&self) -> usize {
        self.words
            .chunks_exact(2)
            .map(|pn| pn[1].count_ones() as usize)
            .sum()
    }

    #[must_use]
    pub fn count_unknown(&self) -> usize {
        self.width - self.count_true() - self.count_false()
    }
}

impl std::ops::Not for &KleeneVec {
    type Output = KleeneVec;

    fn not(self) -> KleeneVec {
        KleeneVec::not(self)
    }
}

impl std::ops::Not for KleeneVec {
    type Output = KleeneVec;

    fn not(self) -> KleeneVec {
        KleeneVec::not(&self)
    }
}

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $inherent:ident) => {
        impl std::ops::$trait for &KleeneVec {
            type Output = KleeneVec;

            fn $method(self, rhs: Self) -> KleeneVec {
                self.$inherent(rhs)
            }
        }

        impl std::ops::$trait<&KleeneVec> for KleeneVec {
            type Output = KleeneVec;

            fn $method(self, rhs: &KleeneVec) -> KleeneVec {
                self.$inherent(rhs)
            }
        }

        impl std::ops::$trait<KleeneVec> for &KleeneVec {
            type Output = KleeneVec;

            fn $method(self, rhs: KleeneVec) -> KleeneVec {
                self.$inherent(&rhs)
            }
        }

        impl std::ops::$trait for KleeneVec {
            type Output = KleeneVec;

            fn $method(self, rhs: Self) -> KleeneVec {
                self.$inherent(&rhs)
            }
        }
    };
}

impl_binop!(BitAnd, bitand, and);
impl_binop!(BitOr, bitor, or);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_ops() {
        use Kleene::*;
        assert_eq!(!True, False);
        assert_eq!(!False, True);
        assert_eq!(!Unknown, Unknown);

        assert_eq!(True & True, True);
        assert_eq!(True & False, False);
        assert_eq!(True & Unknown, Unknown);
        assert_eq!(False & Unknown, False);
        assert_eq!(Unknown & Unknown, Unknown);

        assert_eq!(False | False, False);
        assert_eq!(False | True, True);
        assert_eq!(False | Unknown, Unknown);
        assert_eq!(True | Unknown, True);
        assert_eq!(Unknown | Unknown, Unknown);
    }

    #[test]
    fn vec_get_set() {
        let mut v = KleeneVec::new(100);
        assert_eq!(v.get(0).unwrap(), Kleene::Unknown);
        v.set(0, Kleene::True);
        v.set(1, Kleene::False);
        v.set(99, Kleene::True);
        assert_eq!(v.get(0).unwrap(), Kleene::True);
        assert_eq!(v.get(1).unwrap(), Kleene::False);
        assert_eq!(v.get(2).unwrap(), Kleene::Unknown);
        assert_eq!(v.get(99).unwrap(), Kleene::True);
    }

    #[test]
    fn vec_and() {
        let a = KleeneVec::all_true(64);
        let b = KleeneVec::all_false(64);
        let c = a.and(&b);
        assert!(c.is_all_false());
    }

    #[test]
    fn vec_or() {
        let a = KleeneVec::all_false(64);
        let b = KleeneVec::all_true(64);
        let c = a.or(&b);
        assert!(c.is_all_true());
    }

    #[test]
    fn vec_not() {
        let a = KleeneVec::all_true(100);
        let b = a.not();
        assert!(b.is_all_false());
        let c = b.not();
        assert!(c.is_all_true());
    }

    #[test]
    fn vec_unknown_and() {
        let a = KleeneVec::new(64); // all unknown
        let b = KleeneVec::all_true(64);
        let c = a.and(&b);
        assert_eq!(c.count_unknown(), 64);

        let d = KleeneVec::all_false(64);
        let e = a.and(&d);
        assert!(e.is_all_false());
    }

    #[test]
    fn counts() {
        let mut v = KleeneVec::new(10);
        v.set(0, Kleene::True);
        v.set(1, Kleene::True);
        v.set(2, Kleene::False);
        assert_eq!(v.count_true(), 2);
        assert_eq!(v.count_false(), 1);
        assert_eq!(v.count_unknown(), 7);
    }

    #[test]
    fn get_out_of_bounds() {
        let v = KleeneVec::new(10);
        assert_eq!(v.get(10), Err(OutOfBounds));
        assert_eq!(v.get(100), Err(OutOfBounds));
    }

    #[test]
    fn set_auto_grows() {
        let mut v = KleeneVec::new(10);
        v.set(100, Kleene::True);
        assert_eq!(v.width(), 101);
        assert_eq!(v.get(100), Ok(Kleene::True));
        assert_eq!(v.get(50), Ok(Kleene::Unknown));
        assert_eq!(v.get(200), Err(OutOfBounds));
    }

    #[test]
    fn truncate() {
        // no-op when new_width >= width
        let mut v = KleeneVec::all_true(100);
        v.truncate(100);
        assert_eq!(v.width(), 100);
        assert!(v.is_all_true());
        v.truncate(200);
        assert_eq!(v.width(), 100);
        assert!(v.is_all_true());

        // truncate to zero
        let mut v = KleeneVec::all_true(100);
        v.truncate(0);
        assert_eq!(v.width(), 0);
        assert_eq!(v.count_true(), 0);

        // partial word (within a single word)
        let mut v = KleeneVec::all_true(64);
        v.truncate(30);
        assert_eq!(v.width(), 30);
        assert!(v.is_all_true());
        assert_eq!(v.count_true(), 30);

        // across word boundary
        let mut v = KleeneVec::all_true(200);
        v.truncate(65);
        assert_eq!(v.width(), 65);
        assert!(v.is_all_true());
        assert_eq!(v.count_true(), 65);
    }

    #[test]
    fn resize() {
        // grow with Unknown fill
        let mut v = KleeneVec::all_true(10);
        v.resize(100, Kleene::Unknown);
        assert_eq!(v.width(), 100);
        assert_eq!(v.count_true(), 10);
        assert_eq!(v.count_unknown(), 90);

        // grow with False fill
        let mut v = KleeneVec::all_true(10);
        v.resize(100, Kleene::False);
        assert_eq!(v.width(), 100);
        assert_eq!(v.count_true(), 10);
        assert_eq!(v.count_false(), 90);
        assert_eq!(v.count_unknown(), 0);

        // grow with True fill
        let mut v = KleeneVec::new(10);
        v.resize(100, Kleene::True);
        assert_eq!(v.width(), 100);
        assert_eq!(v.count_unknown(), 10);
        assert_eq!(v.count_true(), 90);

        // grow across word boundary
        let mut v = KleeneVec::all_false(60);
        v.resize(200, Kleene::True);
        assert_eq!(v.width(), 200);
        assert_eq!(v.count_false(), 60);
        assert_eq!(v.count_true(), 140);

        // shrink
        let mut v = KleeneVec::all_true(100);
        v.resize(10, Kleene::False);
        assert_eq!(v.width(), 10);
        assert!(v.is_all_true());

        // grow from empty
        let mut v = KleeneVec::new(0);
        v.resize(64, Kleene::True);
        assert_eq!(v.width(), 64);
        assert!(v.is_all_true());

        let mut v = KleeneVec::new(0);
        v.resize(100, Kleene::False);
        assert_eq!(v.width(), 100);
        assert!(v.is_all_false());
    }
}
