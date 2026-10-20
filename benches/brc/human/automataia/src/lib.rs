//! Core types and helpers shared by all 1BRC variants.

pub mod hash;

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub min: i32,
    pub max: i32,
    pub sum: i64,
    pub count: u32,
}

impl Stats {
    #[inline]
    pub fn new(t: i32) -> Self {
        Self {
            min: t,
            max: t,
            sum: t as i64,
            count: 1,
        }
    }

    #[inline]
    pub fn update(&mut self, t: i32) {
        if t < self.min {
            self.min = t;
        }
        if t > self.max {
            self.max = t;
        }
        self.sum += t as i64;
        self.count += 1;
    }

    #[inline]
    pub fn merge(&mut self, other: &Stats) {
        if other.min < self.min {
            self.min = other.min;
        }
        if other.max > self.max {
            self.max = other.max;
        }
        self.sum += other.sum;
        self.count += other.count;
    }
}

/// Split `data` into roughly `n` chunks aligned on `\n` boundaries.
/// Each chunk is a `[start, end)` byte range; chunks cover the data up
/// to the last `\n` (any trailing bytes after the final newline — e.g.
/// the 8-byte SWAR padding written by `gen` — are ignored).
pub fn chunk_bounds(data: &[u8], n: usize) -> Vec<(usize, usize)> {
    let n = n.max(1);
    let raw = data.len();
    if raw == 0 {
        return vec![(0, 0)];
    }
    // Trim trailing bytes past the last `\n`. Padding from `gen` is 8
    // bytes; we search the last 16 to be safe but bounded.
    let tail_lo = raw.saturating_sub(16);
    let len = match memchr::memrchr(b'\n', &data[tail_lo..]) {
        Some(p) => tail_lo + p + 1,
        None => raw,
    };
    if len == 0 {
        return vec![(0, 0)];
    }
    let approx = len / n;
    let mut bounds = Vec::with_capacity(n);
    let mut start = 0usize;
    for i in 0..n {
        let mut end = if i + 1 == n { len } else { start + approx };
        if end >= len {
            end = len;
        } else {
            while end < len && data[end] != b'\n' {
                end += 1;
            }
            if end < len {
                end += 1;
            } // include the newline
        }
        bounds.push((start, end));
        start = end;
        if start >= len {
            break;
        }
    }
    bounds
}

/// Parse a temperature byte slice (no trailing newline) like "12.3", "-7.4",
/// "0.5", "-99.9" into a fixed-point integer (value * 10).
#[inline]
pub fn parse_temp(s: &[u8]) -> i32 {
    let (sign, s) = if s[0] == b'-' {
        (-1i32, &s[1..])
    } else {
        (1, s)
    };
    let v = match s {
        [a, b, b'.', c] => {
            (*a as i32 - b'0' as i32) * 100
                + (*b as i32 - b'0' as i32) * 10
                + (*c as i32 - b'0' as i32)
        }
        [a, b'.', c] => (*a as i32 - b'0' as i32) * 10 + (*c as i32 - b'0' as i32),
        _ => 0,
    };
    sign * v
}

/// True merykitty-style branchless SWAR parser. Reads 8 bytes as a `u64`,
/// finds the decimal point via `(!w & 0x10101010).trailing_zeros()`, then
/// selects between the X.Y and XY.Z layouts without branches in the hot
/// computation. Returns `(temp_x10, bytes_consumed_including_newline)`.
///
/// Safety: caller must guarantee `bytes` has at least 8 readable bytes
/// (mmap of file rounds up to page boundary, so a few extra bytes after
/// the last newline are safe to read).
#[inline(always)]
pub fn parse_temp_merykitty(bytes: &[u8]) -> (i32, usize) {
    let word = u64::from_le_bytes(bytes[..8].try_into().unwrap());

    // Sign detection: first byte == '-'?
    let neg = (word & 0xFF) == b'-' as u64;
    let off = neg as u64;
    let w = word >> (off << 3);

    // Decimal point position (in bits): 12 for X.Y\n, 20 for XY.Z\n.
    let dec_pos_bits = (!w & 0x1010_1010_u64).trailing_zeros();

    // Digit byte values (low nibble of ASCII '0'..'9' is the digit value).
    let b0 = (w & 0x0F) as i32;
    let b1 = ((w >> 8) & 0x0F) as i32;
    let b2 = ((w >> 16) & 0x0F) as i32;
    let b3 = ((w >> 24) & 0x0F) as i32;

    let val_xy = b0 * 10 + b2; // X.Y
    let val_xyz = b0 * 100 + b1 * 10 + b3; // XY.Z

    // Select: dec_pos_bits == 12 → X.Y, dec_pos_bits == 20 → XY.Z.
    // (dec_pos_bits >= 20) ? val_xyz : val_xy — compiles to cmov.
    let is_xyz = (dec_pos_bits >= 20) as i32;
    let val = val_xy * (1 - is_xyz) + val_xyz * is_xyz;

    let signed = if neg { -val } else { val };
    // dec_pos_bits >> 3 = byte index of '.' (relative to start of value after sign).
    // Total: sign byte + (dot_index + 1 byte of '.' + 1 digit + '\n') = off + dot_index + 3.
    let consumed = (off as usize) + ((dec_pos_bits >> 3) as usize) + 3;
    (signed, consumed)
}

