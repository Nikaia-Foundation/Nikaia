//! Issue #876's workloads in Rust, beside `wide.nika`, printing the same line.
//!
//! `wide-ops WORKLOAD VARIANT N`:
//! * `emu`: the algorithm `wide.nika` writes, line for line (32-bit halves,
//!   compare carries, a table for the carry-less product), so `emu` against
//!   Nikaia is the language and `native` against `emu` is the instruction;
//! * `native`: what tuned Rust writes: `u128`, `_addcarry_u64`,
//!   `PCLMULQDQ`;
//! * `mulmod` also has `u128rem` (`(a * b % m) as u64` on `u128`, which LLVM
//!   lowers to a call of `__umodti3`) and `div` (one `DIV r64`, the 128 ÷ 64
//!   instruction `wide_rem` would reach).
//!
//! A variant a workload does not have stops with a message.

use std::hint::black_box;

const M32: u64 = 0xffff_ffff;

struct Rng {
    state: u64,
}

impl Rng {
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

fn fold(h: u64, w: u64) -> u64 {
    (h ^ w).wrapping_mul(0x100_0000_01b3)
}

#[inline]
fn mul_wide_emu(a: u64, b: u64) -> (u64, u64) {
    let (a0, a1, b0, b1) = (a & M32, a >> 32, b & M32, b >> 32);
    let (p00, p01, p10, p11) = (a0 * b0, a0 * b1, a1 * b0, a1 * b1);
    let mid = (p00 >> 32) + (p01 & M32) + (p10 & M32);
    let lo = (p00 & M32) | (mid << 32);
    let hi = p11 + (p01 >> 32) + (p10 >> 32) + (mid >> 32);
    (lo, hi)
}

#[inline]
fn mul_wide_native(a: u64, b: u64) -> (u64, u64) {
    let p = a as u128 * b as u128;
    (p as u64, (p >> 64) as u64)
}

// ---- 1. bigint, 4096 bits ----

const LIMBS32: usize = 128;
const LIMBS64: usize = 64;

fn random_big64(rng: &mut Rng) -> Vec<u64> {
    (0..LIMBS64).map(|_| rng.next()).collect()
}

fn to32(v: &[u64]) -> Vec<u32> {
    v.iter()
        .flat_map(|w| [*w as u32, (*w >> 32) as u32])
        .collect()
}

fn sum32(a: &[u32]) -> u64 {
    let mut h = 0;
    for k in 0..LIMBS32 / 2 {
        h = fold(h, a[2 * k] as u64 | ((a[2 * k + 1] as u64) << 32));
    }
    h
}

fn sum64(a: &[u64]) -> u64 {
    a.iter().fold(0, |h, w| fold(h, *w))
}

fn big_add32(a: &mut [u32], b: &[u32]) {
    let mut carry = 0u64;
    for i in 0..LIMBS32 {
        let s = a[i] as u64 + b[i] as u64 + carry;
        a[i] = s as u32;
        carry = s >> 32;
    }
}

fn big_add64(a: &mut [u64; LIMBS64], b: &[u64; LIMBS64]) {
    use core::arch::x86_64::_addcarry_u64;
    let mut carry = 0u8;
    for i in 0..LIMBS64 {
        let mut out = 0;
        carry = _addcarry_u64(carry, a[i], b[i], &mut out);
        a[i] = out;
    }
}

fn bigadd(variant: &str, n: u64) -> u64 {
    let mut rng = Rng { state: 1 };
    let a = random_big64(&mut rng);
    let b = random_big64(&mut rng);
    match variant {
        "emu" => {
            let (mut a, mut b) = (to32(&a), to32(&b));
            for _ in 0..n {
                big_add32(black_box(&mut a), &b);
                big_add32(black_box(&mut b), &a);
            }
            sum32(&a) ^ sum32(&b)
        }
        "native" => {
            let mut a: [u64; LIMBS64] = a.try_into().unwrap();
            let mut b: [u64; LIMBS64] = b.try_into().unwrap();
            for _ in 0..n {
                big_add64(black_box(&mut a), &b);
                big_add64(black_box(&mut b), &a);
            }
            sum64(&a) ^ sum64(&b)
        }
        _ => no(variant),
    }
}

fn big_mul32(a: &[u32], b: &[u32], r: &mut [u32]) {
    r.fill(0);
    for i in 0..LIMBS32 {
        let mut carry = 0u64;
        let ai = a[i] as u64;
        for j in 0..LIMBS32 {
            let t = ai * b[j] as u64 + r[i + j] as u64 + carry;
            r[i + j] = t as u32;
            carry = t >> 32;
        }
        r[i + LIMBS32] = carry as u32;
    }
}

fn big_mul64(a: &[u64; LIMBS64], b: &[u64; LIMBS64], r: &mut [u64; 2 * LIMBS64]) {
    r.fill(0);
    for i in 0..LIMBS64 {
        let mut carry = 0u64;
        let ai = a[i] as u128;
        for j in 0..LIMBS64 {
            let t = ai * b[j] as u128 + r[i + j] as u128 + carry as u128;
            r[i + j] = t as u64;
            carry = (t >> 64) as u64;
        }
        r[i + LIMBS64] = carry;
    }
}

fn bigmul(variant: &str, n: u64) -> u64 {
    let mut rng = Rng { state: 2 };
    let a = random_big64(&mut rng);
    let b = random_big64(&mut rng);
    match variant {
        "emu" => {
            let (mut a, b) = (to32(&a), to32(&b));
            let mut r = vec![0u32; 2 * LIMBS32];
            for _ in 0..n {
                big_mul32(black_box(&a), &b, &mut r);
                let mut carry = 0u64;
                for k in 0..LIMBS32 {
                    let s = r[k] as u64 + r[k + LIMBS32] as u64 + carry;
                    a[k] = s as u32;
                    carry = s >> 32;
                }
            }
            sum32(&a)
        }
        "native" => {
            let mut a: [u64; LIMBS64] = a.try_into().unwrap();
            let b: [u64; LIMBS64] = b.try_into().unwrap();
            let mut r = [0u64; 2 * LIMBS64];
            for _ in 0..n {
                big_mul64(black_box(&a), &b, &mut r);
                let mut carry = 0u128;
                for k in 0..LIMBS64 {
                    let s = r[k] as u128 + r[k + LIMBS64] as u128 + carry;
                    a[k] = s as u64;
                    carry = s >> 64;
                }
            }
            sum64(&a)
        }
        _ => no(variant),
    }
}

// ---- 2. Miller-Rabin over 64-bit odd numbers ----

const BASES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

struct Mont {
    m: u64,
    inv: u64,
    r1: u64,
    r2: u64,
}

fn add_mod(a: u64, b: u64, m: u64) -> u64 {
    if a >= m - b { a - (m - b) } else { a + b }
}

impl Mont {
    fn new(m: u64) -> Mont {
        let mut inv = m;
        for _ in 0..5 {
            inv = inv.wrapping_mul(2u64.wrapping_sub(m.wrapping_mul(inv)));
        }
        let r1 = 0u64.wrapping_sub(m) % m;
        let mut r2 = r1;
        for _ in 0..64 {
            r2 = add_mod(r2, r2, m);
        }
        Mont { m, inv, r1, r2 }
    }

