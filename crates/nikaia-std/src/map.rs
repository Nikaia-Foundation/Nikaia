//! **A map: `std`'s own type over `hashbrown`'s table**
//! ([ADR-296](../../../docs/specification/adr/adr-296.md) D42).
//!
//! Every Nikaia map, keyed or trusted, is this one type. It is `hashbrown`'s
//! `HashTable` with the hash and the equality chosen here per key type, and
//! otherwise the same calls in the same order `hashbrown`'s `HashMap` makes, so
//! a map holds its entries where that map held them.
//!
//! **Text keys** (`&str`, `String`, `str` and the other types `text-key`
//! names) are read a word at a time through their pointer: the equality
//! compares the two keys' first and last words (or halves, or bytes, for a
//! key under 8 bytes) and never calls `memcmp`, which Rust's `str` `==`
//! always does; with the fast hasher the hash is the one [`FxHasher`] gives
//! the text, worked out inline for a key of up to 16 bytes. Which key type it
//! is, is known when compiling: `text-key` answers from two type ids, and the
//! optimiser keeps one branch. Every other key, and every key under the keyed
//! hasher, is hashed and compared by its own `Hash` and `Eq`.
//!
//! Keys are not copied into the slot: a view handed in is the view handed
//! back by `keys()` and `drain`.
//!
//! [`FxHasher`]: crate::hash::FxHasher

use std::borrow::Borrow;
use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::iter::FusedIterator;

use hashbrown::hash_table::{self, HashTable};

use crate::hash::{Fast, Keyed};

/// A table from keys to values, with the hasher `S`.
pub struct HashMap<K, V, S = Keyed> {
    table: HashTable<(K, V)>,
    hash_builder: S,
}

// ---------------------------------------------------------------------------
// The hash and the equality
// ---------------------------------------------------------------------------

/// The hash of a key, the one `hash_builder.hash_one(key)` gives, worked out
/// inline for a text key under the fast hasher.
#[inline(always)]
fn hash_of<Q: Hash + ?Sized, S: BuildHasher>(hash_builder: &S, key: &Q) -> u64 {
    if text_key::same::<S, Fast>()
        && let Some(bytes) = text_key::bytes(key)
    {
        return crate::hash::fx_text(bytes);
    }
    hash_builder.hash_one(key)
}

/// **Binds `$eq` to "this stored entry's key equals `$key`"** and evaluates
/// `$body` with it. A text key is compared a word at a time, and the query's
/// words are read once, before the search: the closure handed to the table is
/// then one compare of the length and two of words. Its length's class is
/// decided here, once per search, and each class gets a search of its own;
/// one closure deciding it per stored key made `HashTable::find` too big for
/// LLVM to inline, and a fold paid a call per row. Every other key is
/// compared by its own `==`.
macro_rules! with_key_eq {
    ($key:expr, |$eq:ident| $body:expr) => {{
        let query = $key;
        match text_key::bytes(query) {
            None => {
                let $eq = |(k, _): &(K, V)| query == k.borrow();
                $body
            }
            Some(q) => {
                let n = q.len();
                if n > 16 {
                    let $eq = |(k, _): &(K, V)| {
                        let k = stored(k.borrow());
                        k.len() == n && long_eq(q, k)
                    };
                    $body
                } else if n >= 8 {
                    let (first, last) = (word(q, 0), word(q, n - 8));
                    let $eq = |(k, _): &(K, V)| {
                        let k = stored(k.borrow());
                        k.len() == n && ((word(k, 0) ^ first) | (word(k, n - 8) ^ last)) == 0
                    };
                    $body
                } else if n >= 4 {
                    let (first, last) = (half(q, 0), half(q, n - 4));
                    let $eq = |(k, _): &(K, V)| {
                        let k = stored(k.borrow());
                        k.len() == n && ((half(k, 0) ^ first) | (half(k, n - 4) ^ last)) == 0
                    };
                    $body
                } else if n > 0 {
                    // One to three bytes: the first, the middle and the last
                    // cover them all.
                    let (first, middle, last) = (q[0], q[n / 2], q[n - 1]);
                    let $eq = |(k, _): &(K, V)| {
                        let k = stored(k.borrow());
                        k.len() == n
                            && ((k[0] ^ first) | (k[n / 2] ^ middle) | (k[n - 1] ^ last)) == 0
                    };
                    $body
                } else {
                    let $eq = |(k, _): &(K, V)| stored(k.borrow()).is_empty();
                    $body
                }
            }
        }
    }};
}

