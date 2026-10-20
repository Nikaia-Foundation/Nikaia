//! **A shared, immutable run of bytes with a counted handle on what owns it**
//! (Nikaia's ADR-373 D1).
//!
//! A [`Shared`] is where its bytes are and how many there are - a pointer and
//! a length, read without touching anything else - and a counted handle on
//! whatever owns that memory: a `Vec<u8>` handed over, a mapped file, any
//! value that lends its bytes through `AsRef<[u8]>`. Static data needs no
//! handle at all. Cloning is a count, a cut ([`Shared::cut`]) shares the
//! owner, and the owner is dropped when the last holder goes.
//!
//! **Why a trait object for the owner, and not an enum of the owners there
//! are:** reads never look at the owner - the pointer and the length are in
//! the value - so the one thing the owner's type decides is what a clone and a
//! drop cost, and an `Arc` of a trait object costs a count for each, whatever
//! owns the bytes. An enum would make this crate name every owner (a mapping
//! of the crate that maps files among them) and add a branch to each clone.
//!
//! Every `unsafe` in this crate, and the argument for it, is listed in
//! `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ops::{Bound, Deref, RangeBounds};
use std::ptr::NonNull;
use std::sync::Arc;

/// What owns the memory of a [`Shared`]: anything that lends its bytes.
type Owner = Arc<dyn AsRef<[u8]> + Send + Sync>;

/// A shared, immutable run of bytes and a counted handle on its owner.
pub struct Shared {
    ptr: NonNull<u8>,
    len: usize,
    /// `None` for static data, which nothing owns.
    owner: Option<Owner>,
}

// SAFETY: see README.md - the pointer is only read, and the owner is
// `Send + Sync` by its type.
unsafe impl Send for Shared {}
// SAFETY: as for `Send`: shared access only ever reads the bytes.
unsafe impl Sync for Shared {}

impl Shared {
    /// Static bytes: no owner, no count.
    pub const fn from_static(bytes: &'static [u8]) -> Shared {
        Shared {
            // A slice's pointer is never null, also where it is empty.
            ptr: match NonNull::new(bytes.as_ptr().cast_mut()) {
                Some(ptr) => ptr,
                None => NonNull::dangling(),
            },
            len: bytes.len(),
            owner: None,
        }
    }

    /// The bytes `owner` lends, owned from now on by the `Shared` values made
    /// from it: nothing is copied.
    pub fn from_owner<O: AsRef<[u8]> + Send + Sync + 'static>(owner: O) -> Shared {
        Shared::from_arc(Arc::new(owner))
    }

    /// The bytes an owner already counted lends, at one more count: what a
    /// value that holds its owner in an `Arc` (a mapped file) hands over.
    pub fn from_arc<O: AsRef<[u8]> + Send + Sync + 'static>(owner: Arc<O>) -> Shared {
        let owner: Owner = owner;
        let bytes: &[u8] = (*owner).as_ref();
        Shared {
            ptr: NonNull::from(bytes).cast(),
            len: bytes.len(),
            owner: Some(owner),
        }
    }

    /// The bytes.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: README.md, `as_slice`.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// How many bytes there are.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are none.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The bytes `range` names, sharing this owner: it stays alive as long as
    /// the cut does. Stops the program as a slice's index does where the
    /// range is not inside.
    #[track_caller]
    pub fn cut(&self, range: impl RangeBounds<usize>) -> Shared {
        let start = match range.start_bound() {
            Bound::Included(&s) => s,
            Bound::Excluded(&s) => s.checked_add(1).expect("a cut that starts past usize::MAX"),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&e) => e.checked_add(1).expect("a cut that ends past usize::MAX"),
            Bound::Excluded(&e) => e,
            Bound::Unbounded => self.len,
        };
        // The slice's own check, with its own message.
        let part = &self.as_slice()[start..end];
        Shared {
            ptr: NonNull::from(part).cast(),
            len: part.len(),
            owner: self.owner.clone(),
        }
    }

    /// Whether the two share one owner (both static counts as not).
    pub fn same_owner(&self, other: &Shared) -> bool {
        match (&self.owner, &other.owner) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Clone for Shared {
    #[inline]
    fn clone(&self) -> Shared {
        Shared {
            ptr: self.ptr,
            len: self.len,
            owner: self.owner.clone(),
        }
    }
}

impl Default for Shared {
    fn default() -> Shared {
        Shared::from_static(&[])
    }
}

impl Deref for Shared {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for Shared {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

/// Equal by content, whoever owns it.
impl PartialEq for Shared {
    fn eq(&self, other: &Shared) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for Shared {}

impl PartialOrd for Shared {
    fn partial_cmp(&self, other: &Shared) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Shared {
    fn cmp(&self, other: &Shared) -> std::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl std::hash::Hash for Shared {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::Shared;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// An owner that counts its drops.
    struct Counted(Vec<u8>, Arc<AtomicUsize>);
    impl AsRef<[u8]> for Counted {
        fn as_ref(&self) -> &[u8] {
            &self.0
        }
    }
    impl Drop for Counted {
        fn drop(&mut self) {
            self.1.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn a_vec_is_handed_over_not_copied() {
        let v = vec![1u8, 2, 3];
        let at = v.as_ptr();
        let s = Shared::from_owner(v);
        assert_eq!(s.as_slice().as_ptr(), at);
        assert_eq!(&*s, &[1, 2, 3]);
        let t = s.clone();
        assert!(s.same_owner(&t));
        assert_eq!(t.as_slice().as_ptr(), at);
    }

    #[test]
    fn equal_by_content_across_owners() {
        let a = Shared::from_owner(b"abc".to_vec());
        let b = Shared::from_static(b"abc");
        let c = Shared::from_owner(Box::<[u8]>::from(&b"xabcx"[..])).cut(1..4);
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert!(a < Shared::from_static(b"abd"));
        assert_eq!(Shared::default(), Shared::from_owner(Vec::new()));
        assert!(Shared::default().is_empty());
    }

    #[test]
    fn a_cut_keeps_the_owner_and_the_last_holder_drops_it() {
        let drops = Arc::new(AtomicUsize::new(0));
        let whole = Shared::from_owner(Counted(b"hello world".to_vec(), drops.clone()));
        let word = whole.cut(6..);
        let again = word.clone();
        drop(whole);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert_eq!(&*word, b"world");
        assert_eq!(&*again.cut(..=1), b"wo");
        drop(word);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(again);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn an_owner_counted_elsewhere_is_shared() {
        let owner = Arc::new(b"mapped".to_vec());
        let s = Shared::from_arc(owner.clone());
        assert_eq!(Arc::strong_count(&owner), 2);
        drop(owner);
        assert_eq!(&*s, b"mapped");
    }

    #[test]
    #[should_panic(expected = "range end index 5 out of range for slice of length 3")]
    fn a_cut_outside_stops() {
        let _ = Shared::from_static(b"abc").cut(1..5);
    }

    #[test]
    fn it_crosses_threads() {
        let s = Shared::from_owner(vec![7u8; 100]);
        let t = s.cut(10..20);
        let n = std::thread::spawn(move || t.iter().map(|&b| b as usize).sum::<usize>())
            .join()
            .unwrap();
        assert_eq!(n, 70);
        assert_eq!(s.len(), 100);
    }
}
