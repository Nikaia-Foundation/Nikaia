//! AI variant 2: the same aggregation, written by an AI assistant over several
//! rounds, each measured with callgrind and improved. One vector compare finds
//! the `;` (SSE2, AVX2 where the CPU level has it), the name is hashed a word
//! at a time (crc32c where the CPU level has it, a multiply mixer otherwise),
//! the temperature is parsed without a branch, the stations live in an own
//! open-addressing table, and the end of the input is run from a zero-padded
//! copy, so that nothing reads past the end of the mapping. See
//! `benches/brc/README.md`.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// The main loop only starts rows below `len - MARGIN`; every read of a row
/// (the scan, the name's words, the temperature's word) stays below the row's
/// start + MARGIN.
const MARGIN: usize = 256;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("a measurements file is required");
    let file = std::fs::File::open(&path).expect("open the measurements");
    // SAFETY: the file is not changed while the program runs.
    let map = unsafe { memmap2::Mmap::map(&file).expect("map the measurements") };
    let data: &[u8] = &map;
    let cut = cut(data);
    let tail = tail(&data[cut..]);
    let mut stats = Table::new();
    run(data, cut, &mut stats);
    run(&tail, data.len() - cut, &mut stats);
    report(&stats);
}

/// Where the main loop ends: just after the last newline at or before
/// `len - MARGIN` (0 for a short file).
fn cut(data: &[u8]) -> usize {
    if data.len() <= MARGIN {
        return 0;
    }
    let mut i = data.len() - MARGIN;
    while i > 0 && data[i - 1] != b'\n' {
        i -= 1;
    }
    i
}

/// The rows after `cut`, copied into a buffer padded with zeros, and a `;`
/// right after them.
fn tail(rest: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8; rest.len() + MARGIN];
    v[..rest.len()].copy_from_slice(rest);
    v[rest.len()] = b';';
    v
}

#[inline(never)]
fn run<'a>(buf: &'a [u8], stop: usize, stats: &mut Table<'a>) {
    let p = buf.as_ptr();
    let mut pos = 0usize;
    while pos < stop {
        // SAFETY: a row starts below `stop`, which is at least MARGIN bytes
        // before the end of `buf`, and every read stays within MARGIN of it.
        unsafe {
            let sep = find_semi(p, pos);
            let h = name_hash(p.add(pos), sep - pos);
            let (v, n) = temp_swar(p.add(sep + 1));
            stats.update(h, &buf[pos..sep], v);
            pos = sep + 1 + n;
        }
    }
}

// ---- the `;`: compare a vector of bytes, take the first set bit -------------

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
const W: usize = 32;
#[cfg(all(target_arch = "x86_64", not(target_feature = "avx2")))]
const W: usize = 16;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn mask_semi(p: *const u8) -> u32 {
    #[cfg(target_feature = "avx2")]
    {
        let v = _mm256_loadu_si256(p as *const __m256i);
        _mm256_movemask_epi8(_mm256_cmpeq_epi8(v, _mm256_set1_epi8(b';' as i8))) as u32
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        let v = _mm_loadu_si128(p as *const __m128i);
        _mm_movemask_epi8(_mm_cmpeq_epi8(v, _mm_set1_epi8(b';' as i8))) as u32
    }
}

/// The `;` at or after `pos`.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn find_semi(p: *const u8, pos: usize) -> usize {
    let mut base = pos;
    loop {
        let m = mask_semi(p.add(base));
        if m != 0 {
            return base + m.trailing_zeros() as usize;
        }
        base += W;
    }
}

/// The `;` at or after `pos`, a word at a time where there is no x86 vector.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
unsafe fn find_semi(p: *const u8, pos: usize) -> usize {
    const ONES: u64 = 0x0101_0101_0101_0101;
    let mut base = pos;
    loop {
        let x = word(p.add(base)) ^ (ONES * b';' as u64);
        let m = x.wrapping_sub(ONES) & !x & (ONES << 7);
        if m != 0 {
            return base + (m.trailing_zeros() / 8) as usize;
        }
        base += 8;
    }
}

// ---- the name's hash, from its words ----------------------------------------

#[inline(always)]
unsafe fn word(p: *const u8) -> u64 {
    u64::from_le((p as *const u64).read_unaligned())
}