/// A stored key's bytes, where the query's type is text and so is it: `K`
/// borrows as the query's type.
#[inline(always)]
fn stored<Q: ?Sized>(key: &Q) -> &[u8] {
    text_key::bytes(key).unwrap_or_default()
}

#[inline(always)]
fn word(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("eight"))
}

#[inline(always)]
fn half(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four"))
}

/// More than 16 bytes of equal length: word by word, the last word
/// overlapping the one before.
#[inline(never)]
fn long_eq(a: &[u8], b: &[u8]) -> bool {
    let n = a.len();
    let mut at = 0;
    while at + 8 < n {
        if word(a, at) != word(b, at) {
            return false;
        }
        at += 8;
    }
    word(a, n - 8) == word(b, n - 8)
}

// ---------------------------------------------------------------------------
// The map
// ---------------------------------------------------------------------------

impl<K, V, S> HashMap<K, V, S> {
    /// An empty map with the hasher given.
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self {
            table: HashTable::new(),
            hash_builder,
        }
    }

    /// An empty map with room for `capacity` entries.
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            table: HashTable::with_capacity(capacity),
            hash_builder,
        }
    }

    /// The hasher.
    #[inline]
    pub fn hasher(&self) -> &S {
        &self.hash_builder
    }

    /// How many entries fit without growing.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    /// The number of entries.
    #[inline]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Whether there are none.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Every entry, in the table's order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.table.iter(),
        }
    }

    /// Every entry, its value changeable.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            inner: self.table.iter_mut(),
        }
    }

    /// Every key.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            inner: self.table.iter(),
        }
    }

    /// Every value.
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        Values {
            inner: self.table.iter(),
        }
    }

    /// Every value, changeable.
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            inner: self.table.iter_mut(),
        }
    }

    /// The keys, the map consumed.
    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys {
            inner: self.table.into_iter(),
        }
    }

    /// The values, the map consumed.
    #[inline]
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues {
            inner: self.table.into_iter(),
        }
    }

    /// Every entry taken out; the map is left empty.
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        Drain {
            inner: self.table.drain(),
        }
    }

    /// Keeps the entries `f` says yes to.
    #[inline]
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        self.table.retain(|(k, v)| f(k, v));
    }

    /// Removes every entry, keeping the room.
    #[inline]
    pub fn clear(&mut self) {
        self.table.clear();
    }
}

impl<K, V, S: Default> HashMap<K, V, S> {
    /// An empty map; nothing is allocated until the first entry.
    #[inline]
    pub fn new() -> Self {
        Self::with_hasher(S::default())
    }

    /// An empty map with room for `capacity` entries.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, S::default())
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S> {
    /// Room for `additional` more entries.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let hash_builder = &self.hash_builder;
        self.table
            .reserve(additional, |(k, _)| hash_of(hash_builder, k));
    }

