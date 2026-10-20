//! **Vectors a program holds** ([ADR-377](../../../docs/specification/adr/adr-377.md)):
//! `simd::Lanes[T]` at the build's width, `simd::Fixed[T, N]` at a width the
//! algorithm names, and `simd::Mask[T]` / `simd::Mask[T, N]`, what a
//! comparison answers.
//!
//! **The meaning is the same at every width, the lowering is the backend's**
//! (D6): a vector is an array of `N` lanes, and every operation is written
//! lane by lane over it, `#[inline(always)]`, which LLVM turns into the
//! vector instructions the CPU level offers: `pcmpeqq`, `pmuludq`, `pshufb`
//! for a constant shuffle, `pmaddubsw` / `pmaddwd` for `dot_pairs`. No
//! `unsafe` is written, here or in the program.
//!
//! **`Lanes[T]` holds as many lanes as the build's widest integer register**
//! (D1): 16 bytes at x86-64's baseline and on aarch64, 32 where the build
//! enables AVX2 (`--cpu=x86-64-v3`).

use std::ops::{BitAnd, BitOr, BitXor, Not, Shl, Shr};

/// The bytes of the build's widest integer vector register (D1).
pub const WIDTH: usize = if cfg!(all(target_arch = "x86_64", target_feature = "avx2")) {
    32
} else {
    16
};

/// **A lane's type**: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`.
pub trait Lane: Copy + Default + PartialEq + PartialOrd + std::fmt::Debug + 'static {
    /// Every bit set: a mask lane that holds.
    const ONES: Self;
    /// The bytes one lane takes.
    const SIZE: usize;
    /// `Lanes[Self]`: as many lanes as the build's widest register holds.
    type Lanes: Copy;
    /// `Mask[Self]`: what a comparison of two `Lanes[Self]` answers.
    type LanesMask: Copy;
    fn wrapping_add(self, other: Self) -> Self;
    fn wrapping_sub(self, other: Self) -> Self;
    fn wrapping_mul(self, other: Self) -> Self;
    fn saturating_add(self, other: Self) -> Self;
    fn saturating_sub(self, other: Self) -> Self;
    fn and(self, other: Self) -> Self;
    fn or(self, other: Self) -> Self;
    fn xor(self, other: Self) -> Self;
    fn not(self) -> Self;
    fn shl(self, k: u32) -> Self;
    fn shr(self, k: u32) -> Self;
    /// The lane read from `Self::SIZE` bytes, least significant first.
    fn from_le(bytes: &[u8]) -> Self;
}

macro_rules! lane {
    ($t:ty, $size:expr) => {
        impl Lane for $t {
            const ONES: Self = !0;
            const SIZE: usize = $size;
            type Lanes = Fixed<$t, { WIDTH / $size }>;
            type LanesMask = Mask<$t, { WIDTH / $size }>;
            #[inline(always)]
            fn wrapping_add(self, other: Self) -> Self {
                <$t>::wrapping_add(self, other)
            }
            #[inline(always)]
            fn wrapping_sub(self, other: Self) -> Self {
                <$t>::wrapping_sub(self, other)
            }
            #[inline(always)]
            fn wrapping_mul(self, other: Self) -> Self {
                <$t>::wrapping_mul(self, other)
            }
            #[inline(always)]
            fn saturating_add(self, other: Self) -> Self {
                <$t>::saturating_add(self, other)
            }
            #[inline(always)]
            fn saturating_sub(self, other: Self) -> Self {
                <$t>::saturating_sub(self, other)
            }
            #[inline(always)]
            fn and(self, other: Self) -> Self {
                self & other
            }
            #[inline(always)]
            fn or(self, other: Self) -> Self {
                self | other
            }
            #[inline(always)]
            fn xor(self, other: Self) -> Self {
                self ^ other
            }
            #[inline(always)]
            fn not(self) -> Self {
                !self
            }
            #[inline(always)]
            fn shl(self, k: u32) -> Self {
                <$t>::wrapping_shl(self, k)
            }
            #[inline(always)]
            fn shr(self, k: u32) -> Self {
                <$t>::wrapping_shr(self, k)
            }
            #[inline(always)]
            fn from_le(bytes: &[u8]) -> Self {
                let mut word = [0u8; $size];
                word.copy_from_slice(bytes);
                <$t>::from_le_bytes(word)
            }
        }
    };
}

lane!(u8, 1);
lane!(i8, 1);
lane!(u16, 2);
lane!(i16, 2);
lane!(u32, 4);
lane!(i32, 4);
lane!(u64, 8);
lane!(i64, 8);

