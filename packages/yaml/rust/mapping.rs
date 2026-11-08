//! `Mapping` (ADR-yaml-1 D10): a YAML mapping, its keys in the order they
//! were read, as serde_yaml 0.9 keeps it.

use crate::value::Value;
use indexmap::IndexMap;
use indexmap::map::Entry;
use serde::{Deserialize, Deserializer};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

/// A YAML mapping in which the keys and the values are both `Value`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Mapping {
    map: IndexMap<Value, Value>,
}

impl Mapping {
    /// An empty mapping.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty mapping with room for `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Mapping {
            map: IndexMap::with_capacity(capacity),
        }
    }

    /// Inserts a key and its value; gives the old value of the key, which
    /// keeps its place.
    pub fn insert(&mut self, k: Value, v: Value) -> Option<Value> {
        self.map.insert(k, v)
    }

    /// True where the mapping holds the key.
    pub fn contains_key(&self, k: &Value) -> bool {
        self.map.contains_key(k)
    }

    /// The value of the key.
    pub fn get(&self, k: &Value) -> Option<&Value> {
        self.map.get(k)
    }

    /// The value of the key, to change.
    pub fn get_mut(&mut self, k: &Value) -> Option<&mut Value> {
        self.map.get_mut(k)
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True where the mapping has no entry.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The entries, in order.
    pub fn iter(&self) -> indexmap::map::Iter<'_, Value, Value> {
        self.map.iter()
    }

    /// The keys, in order.
    pub fn keys(&self) -> indexmap::map::Keys<'_, Value, Value> {
        self.map.keys()
    }

    /// The values, in order.
    pub fn values(&self) -> indexmap::map::Values<'_, Value, Value> {
        self.map.values()
    }
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for Mapping {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Not sensitive to the order of the entries, as `Eq` is not.
        let mut xor = 0;
        for (k, v) in self {
            let mut hasher = DefaultHasher::new();
            k.hash(&mut hasher);
            v.hash(&mut hasher);
            xor ^= hasher.finish();
        }
        xor.hash(state);
    }
}

impl PartialOrd for Mapping {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut self_entries = Vec::from_iter(self);
        let mut other_entries = Vec::from_iter(other);
        // No two keys of one mapping are equal, so sorting by key alone is a
        // total order here.
        let by_key = |&(a, _): &(&Value, &Value), &(b, _): &(&Value, &Value)| a.total_cmp(b);
        self_entries.sort_by(by_key);
        other_entries.sort_by(by_key);
        self_entries.partial_cmp(&other_entries)
    }
}

impl Debug for Mapping {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Mapping ")?;
        f.debug_map().entries(self.map.iter()).finish()
    }
}

impl<'a> IntoIterator for &'a Mapping {
    type Item = (&'a Value, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl IntoIterator for Mapping {
    type Item = (Value, Value);
    type IntoIter = indexmap::map::IntoIter<Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl FromIterator<(Value, Value)> for Mapping {
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> Self {
        Mapping {
            map: IndexMap::from_iter(iter),
        }
    }
}

impl Extend<(Value, Value)> for Mapping {
    fn extend<I: IntoIterator<Item = (Value, Value)>>(&mut self, iter: I) {
        self.map.extend(iter);
    }
}

impl<'de> Deserialize<'de> for Mapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Mapping;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a YAML mapping")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Mapping::new())
            }

            fn visit_map<A>(self, mut data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut mapping = Mapping::new();
                while let Some(key) = data.next_key()? {
                    match mapping.map.entry(key) {
                        Entry::Occupied(entry) => {
                            return Err(serde::de::Error::custom(DuplicateKey(entry.key())));
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(data.next_value()?);
                        }
                    }
                }
                Ok(mapping)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

struct DuplicateKey<'a>(&'a Value);

impl Display for DuplicateKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("duplicate entry ")?;
        match self.0 {
            Value::Null => f.write_str("with null key"),
            Value::Bool(boolean) => write!(f, "with key `{}`", boolean),
            Value::Number(number) => write!(f, "with key {}", number),
            Value::String(string) => write!(f, "with key {:?}", string),
            Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => f.write_str("in YAML map"),
        }
    }
}