    /// As little room as the entries need.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        let hash_builder = &self.hash_builder;
        self.table.shrink_to_fit(|(k, _)| hash_of(hash_builder, k));
    }

    /// The place `key` names, filled or not.
    ///
    /// **Always inlined**: a fold's step is this call and little else
    /// (ADR-296 D41), and returned through memory the place costs more than
    /// the search.
    #[inline(always)]
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, S> {
        let hash = hash_of(&self.hash_builder, &key);
        let hash_builder = &self.hash_builder;
        let table = &mut self.table;
        with_key_eq!(&key, |eq| match table.find_entry(hash, eq) {
            Ok(elem) => Entry::Occupied(OccupiedEntry { elem }),
            Err(absent) => Entry::Vacant(VacantEntry {
                hash,
                key,
                table: absent.into_table(),
                hash_builder
            }),
        })
    }

    #[inline]
    fn find<Q>(&self, key: &Q) -> Option<&(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let hash = hash_of(&self.hash_builder, key);
        with_key_eq!(key, |eq| self.table.find(hash, eq))
    }

    #[inline]
    fn find_mut<Q>(&mut self, key: &Q) -> Option<&mut (K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let hash = hash_of(&self.hash_builder, key);
        let table = &mut self.table;
        with_key_eq!(key, |eq| table.find_mut(hash, eq))
    }

    /// The value at `key`.
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.find(key).map(|(_, v)| v)
    }

    /// The key as stored and the value at `key`.
    #[inline]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.find(key).map(|(k, v)| (k, v))
    }

    /// The value at `key`, changeable.
    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.find_mut(key).map(|(_, v)| v)
    }

    /// Whether `key` is there.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.find(key).is_some()
    }

    /// Puts `value` at `key`, handing back the value that was there.
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash_builder = &self.hash_builder;
        let hash = hash_of(hash_builder, &key);
        let table = &mut self.table;
        let place = with_key_eq!(&key, |eq| match table
            .entry(hash, eq, |(k, _)| hash_of(hash_builder, k))
        {
            hash_table::Entry::Occupied(found) => Ok(found),
            hash_table::Entry::Vacant(empty) => Err(empty),
        });
        match place {
            Ok(mut found) => Some(std::mem::replace(&mut found.get_mut().1, value)),
            Err(empty) => {
                empty.insert((key, value));
                None
            }
        }
    }

    /// Takes the value at `key` out.
    #[inline]
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }

    /// Takes the key and the value at `key` out.
    #[inline]
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let hash = hash_of(&self.hash_builder, key);
        let table = &mut self.table;
        with_key_eq!(key, |eq| table
            .find_entry(hash, eq)
            .ok()
            .map(|found| found.remove().0))
    }
}

impl<K, V, S: Default> Default for HashMap<K, V, S> {
    #[inline]
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for HashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            hash_builder: self.hash_builder.clone(),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for HashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for HashMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for HashMap<K, V, S> {}

impl<K, Q, V, S> std::ops::Index<&Q> for HashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    type Output = V;

    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for HashMap<K, V, S> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut map = Self::with_capacity_and_hasher(iter.size_hint().0, S::default());
        iter.for_each(|(k, v)| {
            map.insert(k, v);
        });
        map
    }
}

impl<K: Eq + Hash, V, S: BuildHasher + Default, const N: usize> From<[(K, V); N]>
    for HashMap<K, V, S>
{
    fn from(entries: [(K, V); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for HashMap<K, V, S> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        // As `hashbrown` reserves: the whole hint into an empty map, half of
        // it otherwise, since keys may already be there.
        let iter = iter.into_iter();
        let reserve = if self.is_empty() {
            iter.size_hint().0
        } else {
            iter.size_hint().0.div_ceil(2)
        };
        self.reserve(reserve);
        iter.for_each(move |(k, v)| {
            self.insert(k, v);
        });
    }
}

impl<'a, K: Eq + Hash + Copy, V: Copy, S: BuildHasher> Extend<(&'a K, &'a V)> for HashMap<K, V, S> {
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        self.extend(iter.into_iter().map(|(&k, &v)| (k, v)));
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    #[inline]
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter {
            inner: self.table.into_iter(),
        }
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

// ---------------------------------------------------------------------------
// The entry API
// ---------------------------------------------------------------------------

/// The place in a [`HashMap`] a key names, filled or not.
pub enum Entry<'a, K, V, S> {
    /// The key is there.
    Occupied(OccupiedEntry<'a, K, V>),
    /// It is not.
    Vacant(VacantEntry<'a, K, V, S>),
}

/// A place that holds an entry.
pub struct OccupiedEntry<'a, K, V> {
    elem: hash_table::OccupiedEntry<'a, (K, V)>,
}

/// A place a key names that holds nothing yet.
pub struct VacantEntry<'a, K, V, S> {
    hash: u64,
    key: K,
    table: &'a mut HashTable<(K, V)>,
    hash_builder: &'a S,
}