    #[inline]
    fn mul<const NATIVE: bool>(&self, a: u64, b: u64) -> u64 {
        let wide = if NATIVE {
            mul_wide_native
        } else {
            mul_wide_emu
        };
        let (lo, hi) = wide(a, b);
        let q = lo.wrapping_mul(self.inv);
        let (_, h) = wide(q, self.m);
        if hi < h {
            hi.wrapping_sub(h).wrapping_add(self.m)
        } else {
            hi - h
        }
    }
}

fn is_prime_mont<const NATIVE: bool>(n: u64) -> bool {
    let t = Mont::new(n);
    let s = (n - 1).trailing_zeros();
    let d = (n - 1) >> s;
    let (one, minus_one) = (t.r1, n - t.r1);
    'bases: for base in BASES {
        let mut x = one;
        let mut p = t.mul::<NATIVE>(base, t.r2);
        let mut e = d;
        while e > 0 {
            if e & 1 == 1 {
                x = t.mul::<NATIVE>(x, p);
            }
            p = t.mul::<NATIVE>(p, p);
            e >>= 1;
        }
        if x == one || x == minus_one {
            continue;
        }
        for _ in 1..s {
            x = t.mul::<NATIVE>(x, x);
            if x == minus_one {
                continue 'bases;
            }
        }
        return false;
    }
    true
}

