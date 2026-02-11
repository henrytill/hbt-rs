//! Kleene's three-valued logic: scalar type and packed bitvector.

// We use unchecked casts to convert u64 (and u32) to usize.
const _: () = assert!(std::mem::size_of::<usize>() >= std::mem::size_of::<u64>());

/// A single Kleene truth value.
///
/// Uses `#[repr(u8)]` with discriminants encoding `(known_bit << 1) | value_bit`:
///
/// | known | value | bits   | variant   |
/// |-------|-------|--------|-----------|
/// | 0     | 0     | `0b00` | `Unknown` |
/// | 1     | 0     | `0b10` | `False`   |
/// | 1     | 1     | `0b11` | `True`    |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kleene {
    Unknown = 0b00, // known=0, value=0
    False = 0b10,   // known=1, value=0
    True = 0b11,    // known=1, value=1
}

const FROM_BITS: [Kleene; 4] = [
    Kleene::Unknown, // 0b00: known=0, value=0
    Kleene::Unknown, // 0b01: impossible by invariant (value=1, known=0)
    Kleene::False,   // 0b10: known=1, value=0
    Kleene::True,    // 0b11: known=1, value=1
];

impl Kleene {
    #[inline]
    #[must_use]
    pub const fn is_known(self) -> bool {
        self as u8 & 0b10 != 0
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
/// - known=0, value=0 → Unknown
/// - known=1, value=0 → False
/// - known=1, value=1 → True
///
/// Invariant: `value[i] & !known[i] == 0` for all `i`.
/// Unused high bits in the last word are always zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KleeneVec {
    width: usize,
    known: Vec<u64>,
    value: Vec<u64>,
}

const BITS_LOG2: u32 = 6;
const BITS_MASK: usize = 2_usize.pow(BITS_LOG2) - 1;

#[inline]
const fn words_needed(n: usize) -> usize {
    (n + BITS_MASK) >> BITS_LOG2
}

#[inline]
const fn tail_mask(n: usize) -> u64 {
    let r = n & BITS_MASK;
    if r == 0 { u64::MAX } else { (1u64 << r) - 1 }
}

impl KleeneVec {
    /// Create a vector of `width` elements, all Unknown.
    #[must_use]
    pub fn new(width: usize) -> Self {
        let nw = words_needed(width);
        Self {
            width,
            known: vec![0; nw],
            value: vec![0; nw],
        }
    }

    #[must_use]
    pub fn all_true(width: usize) -> Self {
        let nw = words_needed(width);
        let mut v = Self {
            width,
            known: vec![u64::MAX; nw],
            value: vec![u64::MAX; nw],
        };
        v.mask_tail();
        v
    }