impl<'a, K: Hash, V, S: BuildHasher> Entry<'a, K, V, S> {
    /// The value there, or `default` put there first.
    #[inline]
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(found) => found.into_mut(),
            Entry::Vacant(empty) => empty.insert(default),
        }
    }

    /// The value there, or what `default` makes put there first.
    #[inline]
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(found) => found.into_mut(),
            Entry::Vacant(empty) => empty.insert(default()),
        }
    }

    /// The value there, or what `default` makes of the key put there first.
    #[inline]
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(found) => found.into_mut(),
            Entry::Vacant(empty) => {
                let value = default(&empty.key);
                empty.insert(value)
            }
        }
    }

    /// The value there, or the default value put there first.
    #[inline]
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }

    /// Changes the value where there is one.
    #[inline]
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self {
        match self {
            Entry::Occupied(mut found) => {
                f(found.get_mut());
                Entry::Occupied(found)
            }
            vacant => vacant,
        }
    }

    /// Puts `value` there, filled or not.
    #[inline]
    pub fn insert(self, value: V) -> OccupiedEntry<'a, K, V> {
        match self {
            Entry::Occupied(mut found) => {
                found.insert(value);
                found
            }
            Entry::Vacant(empty) => empty.insert_entry(value),
        }
    }

    /// The key this place is for.
    #[inline]
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(found) => found.key(),
            Entry::Vacant(empty) => empty.key(),
        }
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// The key as stored.
    #[inline]
    pub fn key(&self) -> &K {
        &self.elem.get().0
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> &V {
        &self.elem.get().1
    }

    /// The value, changeable.
    #[inline]
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.elem.get_mut().1
    }

    /// The value, changeable for as long as the map is borrowed.
    #[inline]
    pub fn into_mut(self) -> &'a mut V {
        &mut self.elem.into_mut().1
    }

    /// Puts `value` there, handing back the one that was.
    #[inline]
    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }

    /// Takes the value out.
    #[inline]
    pub fn remove(self) -> V {
        self.remove_entry().1
    }

    /// Takes the key and the value out.
    #[inline]
    pub fn remove_entry(self) -> (K, V) {
        self.elem.remove().0
    }
}

impl<'a, K: Hash, V, S: BuildHasher> VacantEntry<'a, K, V, S> {
    /// The key this place is for.
    #[inline]
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The key, the place given up.
    #[inline]
    pub fn into_key(self) -> K {
        self.key
    }

    /// Puts `value` there.
    #[inline]
    pub fn insert(self, value: V) -> &'a mut V {
        self.insert_entry(value).into_mut()
    }

    /// Puts `value` there, handing back the filled place.
    #[inline]
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let hash_builder = self.hash_builder;
        let elem = self
            .table
            .insert_unique(self.hash, (self.key, value), |(k, _)| {
                hash_of(hash_builder, k)
            });
        OccupiedEntry { elem }
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for Entry<'_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Occupied(found) => f.debug_tuple("Entry").field(found).finish(),
            Entry::Vacant(empty) => f.debug_tuple("Entry").field(empty).finish(),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedEntry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}

impl<K: fmt::Debug, V, S> fmt::Debug for VacantEntry<'_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(&self.key).finish()
    }
}

// ---------------------------------------------------------------------------
// The iterators
// ---------------------------------------------------------------------------

