//! `Value` (ADR-yaml-1 D10): any YAML value, as serde_yaml 0.9 keeps it.

use crate::mapping::Mapping;
use crate::number::Number;
use serde::de::{self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, VariantAccess};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::mem;

/// Any YAML value.
#[derive(Clone, PartialEq, PartialOrd, Default)]
pub enum Value {
    /// `null`, `~` or nothing.
    #[default]
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer or a floating point number.
    Number(Number),
    /// A string.
    String(String),
    /// A sequence.
    Sequence(Sequence),
    /// A mapping.
    Mapping(Mapping),
    /// A value with a tag, like `!Thing 1`.
    Tagged(Box<TaggedValue>),
}

/// A YAML sequence.
pub type Sequence = Vec<Value>;

/// A tag, like `!Thing`; equal with or without its leading `!`.
#[derive(Clone)]
pub struct Tag {
    string: String,
}

/// A value and its tag.
#[derive(Clone, PartialEq, PartialOrd, Hash, Debug)]
pub struct TaggedValue {
    /// The tag.
    pub tag: Tag,
    /// The value.
    pub value: Value,
}

impl Tag {
    /// A tag; the leading `!` may be left out.
    ///
    /// # Panics
    ///
    /// On an empty string.
    pub fn new(string: impl Into<String>) -> Self {
        let tag: String = string.into();
        assert!(!tag.is_empty(), "empty YAML tag is not allowed");
        Tag { string: tag }
    }
}

fn nobang(maybe_banged: &str) -> &str {
    match maybe_banged.strip_prefix('!') {
        Some("") | None => maybe_banged,
        Some(unbanged) => unbanged,
    }
}

impl Eq for Tag {}

impl PartialEq for Tag {
    fn eq(&self, other: &Tag) -> bool {
        nobang(&self.string) == nobang(&other.string)
    }
}

impl<T: ?Sized + AsRef<str>> PartialEq<T> for Tag {
    fn eq(&self, other: &T) -> bool {
        nobang(&self.string) == nobang(other.as_ref())
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        nobang(&self.string).cmp(nobang(&other.string))
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        nobang(&self.string).hash(hasher);
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "!{}", nobang(&self.string))
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Value {
    /// The value under its tags.
    pub fn untag(self) -> Self {
        let mut cur = self;
        while let Value::Tagged(tagged) = cur {
            cur = tagged.value;
        }
        cur
    }

    /// The value under its tags, borrowed.
    pub fn untag_ref(&self) -> &Self {
        let mut cur = self;
        while let Value::Tagged(tagged) = cur {
            cur = &tagged.value;
        }
        cur
    }

    /// True for `Null`.
    pub fn is_null(&self) -> bool {
        matches!(self.untag_ref(), Value::Null)
    }

    /// The boolean, where the value is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self.untag_ref() {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The number as an `i64`, where it is an integer that fits.
    pub fn as_i64(&self) -> Option<i64> {
        match self.untag_ref() {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// The number as a `u64`, where it is an integer that fits.
    pub fn as_u64(&self) -> Option<u64> {
        match self.untag_ref() {
            Value::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    /// The number as an `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        match self.untag_ref() {
            Value::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    /// The string, where the value is one.
    pub fn as_str(&self) -> Option<&str> {
        match self.untag_ref() {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The sequence, where the value is one.
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self.untag_ref() {
            Value::Sequence(seq) => Some(seq),
            _ => None,
        }
    }

    /// The mapping, where the value is one.
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self.untag_ref() {
            Value::Mapping(map) => Some(map),
            _ => None,
        }
    }

    /// An order over every value, NaN included, consistent with
    /// `PartialOrd` where that gives one.
    pub(crate) fn total_cmp(&self, other: &Self) -> Ordering {
        fn seq_cmp<T>(
            mut a: impl Iterator<Item = T>,
            mut b: impl Iterator<Item = T>,
            cmp: impl Fn(T, T) -> Ordering,
        ) -> Ordering {
            loop {
                match (a.next(), b.next()) {
                    (None, None) => return Ordering::Equal,
                    (None, Some(_)) => return Ordering::Less,
                    (Some(_), None) => return Ordering::Greater,
                    (Some(x), Some(y)) => match cmp(x, y) {
                        Ordering::Equal => {}
                        other => return other,
                    },
                }
            }
        }
        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Bool(_), _) => Ordering::Less,
            (_, Value::Bool(_)) => Ordering::Greater,
            (Value::Number(a), Value::Number(b)) => a.total_cmp(b),
            (Value::Number(_), _) => Ordering::Less,
            (_, Value::Number(_)) => Ordering::Greater,
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::String(_), _) => Ordering::Less,
            (_, Value::String(_)) => Ordering::Greater,
            (Value::Sequence(a), Value::Sequence(b)) => {
                seq_cmp(a.iter(), b.iter(), |x, y| x.total_cmp(y))
            }
            (Value::Sequence(_), _) => Ordering::Less,
            (_, Value::Sequence(_)) => Ordering::Greater,
            (Value::Mapping(a), Value::Mapping(b)) => seq_cmp(a.iter(), b.iter(), |x, y| {
                x.0.total_cmp(y.0).then_with(|| x.1.total_cmp(y.1))
            }),
            (Value::Mapping(_), _) => Ordering::Less,
            (_, Value::Mapping(_)) => Ordering::Greater,
            (Value::Tagged(a), Value::Tagged(b)) => {
                a.tag.cmp(&b.tag).then_with(|| a.value.total_cmp(&b.value))
            }
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(v) => v.hash(state),
            Value::Number(v) => v.hash(state),
            Value::String(v) => v.hash(state),
            Value::Sequence(v) => v.hash(state),
            Value::Mapping(v) => v.hash(state),
            Value::Tagged(v) => v.hash(state),
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Null => f.write_str("Null"),
            Value::Bool(boolean) => write!(f, "Bool({})", boolean),
            Value::Number(number) => write!(f, "Number({})", number),
            Value::String(string) => write!(f, "String({:?})", string),
            Value::Sequence(sequence) => {
                f.write_str("Sequence ")?;
                f.debug_list().entries(sequence).finish()
            }
            Value::Mapping(mapping) => Debug::fmt(mapping, f),
            Value::Tagged(tagged) => Debug::fmt(tagged, f),
        }
    }
}

macro_rules! from_number {
    ($($ty:ty)*) => {$(
        impl From<$ty> for Value {
            fn from(n: $ty) -> Self {
                Value::Number(n.into())
            }
        }
    )*};
}

from_number!(i8 i16 i32 i64 isize u8 u16 u32 u64 usize f32 f64);

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_owned())
    }
}