/// **`simd::Lanes[T]`**: `T`'s lanes at the build's width (D1).
pub type Lanes<T> = <T as Lane>::Lanes;
/// **`simd::Mask[T]`**: what a comparison of two `Lanes[T]` answers (D5).
pub type LanesMask<T> = <T as Lane>::LanesMask;

/// **`simd::Fixed[T, N]`**: `N` lanes of `T` on every target (D2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fixed<T: Lane, const N: usize>([T; N]);

/// **`simd::Mask[T, N]`**: one answer a lane, read as positions (D5). A lane
/// that holds is all ones, one that does not all zeros, as the compare
/// instructions write them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mask<T: Lane, const N: usize>([T; N]);

/// **A vector a load or a `splat` makes**: the type the program wrote down.
pub trait Vector: Copy {
    /// The lane's type.
    type Item: Lane;
    /// Every lane `x`.
    fn splat(x: Self::Item) -> Self;
    /// The lanes read from `bytes`, which hold exactly as many as fit.
    fn from_bytes(bytes: &[u8]) -> Self;
    /// The bytes it reads.
    const BYTES: usize;
}

impl<T: Lane, const N: usize> Vector for Fixed<T, N> {
    type Item = T;
    const BYTES: usize = N * T::SIZE;
    #[inline(always)]
    fn splat(x: T) -> Self {
        Fixed([x; N])
    }
    #[inline(always)]
    fn from_bytes(bytes: &[u8]) -> Self {
        Fixed(std::array::from_fn(|i| {
            T::from_le(&bytes[i * T::SIZE..(i + 1) * T::SIZE])
        }))
    }
}

/// **`simd::load(b, at)`**: the lanes in `b[at..<at + count * size]`, least
/// significant byte first. It reads what that cut reads and stops the
/// program where that cut would (D3).
#[inline(always)]
#[track_caller]
pub fn load<V: Vector>(bytes: &[u8], at: i64) -> V {
    let cut = crate::index::get(
        bytes,
        crate::index::at(at..at.wrapping_add(V::BYTES as i64)),
    );
    V::from_bytes(cut)
}

/// **A load the bounds walk proved inside** (ADR-306 D5): the bytes it
/// reads, handed over by `proven::array`.
#[inline(always)]
pub fn load_proven<V: Vector, const B: usize>(bytes: &[u8; B]) -> V {
    V::from_bytes(bytes)
}

/// **`simd::splat(x)`**: every lane `x`.
#[inline(always)]
pub fn splat<V: Vector>(x: V::Item) -> V {
    V::splat(x)
}

/// **`simd::select(mask, a, b)`**: `a`'s lane where the mask holds, `b`'s
/// where it does not.
#[inline(always)]
pub fn select<T: Lane, const N: usize>(
    mask: Mask<T, N>,
    a: Fixed<T, N>,
    b: Fixed<T, N>,
) -> Fixed<T, N> {
    Fixed(std::array::from_fn(|i| {
        a.0[i].and(mask.0[i]).or(b.0[i].and(mask.0[i].not()))
    }))
}

macro_rules! lanewise {
    ($($name:ident),*) => {
        $(
            #[inline(always)]
            pub fn $name(self, other: Self) -> Self {
                Fixed(std::array::from_fn(|i| self.0[i].$name(other.0[i])))
            }
        )*
    };
}

macro_rules! compare {
    ($($name:ident $op:tt),*) => {
        $(
            #[inline(always)]
            pub fn $name(self, other: Self) -> Mask<T, N> {
                Mask(std::array::from_fn(|i| if self.0[i] $op other.0[i] { T::ONES } else { T::default() }))
            }
        )*
    };
}

impl<T: Lane, const N: usize> Fixed<T, N> {
    /// **`Fixed.from([…])`**: the lanes in the order written.
    #[inline(always)]
    pub fn from(lanes: [T; N]) -> Self {
        Fixed(lanes)
    }

    lanewise!(
        wrapping_add,
        wrapping_sub,
        wrapping_mul,
        saturating_add,
        saturating_sub
    );
    compare!(eq ==, ne !=, lt <, le <=, gt >, ge >=);

    /// The smaller of each pair of lanes.
    #[inline(always)]
    pub fn min(self, other: Self) -> Self {
        Fixed(std::array::from_fn(|i| {
            if other.0[i] < self.0[i] {
                other.0[i]
            } else {
                self.0[i]
            }
        }))
    }

    /// The larger of each pair of lanes.
    #[inline(always)]
    pub fn max(self, other: Self) -> Self {
        Fixed(std::array::from_fn(|i| {
            if other.0[i] > self.0[i] {
                other.0[i]
            } else {
                self.0[i]
            }
        }))
    }

