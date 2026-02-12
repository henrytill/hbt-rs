//! Belnap's four-valued logic: scalar type and packed bitvector.

use crate::bitplane::{self, BITS_LOG2, BITS_MASK, pair, tail_mask, words_needed};
use crate::kleene::{Kleene, KleeneVec, OutOfBounds};

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

impl Belnap {
    /// Returns `true` if this value carries any information (not `Unknown`).
    #[inline]
    #[must_use]
    pub const fn has_info(self) -> bool {
        self as u8 != 0
    }

    /// Returns `true` if this value is exactly `True` or `False`.
    #[inline]
    #[must_use]
    pub const fn is_determined(self) -> bool {
        let b = self as u8;
        (b & 1) ^ (b >> 1) != 0
    }

    /// Returns `true` if this value is `Both` (contradicted).
    #[inline]
    #[must_use]
    pub const fn is_contradicted(self) -> bool {
        self as u8 == 0b11
    }

    /// Converts to `bool` if the value is exactly `True` or `False`.
    #[must_use]
    pub const fn to_bool(self) -> Option<bool> {
        match self {
            Belnap::True => Some(true),
            Belnap::False => Some(false),
            _ => None,
        }
    }

    /// Knowledge-ordering join: combine observations from independent sources.
    #[inline]
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        FROM_BITS[(self as u8 | other as u8) as usize]
    }

    #[must_use]
    pub fn implies(self, rhs: Self) -> Self {
        (!self) | rhs
    }
}

impl std::ops::Not for Belnap {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Belnap::True => Belnap::False,
            Belnap::False => Belnap::True,
            Belnap::Unknown => Belnap::Unknown,
            Belnap::Both => Belnap::Both,
        }
    }
}

impl std::ops::BitAnd for Belnap {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        let r_pos = (self as u8 & 1) & (rhs as u8 & 1);
        let r_neg = (self as u8 >> 1) | (rhs as u8 >> 1);
        FROM_BITS[(r_neg << 1 | r_pos) as usize]
    }
}

impl std::ops::BitOr for Belnap {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        let r_pos = (self as u8 & 1) | (rhs as u8 & 1);
        let r_neg = (self as u8 >> 1) & (rhs as u8 >> 1);
        FROM_BITS[(r_neg << 1 | r_pos) as usize]
    }
}

impl From<Kleene> for Belnap {
    fn from(k: Kleene) -> Self {
        FROM_BITS[k as u8 as usize]
    }
}

impl TryFrom<Belnap> for Kleene {
    type Error = ();

    fn try_from(b: Belnap) -> Result<Self, ()> {
        match b {
            Belnap::Unknown => Ok(Kleene::Unknown),
            Belnap::True => Ok(Kleene::True),
            Belnap::False => Ok(Kleene::False),
            Belnap::Both => Err(()),
        }
    }
}

/// Packed Belnap bitvector: two-bitplane representation.
///
/// Encoding per bit position:
/// - pos=0, neg=0 → Unknown
/// - pos=1, neg=0 → True
/// - pos=0, neg=1 → False
/// - pos=1, neg=1 → Both
///
/// Uses an interleaved layout: `[pos_0, neg_0, pos_1, neg_1, ...]`.
/// All four bit patterns are valid (no invariant).
/// Unused high bits in the last word pair are always zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BelnapVec {
    width: usize,
    words: Vec<u64>,
}

impl BelnapVec {
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

