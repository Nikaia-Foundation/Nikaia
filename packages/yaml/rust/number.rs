//! `Number` (ADR-yaml-1 D10): a YAML integer or float, as serde_yaml 0.9
//! keeps it.

use crate::de::{digits_but_not_number, parse_f64, visit_int};
use crate::error::{self, Error, ErrorImpl};
use serde::de::{Deserialize, Deserializer, Visitor};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// A YAML number, an integer or a floating point one.
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Number {
    n: N,
}

#[derive(Copy, Clone)]
enum N {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    /// May be infinite or NaN.
    Float(f64),
}

impl Number {
    /// True for an integer between `i64::MIN` and `i64::MAX`.
    pub fn is_i64(&self) -> bool {
        match self.n {
            N::PosInt(v) => v <= i64::MAX as u64,
            N::NegInt(_) => true,
            N::Float(_) => false,
        }
    }

    /// True for an integer between 0 and `u64::MAX`.
    pub fn is_u64(&self) -> bool {
        matches!(self.n, N::PosInt(_))
    }

    /// True for a floating point number.
    pub fn is_f64(&self) -> bool {
        matches!(self.n, N::Float(_))
    }

    /// The number as an `i64`, where it is an integer that fits.
    pub fn as_i64(&self) -> Option<i64> {
        match self.n {
            N::PosInt(n) => i64::try_from(n).ok(),
            N::NegInt(n) => Some(n),
            N::Float(_) => None,
        }
    }

    /// The number as a `u64`, where it is an integer that fits.
    pub fn as_u64(&self) -> Option<u64> {
        match self.n {
            N::PosInt(n) => Some(n),
            N::NegInt(_) | N::Float(_) => None,
        }
    }

    /// The number as an `f64`, an integer converted.
    pub fn as_f64(&self) -> Option<f64> {
        match self.n {
            N::PosInt(n) => Some(n as f64),
            N::NegInt(n) => Some(n as f64),
            N::Float(n) => Some(n),
        }
    }

    /// True for `.nan`.
    pub fn is_nan(&self) -> bool {
        matches!(self.n, N::Float(f) if f.is_nan())
    }

    /// True for `.inf` and `-.inf`.
    pub fn is_infinite(&self) -> bool {
        matches!(self.n, N::Float(f) if f.is_infinite())
    }

    /// True for every integer and every float that is neither infinite nor
    /// NaN.
    pub fn is_finite(&self) -> bool {
        match self.n {
            N::PosInt(_) | N::NegInt(_) => true,
            N::Float(f) => f.is_finite(),
        }
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.n {
            N::PosInt(i) => Display::fmt(&i, f),
            N::NegInt(i) => Display::fmt(&i, f),
            N::Float(x) if x.is_nan() => f.write_str(".nan"),
            N::Float(x) if x.is_infinite() => f.write_str(if x.is_sign_negative() {
                "-.inf"
            } else {
                ".inf"
            }),
            // The shortest text that reads back as the same float, with a
            // `.0` on a whole number, as serde_yaml's `ryu` writes it.
            N::Float(x) => Debug::fmt(&x, f),
        }
    }
}

impl Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Number({})", self)
    }
}

impl FromStr for Number {
    type Err = Error;

    fn from_str(repr: &str) -> Result<Self, Self::Err> {
        if let Ok(result) = visit_int(NumberVisitor, repr) {
            return result;
        }
        if !digits_but_not_number(repr)
            && let Some(float) = parse_f64(repr)
        {
            return Ok(float.into());
        }
        Err(error::new(ErrorImpl::FailedToParseNumber))
    }
}

impl PartialEq for N {
    fn eq(&self, other: &N) -> bool {
        match (*self, *other) {
            (N::PosInt(a), N::PosInt(b)) => a == b,
            (N::NegInt(a), N::NegInt(b)) => a == b,
            // YAML has one NaN.
            (N::Float(a), N::Float(b)) => (a.is_nan() && b.is_nan()) || a == b,
            _ => false,
        }
    }
}

impl Eq for Number {}

impl PartialOrd for N {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (N::Float(a), N::Float(b)) if a.is_nan() && b.is_nan() => Some(Ordering::Equal),
            (N::Float(a), N::Float(b)) => a.partial_cmp(&b),
            _ => Some(self.total_cmp(other)),
        }
    }
}

impl N {
    fn total_cmp(&self, other: &Self) -> Ordering {
        match (*self, *other) {
            (N::PosInt(a), N::PosInt(b)) => a.cmp(&b),
            (N::NegInt(a), N::NegInt(b)) => a.cmp(&b),
            (N::NegInt(_), N::PosInt(_)) => Ordering::Less,
            (N::PosInt(_), N::NegInt(_)) => Ordering::Greater,
            (N::Float(a), N::Float(b)) => a.partial_cmp(&b).unwrap_or_else(|| {
                // NaN sorts last.
                if !a.is_nan() {
                    Ordering::Less
                } else if !b.is_nan() {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            }),
            // Integers sort below floats.
            (_, N::Float(_)) => Ordering::Less,
            (N::Float(_), _) => Ordering::Greater,
        }
    }
}

impl Number {
    pub(crate) fn total_cmp(&self, other: &Self) -> Ordering {
        self.n.total_cmp(&other.n)
    }
}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.n {
            // Every float hashes alike; equal floats are equal numbers.
            N::Float(_) => 3.hash(state),
            N::PosInt(u) => u.hash(state),
            N::NegInt(i) => i.hash(state),
        }
    }
}

macro_rules! from_signed {
    ($($ty:ty)*) => {$(
        impl From<$ty> for Number {
            fn from(i: $ty) -> Self {
                if i < 0 {
                    Number { n: N::NegInt(i as i64) }
                } else {
                    Number { n: N::PosInt(i as u64) }
                }
            }
        }
    )*};
}

macro_rules! from_unsigned {
    ($($ty:ty)*) => {$(
        impl From<$ty> for Number {
            fn from(u: $ty) -> Self {
                Number { n: N::PosInt(u as u64) }
            }
        }
    )*};
}

from_signed!(i8 i16 i32 i64 isize);
from_unsigned!(u8 u16 u32 u64 usize);

impl From<f32> for Number {
    fn from(f: f32) -> Self {
        Number::from(f as f64)
    }
}

impl From<f64> for Number {
    fn from(mut f: f64) -> Self {
        if f.is_nan() {
            // YAML has one NaN: no sign, no payload.
            f = f64::NAN.copysign(1.0);
        }
        Number { n: N::Float(f) }
    }
}

pub(crate) struct NumberVisitor;

impl Visitor<'_> for NumberVisitor {
    type Value = Number;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a number")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
        Ok(value.into())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
        Ok(value.into())
    }

    fn visit_f64<E>(self, value: f64) -> Result<Number, E> {
        Ok(value.into())
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Number, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(NumberVisitor)
    }
}
