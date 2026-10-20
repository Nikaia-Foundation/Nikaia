//! The scan alone, against `scan.nika`: where the separators of every row
//! are, found the way `ai2.rs` finds them - the `;` by a vector compare, the
//! `\n` from the temperature's shape - with no name hash and no table. Prints
//! the count of separators and the xor of their positions. See `README.md`.
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
const MARGIN: usize = 256;

fn main() {
    let path = std::env::args().nth(1).expect("a file");
    let file = std::fs::File::open(&path).expect("open");
    // SAFETY: the file is not changed while the program runs.
    let map = unsafe { memmap2::Mmap::map(&file).expect("map") };
    let data: &[u8] = &map;
    let cut = cut(data);
    let tail = tail(&data[cut..]);
    let (mut n, mut x) = (0u64, 0u64);
    run(data, cut, 0, &mut n, &mut x);
    run(&tail, data.len() - cut, cut, &mut n, &mut x);
    println!("{n} {x}");
}

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

fn tail(rest: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8; rest.len() + MARGIN];
    v[..rest.len()].copy_from_slice(rest);
    v[rest.len()] = b';';
    v
}

#[inline(never)]
fn run(buf: &[u8], stop: usize, off: usize, n: &mut u64, x: &mut u64) {
    let p = buf.as_ptr();
    let mut pos = 0usize;
    while pos < stop {
        // SAFETY: as in `ai2.rs`: a row starts below `stop`, at least MARGIN
        // bytes before the end of `buf`, and every read stays within MARGIN.
        unsafe {
            let sep = find_semi(p, pos);
            let len = temp_len(p.add(sep + 1));
            *n += 2;
            *x ^= (off + sep) as u64 ^ (off + sep + len) as u64;
            pos = sep + 1 + len;
        }
    }
}

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
        let w = u64::from_le((p.add(base) as *const u64).read_unaligned());
        let x = w ^ (ONES * b';' as u64);
        let m = x.wrapping_sub(ONES) & !x & (ONES << 7);
        if m != 0 {
            return base + (m.trailing_zeros() / 8) as usize;
        }
        base += 8;
    }
}

/// The bytes after the `;` up to and including the `\n` (ai2's `temp_swar`).
#[inline(always)]
unsafe fn temp_len(p: *const u8) -> usize {
    let w = u64::from_le((p as *const u64).read_unaligned());
    let dot = (!w & 0x1010_1000).trailing_zeros();
    (dot >> 3) as usize + 3
}
