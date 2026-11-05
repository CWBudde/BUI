//! Number formatting into a stack buffer: no heap allocation, no
//! `core::fmt` machinery on the hot path (Blender formats spreadsheet cells
//! into a small fixed buffer the same way, `spreadsheet_layout.cc`).

use std::fmt;

/// A fixed buffer a cell's text is formatted into. Large enough for any
/// integer and for floats up to 15 integer digits and 9 decimals.
#[derive(Clone, Debug)]
pub struct CellBuf {
    bytes: [u8; Self::CAP],
    len: usize,
}

impl Default for CellBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl CellBuf {
    pub const CAP: usize = 48;

    #[must_use]
    pub const fn new() -> Self {
        Self { bytes: [0; Self::CAP], len: 0 }
    }

    pub const fn clear(&mut self) {
        self.len = 0;
    }

    /// The formatted text. Only whole UTF-8 sequences are ever written, so
    /// the check cannot fail; the empty string is the safe fallback.
    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    fn push_bytes(&mut self, s: &[u8]) {
        let n = s.len().min(Self::CAP - self.len);
        self.bytes[self.len..self.len + n].copy_from_slice(&s[..n]);
        self.len += n;
    }

    /// Write the decimal digits of `v`.
    pub fn push_u64(&mut self, mut v: u64) {
        let mut tmp = [0u8; 20];
        let mut i = tmp.len();
        loop {
            i -= 1;
            tmp[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        self.push_bytes(&tmp[i..]);
    }

    pub fn push_i64(&mut self, v: i64) {
        if v < 0 {
            self.push_bytes(b"-");
        }
        self.push_u64(v.unsigned_abs());
    }

    /// `v` with exactly `decimals` digits after the point (at most 9),
    /// rounded half to even on the scaled value. `NaN`, `inf` and values
    /// beyond 15 integer digits use the standard formatter (the exponent
    /// form for huge values), still without heap allocation.
    pub fn push_f64(&mut self, v: f64, decimals: u8) {
        const POW10: [u64; 10] = [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000];
        let d = usize::from(decimals.min(9));
        let scaled = v.abs() * POW10[d] as f64;
        if v.is_finite() && scaled < 9.0e18 {
            let s = scaled.round_ties_even() as u64;
            // a negative value that rounds to zero prints without a sign
            if v.is_sign_negative() && s != 0 {
                self.push_bytes(b"-");
            }
            self.push_u64(s / POW10[d]);
            if d > 0 {
                self.push_bytes(b".");
                let frac = s % POW10[d];
                // zero-pad to `d` digits
                let digits = if frac == 0 { 1 } else { frac.ilog10() as usize + 1 };
                for _ in digits..d {
                    self.push_bytes(b"0");
                }
                self.push_u64(frac);
            }
        } else {
            let _ = fmt::Write::write_fmt(self, format_args!("{v:e}"));
        }
    }
}

impl fmt::Write for CellBuf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // never split a UTF-8 sequence when the buffer is full
        let mut n = s.len().min(Self::CAP - self.len);
        while !s.is_char_boundary(n) {
            n -= 1;
        }
        self.push_bytes(&s.as_bytes()[..n]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f64, d: u8) -> String {
        let mut b = CellBuf::new();
        b.push_f64(v, d);
        b.as_str().to_owned()
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    #[test]
    fn integers_match_std() {
        let mut rng = Rng(7);
        let mut cases = vec![0i64, 1, -1, 9, 10, 99, 100, i64::MAX, i64::MIN, 1_000_000, -1_000_000];
        cases.extend((0..5000).map(|_| (rng.next() >> (rng.next() % 64)) as i64));
        for v in cases {
            let mut b = CellBuf::new();
            b.push_i64(v);
            assert_eq!(b.as_str(), v.to_string());
        }
        for v in [0u64, u64::MAX, 12345] {
            let mut b = CellBuf::new();
            b.push_u64(v);
            assert_eq!(b.as_str(), v.to_string());
        }
    }

    #[test]
    fn decimal_data_matches_std() {
        // k-digit decimals printed with k decimals must equal `format!`
        let mut rng = Rng(99);
        for _ in 0..20_000 {
            let d = (rng.next() % 7) as u8;
            let scale = 10f64.powi(i32::from(d));
            let int = (rng.next() % 2_000_000_000) as i64 - 1_000_000_000;
            let v = int as f64 / scale;
            assert_eq!(f(v, d), format!("{v:.prec$}", prec = usize::from(d)), "v {v} d {d}");
        }
    }

    #[test]
    fn arbitrary_floats_are_within_one_unit_of_std() {
        let mut rng = Rng(5);
        for _ in 0..20_000 {
            let v = (rng.next() as f64 / u64::MAX as f64 - 0.5) * 1e6;
            for d in [0u8, 1, 2, 3, 6] {
                let (a, b) = (f(v, d), format!("{v:.prec$}", prec = usize::from(d)));
                let (pa, pb): (f64, f64) = (a.parse().unwrap(), b.parse().unwrap());
                assert!((pa - pb).abs() <= 1.0001 / 10f64.powi(i32::from(d)), "{a} vs {b}");
            }
        }
    }

    #[test]
    fn specials_and_huge() {
        assert_eq!(f(0.0, 2), "0.00");
        assert_eq!(f(-0.0, 2), "0.00");
        assert_eq!(f(-0.001, 2), "0.00");
        assert_eq!(f(-1.5, 0), "-2");
        assert_eq!(f(2.5, 0), "2");
        assert_eq!(f(f64::NAN, 2), "NaN");
        assert_eq!(f(f64::INFINITY, 2), "inf");
        assert_eq!(f(f64::NEG_INFINITY, 2), "-inf");
        assert_eq!(f(1e300, 2), "1e300");
        assert_eq!(f(0.5, 200), f(0.5, 9));
        assert!(f(1e18, 3).len() < CellBuf::CAP);
    }

    #[test]
    fn full_buffer_truncates_on_a_char_boundary() {
        use std::fmt::Write as _;
        let mut b = CellBuf::new();
        for _ in 0..40 {
            b.write_str("\u{e9}").unwrap();
        }
        assert_eq!(b.as_str().len(), 48);
        assert!(b.as_str().chars().all(|c| c == '\u{e9}'));
        b.clear();
        assert_eq!(b.as_str(), "");
    }
}