#[inline]
fn mulmod_u128(a: u64, b: u64, m: u64) -> u64 {
    (a as u128 * b as u128 % m as u128) as u64
}

/// `a·b mod m` with one `DIV r64`; `a, b < m`, so the high word is below `m`
/// and the quotient fits (no `#DE`).
#[inline]
fn mulmod_div(a: u64, b: u64, m: u64) -> u64 {
    let (lo, hi) = mul_wide_native(a, b);
    let rem: u64;
    // SAFETY: hi < m because a, b < m, so the quotient fits in 64 bits.
    unsafe {
        core::arch::asm!("div {m}", m = in(reg) m, inout("rax") lo => _,
            inout("rdx") hi => rem, options(pure, nomem, nostack));
    }
    rem
}

fn is_prime_plain(n: u64, mulmod: fn(u64, u64, u64) -> u64) -> bool {
    let s = (n - 1).trailing_zeros();
    let d = (n - 1) >> s;
    'bases: for base in BASES {
        let mut x = 1;
        let mut p = base;
        let mut e = d;
        while e > 0 {
            if e & 1 == 1 {
                x = mulmod(x, p, n);
            }
            p = mulmod(p, p, n);
            e >>= 1;
        }
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mulmod(x, x, n);
            if x == n - 1 {
                continue 'bases;
            }
        }
        return false;
    }
    true
}

fn mulmod(variant: &str, n: u64) -> u64 {
    let test: Box<dyn Fn(u64) -> bool> = match variant {
        "emu" => Box::new(is_prime_mont::<false>),
        "native" => Box::new(is_prime_mont::<true>),
        "u128rem" => Box::new(|c| is_prime_plain(c, mulmod_u128)),
        "div" => Box::new(|c| is_prime_plain(c, mulmod_div)),
        _ => no(variant),
    };
    let mut rng = Rng { state: 3 };
    let (mut h, mut primes) = (0, 0);
    for _ in 0..n {
        let c = rng.next() | 0x8000_0000_0000_0001;
        if test(black_box(c)) {
            primes += 1;
            h = fold(h, c);
        }
    }
    fold(h, primes)
}

// ---- 3. wyhash mix, PCG64 ----

fn wymix_with(n: u64, wide: fn(u64, u64) -> (u64, u64)) -> u64 {
    let mut h: u64 = 0x2d35_8dcc_aa6c_78a5;
    for i in 0..n {
        let x = i.wrapping_mul(0x8bb8_4b93_962e_acc9);
        let (lo, hi) = wide(h ^ 0xa076_1d64_78bd_642f, x ^ 0xe703_7ed1_a0b4_28db);
        h = lo ^ hi;
    }
    h
}

fn wymix(variant: &str, n: u64) -> u64 {
    match variant {
        "emu" => wymix_with(n, mul_wide_emu),
        "native" => wymix_with(n, mul_wide_native),
        _ => no(variant),
    }
}

const PCG_MUL: u128 = 0x2360_ed05_1fc6_5da4_4385_df64_9fcc_f645;
const PCG_INC: u128 = 0x5851_f42d_4c95_7f2d_1405_7b7e_f767_814f;