/// Branchless SWAR-style parser. Assumes the temperature ends with `\n`
/// and that at least 8 bytes from `ptr` are readable (always true inside
/// a chunk that ends on a newline; the caller must ensure the slack).
///
/// Returns `(temp_x10, bytes_consumed_including_newline)`.
///
/// Layout cases (positions of '.' and '\n' from `start`):
///   X.Y\n    (4): val = X*10 + Y, sign=+
///   XY.Z\n   (5): val = X*100 + Y*10 + Z, sign=+
///   -X.Y\n   (5): val = X*10 + Y, sign=-
///   -XY.Z\n  (6): val = X*100 + Y*10 + Z, sign=-
#[inline]
pub fn parse_temp_swar(bytes: &[u8]) -> (i32, usize) {
    // Detect sign.
    let neg = bytes[0] == b'-';
    let off = neg as usize;
    let s = &bytes[off..];

    // Find '.' — it can be at index 1 or 2 (after sign offset removed).
    let dot1 = s[1] == b'.';
    let (val, len) = if dot1 {
        // X.Y\n
        let v = (s[0] - b'0') as i32 * 10 + (s[2] - b'0') as i32;
        (v, 4)
    } else {
        // XY.Z\n
        let v = (s[0] - b'0') as i32 * 100 + (s[1] - b'0') as i32 * 10 + (s[3] - b'0') as i32;
        (v, 5)
    };
    let total = off + len; // includes '\n'
    let signed = if neg { -val } else { val };
    (signed, total)
}

/// Format the final aggregate map in the canonical 1BRC output format:
/// `{Station=min/mean/max, Station=min/mean/max, ...}`
///
/// `map` must be alphabetically ordered (BTreeMap).
pub fn format_output<K: AsRef<[u8]>>(map: &BTreeMap<K, Stats>) -> String {
    let mut out = String::with_capacity(map.len() * 40);
    out.push('{');
    let mut first = true;
    for (name, s) in map {
        if !first {
            out.push_str(", ");
        }
        first = false;
        out.push_str(std::str::from_utf8(name.as_ref()).unwrap_or("?"));
        let mean_tenths = round_ties_toward_positive(s.sum, s.count);
        let _ = write!(
            out,
            "={}/{}/{}",
            fmt_tenths(s.min),
            fmt_tenths(mean_tenths),
            fmt_tenths(s.max),
        );
    }
    out.push('}');
    out
}

#[inline]
fn round_ties_toward_positive(sum: i64, count: u32) -> i32 {
    if count == 0 {
        return 0;
    }
    let divisor = i64::from(count);
    let quotient = sum.div_euclid(divisor);
    let remainder = sum.rem_euclid(divisor);
    let rounded = if remainder * 2 >= divisor {
        quotient + 1
    } else {
        quotient
    };
    rounded.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[inline]
fn fmt_tenths(v: i32) -> String {
    let abs = v.unsigned_abs();
    let whole = abs / 10;
    let frac = abs % 10;
    if v < 0 {
        format!("-{whole}.{frac}")
    } else {
        format!("{whole}.{frac}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_temp_basics() {
        assert_eq!(parse_temp(b"12.3"), 123);
        assert_eq!(parse_temp(b"-12.3"), -123);
        assert_eq!(parse_temp(b"0.5"), 5);
        assert_eq!(parse_temp(b"-0.1"), -1);
        assert_eq!(parse_temp(b"99.9"), 999);
        assert_eq!(parse_temp(b"-99.9"), -999);
    }

    #[test]
    fn parse_temp_swar_basics() {
        assert_eq!(parse_temp_swar(b"12.3\nXXXX").0, 123);
        assert_eq!(parse_temp_swar(b"-12.3\nXXX").0, -123);
        assert_eq!(parse_temp_swar(b"0.5\nXXXXX").0, 5);
        assert_eq!(parse_temp_swar(b"-0.1\nXXXX").0, -1);
        assert_eq!(parse_temp_swar(b"12.3\nXXXX").1, 5);
        assert_eq!(parse_temp_swar(b"-12.3\nXXX").1, 6);
    }

    #[test]
    fn parse_temp_merykitty_basics() {
        assert_eq!(parse_temp_merykitty(b"12.3\nXXXX"), (123, 5));
        assert_eq!(parse_temp_merykitty(b"-12.3\nXXX"), (-123, 6));
        assert_eq!(parse_temp_merykitty(b"0.5\nXXXXX"), (5, 4));
        assert_eq!(parse_temp_merykitty(b"-0.1\nXXXX"), (-1, 5));
        assert_eq!(parse_temp_merykitty(b"99.9\nXXXX"), (999, 5));
        assert_eq!(parse_temp_merykitty(b"-99.9\nXXX"), (-999, 6));
        assert_eq!(parse_temp_merykitty(b"5.0\nXXXXX"), (50, 4));
    }

    #[test]
    fn chunks_cover_and_align() {
        let data: &[u8] = b"abc\ndef\nghij\nk\n";
        let bs = chunk_bounds(data, 3);
        let joined: Vec<u8> = bs.iter().flat_map(|(a, b)| data[*a..*b].to_vec()).collect();
        assert_eq!(joined, data);
        for (a, b) in &bs {
            if *b > 0 {
                assert_eq!(data[*b - 1], b'\n');
            }
            assert!(*a == 0 || data[*a - 1] == b'\n');
        }
    }

    #[test]
    fn mean_rounding_ties_toward_positive() {
        assert_eq!(round_ties_toward_positive(15, 2), 8);
        assert_eq!(round_ties_toward_positive(14, 2), 7);
        assert_eq!(round_ties_toward_positive(-15, 2), -7);
        assert_eq!(round_ties_toward_positive(-16, 2), -8);
    }
}