    pub fn resize(&mut self, new_width: usize, fill: Belnap) {
        if new_width <= self.width {
            self.truncate(new_width);
            return;
        }
        let old_width = self.width;
        let old_nw = words_needed(old_width);
        let new_nw = words_needed(new_width);
        let (fill_pos, fill_neg): (u64, u64) = match fill {
            Belnap::Unknown => (0, 0),
            Belnap::True => (u64::MAX, 0),
            Belnap::False => (0, u64::MAX),
            Belnap::Both => (u64::MAX, u64::MAX),
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
        if fill.has_info() {
            self.mask_tail();
        }
    }

    // Scalar access

    #[must_use]
    fn get_unchecked(&self, i: usize) -> Belnap {
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
    pub fn get(&self, i: usize) -> Result<Belnap, OutOfBounds> {
        if i >= self.width {
            return Err(OutOfBounds);
        }
        Ok(self.get_unchecked(i))
    }

    fn set_unchecked(&mut self, i: usize, v: Belnap) {
        debug_assert!(i < self.width);
        let w = i >> BITS_LOG2;
        let b = i & BITS_MASK;
        let pn = &mut self.words[pair(w)];
        match v {
            Belnap::True => {
                pn[0] |= 1u64 << b; // set pos
                pn[1] &= !(1u64 << b); // clear neg
            }
            Belnap::False => {
                pn[0] &= !(1u64 << b); // clear pos
                pn[1] |= 1u64 << b; // set neg
            }
            Belnap::Unknown => {
                pn[0] &= !(1u64 << b); // clear pos
                pn[1] &= !(1u64 << b); // clear neg
            }
            Belnap::Both => {
                pn[0] |= 1u64 << b; // set pos
                pn[1] |= 1u64 << b; // set neg
            }
        }
    }

    pub fn set(&mut self, i: usize, v: Belnap) {
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
        let width = self.width.max(other.width);
        let zero = [0u64; 2];
        let words = self
            .words
            .chunks_exact(2)
            .chain(std::iter::repeat(&zero[..]))
            .zip(
                other
                    .words
                    .chunks_exact(2)
                    .chain(std::iter::repeat(&zero[..])),
            )
            .take(words_needed(width))
            .flat_map(|(a, b)| {
                let (p, n) = bitplane::and_word(a[0], a[1], b[0], b[1]);
                [p, n]
            })
            .collect();
        Self { width, words }
    }

    #[must_use]
    pub fn or(&self, other: &Self) -> Self {
        let width = self.width.max(other.width);
        let zero = [0u64; 2];
        let words = self
            .words
            .chunks_exact(2)
            .chain(std::iter::repeat(&zero[..]))
            .zip(
                other
                    .words
                    .chunks_exact(2)
                    .chain(std::iter::repeat(&zero[..])),
            )
            .take(words_needed(width))
            .flat_map(|(a, b)| {
                let (p, n) = bitplane::or_word(a[0], a[1], b[0], b[1]);
                [p, n]
            })
            .collect();
        Self { width, words }
    }

    #[must_use]
    pub fn implies(&self, other: &Self) -> Self {
        self.not().or(other)
    }

    /// Knowledge-ordering join: combine observations from independent sources.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let width = self.width.max(other.width);
        let zero = [0u64; 2];
        let words = self
            .words
            .chunks_exact(2)
            .chain(std::iter::repeat(&zero[..]))
            .zip(
                other
                    .words
                    .chunks_exact(2)
                    .chain(std::iter::repeat(&zero[..])),
            )
            .take(words_needed(width))
            .flat_map(|(a, b)| {
                let (p, n) = bitplane::merge_word(a[0], a[1], b[0], b[1]);
                [p, n]
            })
            .collect();
        Self { width, words }
    }

    // Queries

    /// Returns `true` if no position is `Both` (valid as Kleene).
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        self.words.chunks_exact(2).all(|pn| pn[0] & pn[1] == 0)
    }