    #[must_use]
    pub fn all_false(width: usize) -> Self {
        let nw = words_needed(width);
        let mut v = Self {
            width,
            known: vec![u64::MAX; nw],
            value: vec![0; nw],
        };
        v.mask_tail();
        v
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    fn mask_tail(&mut self) {
        if let Some(last_k) = self.known.last_mut() {
            let m = tail_mask(self.width);
            *last_k &= m;
        }
        if let Some(last_v) = self.value.last_mut() {
            let m = tail_mask(self.width);
            *last_v &= m;
        }
    }

    // Bulk resize

    pub fn truncate(&mut self, new_width: usize) {
        if new_width >= self.width {
            return;
        }
        self.width = new_width;
        let nw = words_needed(new_width);
        self.known.truncate(nw);
        self.value.truncate(nw);
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
        let (fill_known, fill_value): (u64, u64) = match fill {
            Kleene::Unknown => (0, 0),
            Kleene::False => (u64::MAX, 0),
            Kleene::True => (u64::MAX, u64::MAX),
        };
        // Fill remaining bits in the current last word
        if old_nw > 0 && old_width & BITS_MASK != 0 {
            let high_mask = !tail_mask(old_width);
            self.known[old_nw - 1] |= fill_known & high_mask;
            self.value[old_nw - 1] |= fill_value & high_mask;
        }
        self.known.resize(new_nw, fill_known);
        self.value.resize(new_nw, fill_value);
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
        let known_bit = ((self.known[w] >> b) & 1) as usize;
        let value_bit = ((self.value[w] >> b) & 1) as usize;
        FROM_BITS[known_bit << 1 | value_bit]
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
        match v {
            Kleene::True => {
                self.known[w] |= 1u64 << b;
                self.value[w] |= 1u64 << b;
            }
            Kleene::False => {
                self.known[w] |= 1u64 << b;
                self.value[w] &= !(1u64 << b);
            }
            Kleene::Unknown => {
                self.known[w] &= !(1u64 << b);
                self.value[w] &= !(1u64 << b);
            }
        }
    }

    pub fn set(&mut self, i: usize, v: Kleene) {
        if i >= self.width {
            let new_width = i + 1;
            self.known.resize(words_needed(new_width), 0u64);
            self.value.resize(words_needed(new_width), 0u64);
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
        let known = self.known.clone();
        let value = self
            .known
            .iter()
            .zip(&self.value)
            .map(|(&k, &v)| k & !v)
            .collect();
        Self {
            width: self.width,
            known,
            value,
        }
    }

    #[must_use]
    pub fn and(&self, other: &Self) -> Self {
        self.check_width(other);
        let mut known = Vec::with_capacity(self.known.len());
        let mut value = Vec::with_capacity(self.known.len());
        for i in 0..self.known.len() {
            let (ak, av) = (self.known[i], self.value[i]);
            let (bk, bv) = (other.known[i], other.value[i]);
            let false_a = ak & !av;
            let false_b = bk & !bv;
            let true_a = ak & av;
            let true_b = bk & bv;
            let result_true = true_a & true_b;
            let result_false = false_a | false_b;
            known.push(result_true | result_false);
            value.push(result_true);
        }
        Self {
            width: self.width,
            known,
            value,
        }
    }

    #[must_use]
    pub fn or(&self, other: &Self) -> Self {
        self.check_width(other);
        let mut known = Vec::with_capacity(self.known.len());
        let mut value = Vec::with_capacity(self.known.len());
        for i in 0..self.known.len() {
            let (ak, av) = (self.known[i], self.value[i]);
            let (bk, bv) = (other.known[i], other.value[i]);
            let true_a = ak & av;
            let true_b = bk & bv;
            let false_a = ak & !av;
            let false_b = bk & !bv;
            let result_true = true_a | true_b;
            let result_false = false_a & false_b;
            known.push(result_true | result_false);
            value.push(result_true);
        }
        Self {
            width: self.width,
            known,
            value,
        }
    }

    #[must_use]
    pub fn implies(&self, other: &Self) -> Self {
        self.not().or(other)
    }

    // Queries

    #[must_use]
    pub fn is_all_known(&self) -> bool {
        let nw = self.known.len();
        if nw == 0 {
            return true;
        }
        let m = tail_mask(self.width);
        self.known[..nw - 1].iter().all(|&w| w == u64::MAX) && self.known[nw - 1] == m
    }

    #[must_use]
    pub fn is_all_true(&self) -> bool {
        let nw = self.known.len();
        if nw == 0 {
            return true;
        }
        let m = tail_mask(self.width);
        self.known[..nw - 1].iter().all(|&w| w == u64::MAX)
            && self.known[nw - 1] == m
            && self.value[..nw - 1].iter().all(|&w| w == u64::MAX)
            && self.value[nw - 1] == m
    }

    #[must_use]
    pub fn is_all_false(&self) -> bool {
        self.is_all_known() && self.value.iter().all(|&w| w == 0)
    }

    #[must_use]
    pub fn count_true(&self) -> usize {
        self.value.iter().map(|w| w.count_ones() as usize).sum()
    }

    #[must_use]
    pub fn count_false(&self) -> usize {
        self.known
            .iter()
            .zip(&self.value)
            .map(|(&k, &v)| (k & !v).count_ones() as usize)
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