fn pcg64(variant: &str, n: u64) -> u64 {
    let (mul_hi, mul_lo) = ((PCG_MUL >> 64) as u64, PCG_MUL as u64);
    let (inc_hi, inc_lo) = ((PCG_INC >> 64) as u64, PCG_INC as u64);
    let mut hi: u64 = 0x0123_4567_89ab_cdef;
    let mut lo: u64 = 0xfedc_ba98_7654_3210;
    let mut sum = 0u64;
    match variant {
        "emu" => {
            for _ in 0..n {
                let (plo, phi) = mul_wide_emu(lo, mul_lo);
                let cross = lo
                    .wrapping_mul(mul_hi)
                    .wrapping_add(hi.wrapping_mul(mul_lo));
                let nlo = plo.wrapping_add(inc_lo);
                let carry = (nlo < plo) as u64;
                hi = phi
                    .wrapping_add(cross)
                    .wrapping_add(inc_hi)
                    .wrapping_add(carry);
                lo = nlo;
                sum = sum.wrapping_add((hi ^ lo).rotate_right((hi >> 58) as u32));
            }
        }
        "native" => {
            let mut state = (hi as u128) << 64 | lo as u128;
            for _ in 0..n {
                state = state.wrapping_mul(PCG_MUL).wrapping_add(PCG_INC);
                let (hi, lo) = ((state >> 64) as u64, state as u64);
                sum = sum.wrapping_add((hi ^ lo).rotate_right((hi >> 58) as u32));
            }
        }
        _ => no(variant),
    }
    sum
}

// ---- 4. CRC-64/ECMA-182, GHASH-style GF(2^128) ----

const CRC_POLY: u64 = 0x42f0_e1eb_a9ea_3693;
const DATA_WORDS: usize = 8192;

fn crc_table() -> [u64; 256] {
    let mut table = [0; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = (i as u64) << 56;
        for _ in 0..8 {
            c = if c >> 63 == 1 {
                (c << 1) ^ CRC_POLY
            } else {
                c << 1
            };
        }
        *slot = c;
    }
    table
}

/// floor(x^128 / P) without its x^64 term, by long division in GF(2).
fn barrett_mu() -> u64 {
    // remainder of x^64 / P is CRC_POLY; walk 64 more quotient bits
    let mut rem = CRC_POLY;
    let mut q = 0u64;
    for _ in 0..64 {
        let top = rem >> 63;
        q = (q << 1) | top;
        rem = (rem << 1) ^ if top == 1 { CRC_POLY } else { 0 };
    }
    q
}

#[target_feature(enable = "pclmulqdq,sse2,sse4.1")]
fn clmul_native(a: u64, b: u64) -> (u64, u64) {
    use core::arch::x86_64::*;
    let p = _mm_clmulepi64_si128(_mm_set_epi64x(0, a as i64), _mm_set_epi64x(0, b as i64), 0);
    (
        _mm_cvtsi128_si64(p) as u64,
        _mm_extract_epi64::<1>(p) as u64,
    )
}

/// `(crc ^ w)·x^64 mod P` by Barrett: two carry-less products.
#[target_feature(enable = "pclmulqdq,sse2,sse4.1")]
fn crc_step_clmul(crc: u64, w: u64, mu: u64) -> u64 {
    let a = crc ^ w;
    let q = clmul_native(a, mu).1 ^ a;
    clmul_native(q, CRC_POLY).0
}

fn crc64(variant: &str, n: u64) -> u64 {
    let mut rng = Rng { state: 4 };
    let data: Vec<u64> = (0..DATA_WORDS).map(|_| rng.next()).collect();
    let mut crc = 0u64;
    match variant {
        "emu" => {
            let table = crc_table();
            for _ in 0..n {
                for &w in black_box(&data) {
                    for k in 0..8 {
                        let b = (w >> (56 - 8 * k)) & 0xff;
                        crc = table[((crc >> 56) ^ b) as usize] ^ (crc << 8);
                    }
                }
            }
        }
        "native" => {
            let mu = barrett_mu();
            for _ in 0..n {
                for &w in black_box(&data) {
                    // SAFETY: the bench runs on x86-64 with PCLMULQDQ (checked in main).
                    crc = unsafe { crc_step_clmul(crc, w, mu) };
                }
            }
        }
        _ => no(variant),
    }
    crc
}

fn nibble_table(h: u64) -> [u64; 32] {
    let mut t = [0; 32];
    for k in 1..16 {
        let (mut lo, mut hi) = (0u64, 0u64);
        for bit in 0..4 {
            if (k >> bit) & 1 == 1 {
                lo ^= h << bit;
                if bit > 0 {
                    hi ^= h >> (64 - bit);
                }
            }
        }
        t[2 * k] = lo;
        t[2 * k + 1] = hi;
    }
    t
}