    /// **`lane(i)`**, checked as an index is.
    #[inline(always)]
    #[track_caller]
    pub fn lane(self, i: i64) -> T {
        *crate::index::get(&self.0[..], crate::index::at(i))
    }

    /// The lanes added, wrapping.
    #[inline(always)]
    pub fn wrapping_sum(self) -> T {
        self.0
            .iter()
            .fold(T::default(), |sum, &x| sum.wrapping_add(x))
    }

    /// The smallest lane.
    #[inline(always)]
    pub fn min_lane(self) -> T {
        self.0
            .iter()
            .fold(self.0[0], |m, &x| if x < m { x } else { m })
    }

    /// The largest lane.
    #[inline(always)]
    pub fn max_lane(self) -> T {
        self.0
            .iter()
            .fold(self.0[0], |m, &x| if x > m { x } else { m })
    }

    /// The lanes in the order `of` names them. The checker has proved each
    /// index constant and below `N` (`NK1298`), so the mask only keeps the
    /// lowering free of a check.
    #[inline(always)]
    pub fn shuffle(self, of: Fixed<u8, N>) -> Self {
        Fixed(std::array::from_fn(|i| self.0[of.0[i] as usize & (N - 1)]))
    }

    /// The number of lanes.
    #[inline(always)]
    pub fn count(self) -> i64 {
        N as i64
    }
}

macro_rules! shifts {
    ($($k:ty),*) => {
        $(
            impl<T: Lane, const N: usize> Shl<$k> for Fixed<T, N> {
                type Output = Self;
                #[inline(always)]
                fn shl(self, k: $k) -> Self {
                    Fixed(std::array::from_fn(|i| self.0[i].shl(k as u32)))
                }
            }
            impl<T: Lane, const N: usize> Shr<$k> for Fixed<T, N> {
                type Output = Self;
                #[inline(always)]
                fn shr(self, k: $k) -> Self {
                    Fixed(std::array::from_fn(|i| self.0[i].shr(k as u32)))
                }
            }
        )*
    };
}
shifts!(u32);

macro_rules! bits {
    ($ty:ident) => {
        impl<T: Lane, const N: usize> BitAnd for $ty<T, N> {
            type Output = Self;
            #[inline(always)]
            fn bitand(self, other: Self) -> Self {
                $ty(std::array::from_fn(|i| self.0[i].and(other.0[i])))
            }
        }
        impl<T: Lane, const N: usize> BitOr for $ty<T, N> {
            type Output = Self;
            #[inline(always)]
            fn bitor(self, other: Self) -> Self {
                $ty(std::array::from_fn(|i| self.0[i].or(other.0[i])))
            }
        }
        impl<T: Lane, const N: usize> BitXor for $ty<T, N> {
            type Output = Self;
            #[inline(always)]
            fn bitxor(self, other: Self) -> Self {
                $ty(std::array::from_fn(|i| self.0[i].xor(other.0[i])))
            }
        }
        impl<T: Lane, const N: usize> Not for $ty<T, N> {
            type Output = Self;
            #[inline(always)]
            fn not(self) -> Self {
                $ty(std::array::from_fn(|i| self.0[i].not()))
            }
        }
    };
}
bits!(Fixed);
bits!(Mask);

impl<T: Lane, const N: usize> Mask<T, N> {
    /// Whether lane `i` holds.
    #[inline(always)]
    fn holds(&self, i: usize) -> bool {
        self.0[i] != T::default()
    }

    /// The first lane that holds, or nothing.
    #[inline(always)]
    pub fn first(self) -> Option<i64> {
        (0..N).find(|&i| self.holds(i)).map(|i| i as i64)
    }

    /// The lanes that hold, rising.
    #[inline(always)]
    pub fn positions(self) -> impl Iterator<Item = i64> {
        (0..N).filter(move |&i| self.holds(i)).map(|i| i as i64)
    }

    /// How many lanes hold.
    #[inline(always)]
    pub fn count(self) -> i64 {
        (0..N).filter(|&i| self.holds(i)).count() as i64
    }

    /// Whether any lane holds.
    #[inline(always)]
    pub fn any(self) -> bool {
        (0..N).any(|i| self.holds(i))
    }

    /// Whether every lane holds.
    #[inline(always)]
    pub fn all(self) -> bool {
        (0..N).all(|i| self.holds(i))
    }
}