macro_rules! iterator {
    ($(#[$doc:meta])* $name:ident<$($lt:lifetime,)? K, V>, $inner:ty, $item:ty, |$e:pat_param| $out:expr) => {
        $(#[$doc])*
        pub struct $name<$($lt,)? K, V> {
            inner: $inner,
        }

        impl<$($lt,)? K, V> Iterator for $name<$($lt,)? K, V> {
            type Item = $item;

            #[inline]
            fn next(&mut self) -> Option<$item> {
                self.inner.next().map(|$e| $out)
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
        }

        impl<$($lt,)? K, V> ExactSizeIterator for $name<$($lt,)? K, V> {
            #[inline]
            fn len(&self) -> usize {
                self.inner.len()
            }
        }

        impl<$($lt,)? K, V> FusedIterator for $name<$($lt,)? K, V> {}
    };
}

iterator!(
    /// The entries of a map, borrowed.
    Iter<'a, K, V>, hash_table::Iter<'a, (K, V)>, (&'a K, &'a V), |(k, v)| (k, v)
);
iterator!(
    /// The entries of a map, the values changeable.
    IterMut<'a, K, V>, hash_table::IterMut<'a, (K, V)>, (&'a K, &'a mut V), |(k, v)| (&*k, v)
);
iterator!(
    /// The keys of a map, borrowed.
    Keys<'a, K, V>, hash_table::Iter<'a, (K, V)>, &'a K, |(k, _)| k
);
iterator!(
    /// The values of a map, borrowed.
    Values<'a, K, V>, hash_table::Iter<'a, (K, V)>, &'a V, |(_, v)| v
);
iterator!(
    /// The values of a map, changeable.
    ValuesMut<'a, K, V>, hash_table::IterMut<'a, (K, V)>, &'a mut V, |(_, v)| v
);
iterator!(
    /// The entries of a map, the map consumed.
    IntoIter<K, V>, hash_table::IntoIter<(K, V)>, (K, V), |e| e
);
iterator!(
    /// The keys of a map, the map consumed.
    IntoKeys<K, V>, hash_table::IntoIter<(K, V)>, K, |(k, _)| k
);
iterator!(
    /// The values of a map, the map consumed.
    IntoValues<K, V>, hash_table::IntoIter<(K, V)>, V, |(_, v)| v
);
iterator!(
    /// The entries taken out of a map, which is left empty.
    Drain<'a, K, V>, hash_table::Drain<'a, (K, V)>, (K, V), |e| e
);

impl<K, V> Clone for Iter<'_, K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> Clone for Keys<'_, K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> Clone for Values<'_, K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::TrustedMap;

    fn trusted() -> TrustedMap<&'static str, i64> {
        TrustedMap::default()
    }

    /// The equality by words agrees with `==` at every length and every place
    /// a difference can be: the first byte, the middle, the last, and past 16.
    #[test]
    fn a_lookup_tells_every_byte() {
        for len in 0..48usize {
            let a: String = (0..len)
                .map(|i| (b'a' + (i * 7 % 26) as u8) as char)
                .collect();
            let mut m: TrustedMap<&str, usize> = TrustedMap::default();
            m.insert(a.as_str(), len);
            let mut keyed: HashMap<String, usize> = HashMap::default();
            keyed.insert(a.clone(), len);
            assert_eq!(m.get(a.as_str()), Some(&len), "length {len}");
            assert_eq!(keyed.get(a.as_str()), Some(&len), "length {len}");
            for at in 0..len {
                let mut b = a.clone().into_bytes();
                b[at] ^= 0x20;
                let b = String::from_utf8(b).expect("ASCII");
                assert_eq!(m.get(b.as_str()), None, "length {len}, byte {at}");
                assert!(!m.contains_key(b.as_str()), "length {len}, byte {at}");
                assert_eq!(keyed.get(b.as_str()), None, "length {len}, byte {at}");
            }
            let longer = format!("{a}a");
            assert_eq!(m.get(longer.as_str()), None, "length {len}");
        }
    }

    /// A text key hashes to what the fast hasher gives the text, so a map of
    /// `&str` and one of `String`, and a lookup by `str`, agree.
    #[test]
    fn a_text_key_hashes_as_its_text() {
        let fast = Fast::default();
        for len in 0..40usize {
            let s: String = (0..len).map(|i| (b'a' + (i % 26) as u8) as char).collect();
            assert_eq!(
                hash_of(&fast, s.as_str()),
                fast.hash_one(s.as_str()),
                "length {len}"
            );
            assert_eq!(
                hash_of(&fast, &s.as_str()),
                fast.hash_one(s.as_str()),
                "length {len}"
            );
            assert_eq!(
                hash_of(&fast, &s),
                fast.hash_one(s.as_str()),
                "length {len}"
            );
        }
    }

    #[test]
    fn the_empty_key_is_a_key() {
        let mut m = trusted();
        m.insert("", 1);
        m.insert("a", 2);
        assert_eq!(m.get(""), Some(&1));
        assert_eq!(m.get("a"), Some(&2));
        assert_eq!(m.len(), 2);
    }

    /// Keys longer than 16 bytes that differ only past the 16th, and ones of
    /// one length that differ only in the middle, stay apart.
    #[test]
    fn keys_differing_late_stay_apart() {
        let base = "Station-Name-1234567890-";
        let keys: Vec<String> = (0..64).map(|i| format!("{base}{i:02}")).collect();
        let mut m: TrustedMap<&str, usize> = TrustedMap::default();
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(m.insert(k.as_str(), i), None);
        }
        assert_eq!(m.len(), 64);
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(m.get(k.as_str()), Some(&i));
        }
        assert_eq!(m.get("Station-Name-1234567890-64"), None);
        let mut short: TrustedMap<String, usize> = TrustedMap::default();
        for i in 0..100 {
            *short.entry(format!("ab{i:02}cd")).or_insert(0) += i;
        }
        assert_eq!(short.len(), 100);
        assert_eq!(short.get("ab42cd"), Some(&42));
    }

    /// Keys that share a hash are told apart by their equality. Under the
    /// keyed hasher with a constant hash every key collides.
    #[test]
    fn colliding_keys_are_told_apart() {
        #[derive(Default, Clone)]
        struct Zero;
        struct ZeroHasher;
        impl std::hash::Hasher for ZeroHasher {
            fn finish(&self) -> u64 {
                0
            }
            fn write(&mut self, _: &[u8]) {}
        }
        impl BuildHasher for Zero {
            type Hasher = ZeroHasher;
            fn build_hasher(&self) -> ZeroHasher {
                ZeroHasher
            }
        }
        let mut m: HashMap<&str, i32, Zero> = HashMap::default();
        for (i, k) in [
            "a",
            "b",
            "ab",
            "ba",
            "",
            "abcdefgh",
            "abcdefgi",
            "abcdefghijklmnopq",
        ]
        .iter()
        .enumerate()
        {
            m.insert(k, i as i32);
        }
        assert_eq!(m.len(), 8);
        assert_eq!(m.get("abcdefgi"), Some(&6));
        assert_eq!(m.get("abcdefghijklmnopq"), Some(&7));
        assert_eq!(m.remove("ab"), Some(2));
        assert_eq!(m.get("ab"), None);
        assert_eq!(m.len(), 7);
    }

    /// The entry API, `drain` and `retain` as `hashbrown`'s map has them.
    #[test]
    fn entries_drain_and_retain() {
        let mut m = trusted();
        for word in "a b a c b a".split(' ') {
            m.entry(word).and_modify(|n| *n += 1).or_insert(1);
        }
        assert_eq!(m.get("a"), Some(&3));
        assert_eq!(m["b"], 2);
        m.retain(|_, n| *n > 1);
        assert_eq!(m.len(), 2);
        let mut drained: Vec<(&str, i64)> = m.drain().collect();
        drained.sort();
        assert_eq!(drained, vec![("a", 3), ("b", 2)]);
        assert!(m.is_empty());
    }

    /// Other keys go by their own `Eq`, under either hasher.
    #[test]
    fn other_keys_and_the_keyed_hasher() {
        let mut m: HashMap<(i64, char), i64> = HashMap::default();
        m.insert((1, 'a'), 1);
        m.insert((1, 'b'), 2);
        assert_eq!(m.get(&(1, 'b')), Some(&2));
        let mut k: HashMap<String, i64> = HashMap::default();
        k.insert("Hamburg".to_string(), 1);
        assert_eq!(k.get("Hamburg"), Some(&1));
        assert_eq!(k, k.clone());
    }

    /// Same calls, same table: a trusted map holds its keys in the order
    /// `hashbrown`'s map with the same hasher holds them.
    #[test]
    fn the_order_is_hashbrowns() {
        let keys: Vec<String> = (0..500).map(|i| format!("k{}", i * 7919 % 1000)).collect();
        let mut ours: TrustedMap<&str, usize> = TrustedMap::default();
        let mut theirs: hashbrown::HashMap<&str, usize, Fast> = hashbrown::HashMap::default();
        for (i, k) in keys.iter().enumerate() {
            ours.entry(k.as_str()).or_insert(i);
            theirs.entry(k.as_str()).or_insert(i);
            if i % 3 == 0 {
                ours.insert(k.as_str(), i);
                theirs.insert(k.as_str(), i);
            }
            if i % 5 == 0 {
                ours.remove(keys[i / 2].as_str());
                theirs.remove(keys[i / 2].as_str());
            }
        }
        let a: Vec<_> = ours.iter().collect();
        let b: Vec<_> = theirs.iter().collect();
        assert_eq!(a, b);
    }
}