/// The low `n` bytes of `w` (all of it when `n` is 8 or more).
#[inline(always)]
fn low_bytes(w: u64, n: usize) -> u64 {
    #[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
    {
        // SAFETY: the target has bmi2.
        unsafe { _bzhi_u64(w, (n * 8) as u32) }
    }
    #[cfg(not(all(target_arch = "x86_64", target_feature = "bmi2")))]
    {
        if n >= 8 {
            w
        } else {
            w & ((1u64 << (n * 8)) - 1)
        }
    }
}

/// A hash of the name's first 16 bytes and its length; the table compares the
/// whole name. crc32c where the CPU level has it, else a multiply mixer.
#[inline(always)]
unsafe fn name_hash(p: *const u8, len: usize) -> u64 {
    let a = low_bytes(word(p), len);
    let b = low_bytes(word(p.add(8)), len.saturating_sub(8));
    #[cfg(all(target_arch = "x86_64", target_feature = "sse4.2"))]
    {
        _mm_crc32_u64(_mm_crc32_u64(len as u64, a), b)
    }
    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse4.2")))]
    {
        mix(mix(len as u64, a), b)
    }
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse4.2")))]
#[inline(always)]
fn mix(h: u64, w: u64) -> u64 {
    (h.rotate_left(5) ^ w).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95)
}

// ---- the temperature, without a branch --------------------------------------

/// The word after the `;`: (tenths, bytes up to and including the `\n`).
#[inline(always)]
unsafe fn temp_swar(p: *const u8) -> (i64, usize) {
    let w = word(p);
    let dot = (!w & 0x1010_1000).trailing_zeros();
    let signed = (((!w) << 59) as i64) >> 63;
    let keep = !((signed as u64) & 0xFF);
    let digits = ((w & keep) << (28 - dot)) & 0x0F_000F_0F00;
    let abs = (digits.wrapping_mul(0x640a_0001) >> 32) & 0x3FF;
    (((abs as i64) ^ signed) - signed, (dot >> 3) as usize + 3)
}

// ---- the stations: open addressing, linear probing ---------------------------

const BITS: u32 = 12;

#[derive(Clone, Copy)]
struct Slot<'a> {
    name: &'a [u8],
    h: u64,
    /// min, max, sum, count.
    s: [i64; 4],
}

struct Table<'a> {
    slots: Vec<Slot<'a>>,
}

impl<'a> Table<'a> {
    fn new() -> Self {
        let empty = Slot {
            name: &[],
            h: 0,
            s: [0; 4],
        };
        Table {
            slots: vec![empty; 1 << BITS],
        }
    }

    #[inline(always)]
    fn update(&mut self, h: u64, name: &'a [u8], v: i64) {
        let mask = (1usize << BITS) - 1;
        let mut i = (h.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - BITS)) as usize;
        loop {
            // SAFETY: `i` is masked to the table's length.
            let s = unsafe { self.slots.get_unchecked_mut(i) };
            if s.h == h && s.name == name {
                let e = &mut s.s;
                if v < e[0] {
                    e[0] = v
                }
                if v > e[1] {
                    e[1] = v
                }
                e[2] += v;
                e[3] += 1;
                return;
            }
            if s.name.is_empty() {
                *s = Slot {
                    name,
                    h,
                    s: [v, v, v, 1],
                };
                return;
            }
            i = (i + 1) & mask;
        }
    }
}

// ---- the output --------------------------------------------------------------

fn report(stats: &Table) {
    let mut rows: Vec<_> = stats.slots.iter().filter(|s| !s.name.is_empty()).collect();
    rows.sort_unstable_by(|a, b| a.name.cmp(b.name));
    let body: Vec<String> = rows
        .into_iter()
        .map(|r| {
            let name = std::str::from_utf8(r.name).expect("a station name is text");
            let s = r.s;
            format!(
                "{}={}/{}/{}",
                name,
                tenths(s[0]),
                tenths((2 * s[2] + s[3]).div_euclid(2 * s[3])),
                tenths(s[1])
            )
        })
        .collect();
    println!("{{{}}}", body.join(", "));
}

fn tenths(t: i64) -> String {
    let a = t.unsigned_abs();
    format!("{}{}.{}", if t < 0 { "-" } else { "" }, a / 10, a % 10)
}