/// **`dot_pairs`, `widen` and `narrow_saturating` change the lane's width**
/// (D3), so each is written for every count a `Fixed` may have.
macro_rules! widths {
    (@pair $n:literal, $($narrow:ident $wide:ident),*) => {
        $(
            impl Fixed<$narrow, $n> {
                /// Each lane in a lane of twice the width.
                #[inline(always)]
                pub fn widen(self) -> Fixed<$wide, $n> {
                    Fixed(std::array::from_fn(|i| <$wide>::from(self.0[i])))
                }
            }
            impl Fixed<$wide, $n> {
                /// Each lane in a lane of half the width, the nearest value
                /// it holds where it does not fit.
                #[inline(always)]
                pub fn narrow_saturating(self) -> Fixed<$narrow, $n> {
                    Fixed(std::array::from_fn(|i| {
                        self.0[i].clamp(<$wide>::from(<$narrow>::MIN), <$wide>::from(<$narrow>::MAX)) as $narrow
                    }))
                }
            }
        )*
    };
    ($($n:literal / $half:literal),*) => {
        $(
            impl Fixed<u8, $n> {
                /// Each lane times `w`'s, each neighbouring pair added into
                /// an `i16`, saturating (`pmaddubsw`).
                #[inline(always)]
                pub fn dot_pairs(self, w: Fixed<i8, $n>) -> Fixed<i16, $half> {
                    Fixed(std::array::from_fn(|i| {
                        let a = i32::from(self.0[2 * i]) * i32::from(w.0[2 * i]);
                        let b = i32::from(self.0[2 * i + 1]) * i32::from(w.0[2 * i + 1]);
                        (a + b).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
                    }))
                }
            }
            impl Fixed<i16, $n> {
                /// Each lane times `w`'s, each neighbouring pair added into
                /// an `i32`, wrapping (`pmaddwd`).
                #[inline(always)]
                pub fn dot_pairs(self, w: Fixed<i16, $n>) -> Fixed<i32, $half> {
                    Fixed(std::array::from_fn(|i| {
                        let a = i32::from(self.0[2 * i]) * i32::from(w.0[2 * i]);
                        let b = i32::from(self.0[2 * i + 1]) * i32::from(w.0[2 * i + 1]);
                        a.wrapping_add(b)
                    }))
                }
            }
            widths!(@pair $n, u8 u16, i8 i16, u16 u32, i16 i32, u32 u64, i32 i64);
        )*
    };
}
widths!(2 / 1, 4 / 2, 8 / 4, 16 / 8, 32 / 16, 64 / 32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lanes_at_the_builds_width() {
        let v: Lanes<u8> = splat(1);
        assert_eq!(v.count() as usize, WIDTH);
        let w: Lanes<u64> = splat(1);
        assert_eq!(w.count() as usize, WIDTH / 8);
    }

    #[test]
    fn load_reads_least_significant_first_and_stops_past_the_end() {
        let b = [1u8, 0, 2, 0, 3, 0, 4, 0, 9];
        let v: Fixed<u16, 4> = load(&b, 0);
        assert_eq!(v, Fixed::from([1, 2, 3, 4]));
        assert!(std::panic::catch_unwind(|| load::<Fixed<u16, 4>>(&b, 2)).is_err());
    }

    #[test]
    fn lane_operations() {
        let a = Fixed::<u8, 4>::from([250, 1, 2, 3]);
        let b = splat::<Fixed<u8, 4>>(10);
        assert_eq!(a.wrapping_add(b), Fixed::from([4, 11, 12, 13]));
        assert_eq!(a.saturating_add(b), Fixed::from([255, 11, 12, 13]));
        assert_eq!(a.lt(b).positions().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(a.eq(b).first(), None);
        assert_eq!(select(a.gt(b), a, b), Fixed::from([250, 10, 10, 10]));
        assert_eq!((a << 1u32) >> 1u32, Fixed::from([122, 1, 2, 3]));
        assert_eq!(a.wrapping_sum(), 0);
        assert_eq!(a.max_lane(), 250);
        assert_eq!(
            a.shuffle(Fixed::from([3, 2, 1, 0])),
            Fixed::from([3, 2, 1, 250])
        );
    }

    #[test]
    fn dot_pairs_widen_narrow() {
        let d = Fixed::<u8, 4>::from([1, 2, 255, 255]).dot_pairs(Fixed::from([10, 1, 127, 127]));
        assert_eq!(d, Fixed::from([12, i16::MAX]));
        let e = Fixed::<i16, 2>::from([3, 4]).dot_pairs(Fixed::from([5, 6]));
        assert_eq!(e, Fixed::from([39]));
        let w = Fixed::<u8, 2>::from([1, 255]).widen();
        assert_eq!(w, Fixed::<u16, 2>::from([1, 255]));
        assert_eq!(
            Fixed::<i16, 2>::from([300, -300]).narrow_saturating(),
            Fixed::<i8, 2>::from([127, -128])
        );
    }
}