impl From<Number> for Value {
    fn from(n: Number) -> Self {
        Value::Number(n)
    }
}

impl From<Mapping> for Value {
    fn from(m: Mapping) -> Self {
        Value::Mapping(m)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Sequence(v.into_iter().map(Into::into).collect())
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any YAML value")
            }

            fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
                Ok(Value::Bool(b))
            }

            fn visit_i64<E>(self, i: i64) -> Result<Value, E> {
                Ok(Value::Number(i.into()))
            }

            fn visit_u64<E>(self, u: u64) -> Result<Value, E> {
                Ok(Value::Number(u.into()))
            }

            fn visit_f64<E>(self, f: f64) -> Result<Value, E> {
                Ok(Value::Number(f.into()))
            }

            fn visit_str<E>(self, s: &str) -> Result<Value, E> {
                Ok(Value::String(s.to_owned()))
            }

            fn visit_string<E>(self, s: String) -> Result<Value, E> {
                Ok(Value::String(s))
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A>(self, data: A) -> Result<Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let de = de::value::SeqAccessDeserializer::new(data);
                Ok(Value::Sequence(Sequence::deserialize(de)?))
            }

            fn visit_map<A>(self, data: A) -> Result<Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let de = de::value::MapAccessDeserializer::new(data);
                Ok(Value::Mapping(Mapping::deserialize(de)?))
            }

            fn visit_enum<A>(self, data: A) -> Result<Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let (tag, contents) = data.variant_seed(TagStringVisitor)?;
                let value = contents.newtype_variant()?;
                Ok(Value::Tagged(Box::new(TaggedValue { tag, value })))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Reads a tag from the name of an enum's variant.
struct TagStringVisitor;

impl de::Visitor<'_> for TagStringVisitor {
    type Value = Tag;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a YAML tag string")
    }

    fn visit_str<E: de::Error>(self, string: &str) -> Result<Tag, E> {
        self.visit_string(string.to_owned())
    }

    fn visit_string<E: de::Error>(self, string: String) -> Result<Tag, E> {
        if string.is_empty() {
            return Err(E::custom("empty YAML tag is not allowed"));
        }
        Ok(Tag::new(string))
    }
}

impl<'de> DeserializeSeed<'de> for TagStringVisitor {
    type Value = Tag;

    fn deserialize<D>(self, deserializer: D) -> Result<Tag, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(self)
    }
}