    /// Returns `true` if every position is `True` or `False`.
    #[must_use]
    pub fn is_all_determined(&self) -> bool {
        let nw = words_needed(self.width);
        if nw == 0 {
            return true;
        }
        let m = tail_mask(self.width);
        self.words
            .chunks_exact(2)
            .take(nw - 1)
            .all(|pn| pn[0] ^ pn[1] == u64::MAX)
            && {
                let pn = &self.words[pair(nw - 1)];
                pn[0] ^ pn[1] == m
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
        self.width - self.count_true() - self.count_false() - self.count_both()
    }

    /// Convert to `KleeneVec` if no position is `Both`.
    #[must_use]
    pub fn to_kleene(&self) -> Option<KleeneVec> {
        if self.is_consistent() {
            Some(KleeneVec::from_raw_parts(self.width, self.words.clone()))
        } else {
            None
        }
    }
}

impl From<&KleeneVec> for BelnapVec {
    fn from(kv: &KleeneVec) -> Self {
        Self {
            width: kv.width(),
            words: kv.words_raw().to_vec(),
        }
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

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $inherent:ident) => {
        impl std::ops::$trait for &BelnapVec {
            type Output = BelnapVec;

            fn $method(self, rhs: Self) -> BelnapVec {
                self.$inherent(rhs)
            }
        }

        impl std::ops::$trait<&BelnapVec> for BelnapVec {
            type Output = BelnapVec;

            fn $method(self, rhs: &BelnapVec) -> BelnapVec {
                self.$inherent(rhs)
            }
        }

        impl std::ops::$trait<BelnapVec> for &BelnapVec {
            type Output = BelnapVec;

            fn $method(self, rhs: BelnapVec) -> BelnapVec {
                self.$inherent(&rhs)
            }
        }

        impl std::ops::$trait for BelnapVec {
            type Output = BelnapVec;

            fn $method(self, rhs: Self) -> BelnapVec {
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
    fn scalar_not() {
        assert_eq!(!Belnap::True, Belnap::False);
        assert_eq!(!Belnap::False, Belnap::True);
        assert_eq!(!Belnap::Unknown, Belnap::Unknown);
        assert_eq!(!Belnap::Both, Belnap::Both);
    }

    #[test]
    fn scalar_and_truth_table() {
        use Belnap::*;
        // Full 4x4 truth table per Wikipedia B4
        // Rows/columns follow `variants` order: N, T, F, B
        let expected: [[Belnap; 4]; 4] = [
            //         N       T       F      B
            /* N */
            [Unknown, Unknown, False, False],
            /* T */ [Unknown, True, False, Both],
            /* F */ [False, False, False, False],
            /* B */ [False, Both, False, Both],
        ];
        let variants = [Unknown, True, False, Both];
        for (i, &a) in variants.iter().enumerate() {
            for (j, &b) in variants.iter().enumerate() {
                assert_eq!(a & b, expected[i][j], "{a:?} & {b:?}");
            }
        }
    }

    #[test]
    fn scalar_or_truth_table() {
        use Belnap::*;
        // Full 4x4 truth table per Wikipedia B4
        // Rows/columns follow `variants` order: N, T, F, B
        let expected: [[Belnap; 4]; 4] = [
            //         N       T     F       B
            /* N */
            [Unknown, True, Unknown, True],
            /* T */ [True, True, True, True],
            /* F */ [Unknown, True, False, Both],
            /* B */ [True, True, Both, Both],
        ];
        let variants = [Unknown, True, False, Both];
        for (i, &a) in variants.iter().enumerate() {
            for (j, &b) in variants.iter().enumerate() {
                assert_eq!(a | b, expected[i][j], "{a:?} | {b:?}");
            }
        }
    }

    #[test]
    fn scalar_merge() {
        use Belnap::*;
        assert_eq!(Unknown.merge(Unknown), Unknown);
        assert_eq!(Unknown.merge(True), True);
        assert_eq!(Unknown.merge(False), False);
        assert_eq!(True.merge(False), Both);
        assert_eq!(Both.merge(True), Both);
        assert_eq!(Both.merge(False), Both);
        assert_eq!(Both.merge(Unknown), Both);
        assert_eq!(True.merge(True), True);
        assert_eq!(False.merge(False), False);
    }

    #[test]
    fn scalar_queries() {
        use Belnap::*;
        assert!(!Unknown.has_info());
        assert!(True.has_info());
        assert!(False.has_info());
        assert!(Both.has_info());

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
    fn scalar_conversions() {
        // Kleene -> Belnap (infallible)
        assert_eq!(Belnap::from(Kleene::Unknown), Belnap::Unknown);
        assert_eq!(Belnap::from(Kleene::True), Belnap::True);
        assert_eq!(Belnap::from(Kleene::False), Belnap::False);

        // Belnap -> Kleene (fallible)
        assert_eq!(Kleene::try_from(Belnap::Unknown), Ok(Kleene::Unknown));
        assert_eq!(Kleene::try_from(Belnap::True), Ok(Kleene::True));
        assert_eq!(Kleene::try_from(Belnap::False), Ok(Kleene::False));
        assert_eq!(Kleene::try_from(Belnap::Both), Err(()));
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
    fn vec_to_kleene() {
        let mut v = BelnapVec::new(10);
        v.set(0, Belnap::True);
        v.set(1, Belnap::False);
        let k = v.to_kleene();
        assert!(k.is_some());
        let k = k.unwrap();
        assert_eq!(k.get(0), Ok(Kleene::True));
        assert_eq!(k.get(1), Ok(Kleene::False));
        assert_eq!(k.get(2), Ok(Kleene::Unknown));

        // With Both, conversion should fail
        v.set(2, Belnap::Both);
        assert!(v.to_kleene().is_none());
    }

    #[test]
    fn vec_from_kleene() {
        let mut k = KleeneVec::new(10);
        k.set(0, Kleene::True);
        k.set(1, Kleene::False);
        let b = BelnapVec::from(&k);
        assert_eq!(b.get(0).unwrap(), Belnap::True);
        assert_eq!(b.get(1).unwrap(), Belnap::False);
        assert_eq!(b.get(2).unwrap(), Belnap::Unknown);
        assert!(b.is_consistent());
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
}