#[inline]
fn clmul_table(t: &[u64; 32], x: u64) -> (u64, u64) {
    let (mut lo, mut hi) = (0u64, 0u64);
    let mut s = 60i32;
    while s >= 0 {
        hi = (hi << 4) | (lo >> 60);
        lo <<= 4;
        let k = ((x >> s) & 0xf) as usize;
        lo ^= t[2 * k];
        hi ^= t[2 * k + 1];
        s -= 4;
    }
    (lo, hi)
}

#[inline]
fn times87(x: u64) -> (u64, u64) {
    (
        x ^ (x << 1) ^ (x << 2) ^ (x << 7),
        (x >> 63) ^ (x >> 62) ^ (x >> 57),
    )
}

#[inline]
fn gf_reduce(p0: u64, mut p1: u64, mut p2: u64, p3: u64) -> (u64, u64) {
    let (e_lo, e_hi) = times87(p3);
    p1 ^= e_lo;
    p2 ^= e_hi;
    let (f_lo, f_hi) = times87(p2);
    (p0 ^ f_lo, p1 ^ f_hi)
}

#[target_feature(enable = "pclmulqdq,sse2,sse4.1")]
fn gf_mul_native(x_lo: u64, x_hi: u64, h_lo: u64, h_hi: u64) -> (u64, u64) {
    let (a_lo, a_hi) = clmul_native(x_lo, h_lo);
    let (b_lo, b_hi) = clmul_native(x_lo, h_hi);
    let (c_lo, c_hi) = clmul_native(x_hi, h_lo);
    let (d_lo, d_hi) = clmul_native(x_hi, h_hi);
    gf_reduce(a_lo, a_hi ^ b_lo ^ c_lo, b_hi ^ c_hi ^ d_lo, d_hi)
}

fn ghash(variant: &str, n: u64) -> u64 {
    let mut rng = Rng { state: 5 };
    let (h_lo, h_hi) = (rng.next(), rng.next());
    let (mut y_lo, mut y_hi) = (0u64, 0u64);
    match variant {
        "emu" => {
            let (t_lo, t_hi) = (nibble_table(h_lo), nibble_table(h_hi));
            for _ in 0..n {
                let x_lo = y_lo ^ rng.next();
                let x_hi = y_hi ^ rng.next();
                let (a_lo, a_hi) = clmul_table(&t_lo, x_lo);
                let (b_lo, b_hi) = clmul_table(&t_hi, x_lo);
                let (c_lo, c_hi) = clmul_table(&t_lo, x_hi);
                let (d_lo, d_hi) = clmul_table(&t_hi, x_hi);
                (y_lo, y_hi) = gf_reduce(a_lo, a_hi ^ b_lo ^ c_lo, b_hi ^ c_hi ^ d_lo, d_hi);
            }
        }
        "native" => {
            for _ in 0..n {
                let x_lo = y_lo ^ rng.next();
                let x_hi = y_hi ^ rng.next();
                // SAFETY: PCLMULQDQ and SSE4.1 are checked in main.
                (y_lo, y_hi) = unsafe { gf_mul_native(x_lo, x_hi, h_lo, h_hi) };
            }
        }
        _ => no(variant),
    }
    fold(y_lo, y_hi)
}

fn no(variant: &str) -> ! {
    eprintln!("no variant {variant} of this workload");
    std::process::exit(2)
}

fn main() {
    assert!(
        is_x86_feature_detected!("pclmulqdq") && is_x86_feature_detected!("sse4.1"),
        "the native variants need PCLMULQDQ and SSE4.1"
    );
    let args: Vec<String> = std::env::args().collect();
    let workload = args.get(1).map_or("wymix", String::as_str);
    let variant = args.get(2).map_or("native", String::as_str);
    let n: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let sum = match workload {
        "bigadd" => bigadd(variant, n),
        "bigmul" => bigmul(variant, n),
        "mulmod" => mulmod(variant, n),
        "wymix" => wymix(variant, n),
        "pcg64" => pcg64(variant, n),
        "crc64" => crc64(variant, n),
        "ghash" => ghash(variant, n),
        _ => {
            eprintln!("no workload {workload}");
            std::process::exit(2)
        }
    };
    println!("{workload} {n} {sum}");
}
