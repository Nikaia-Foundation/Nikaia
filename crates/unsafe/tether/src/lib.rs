//! Views that outlive the scope that read their buffer.
//!
//! A view of text is a plain `&str`. When views have to outlive the local that
//! read the buffer, the buffer moves somewhere that lives as long as they do:
//!
//! * [`Keep`] — an append-only home for buffers; every buffer in it stays at
//!   its address until the `Keep` is dropped.
//! * [`Held`] — one view of text carrying its own handle on the buffer, for a
//!   container that drops entries while it keeps reading.
//! * [`Holding`] — a whole struct of views carrying a handle on each buffer it
//!   points into, with a **safe** constructor: the struct is built inside a
//!   closure that is handed the keeps and nothing else with a lifetime it
//!   could keep, so every view in it provably points into them.
//! * [`Keeps`] / [`Rebase`] / [`Hold`] — how a value made *outside* such a
//!   closure is carried in: each view is found again in the keep it points
//!   into ([`Keep::view`]), checked by address, so a view that points anywhere
//!   else becomes a copy the keep owns rather than a dangling reference.
//!
//! Every `unsafe` in this crate, and the argument for it, is listed in
//! `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

/// **A value whose references may outlive what they point into, as far as the
/// compiler is told.**
///
/// A struct holding a `&'static str` promises the compiler that the text is
/// valid wherever the struct is: passed by value into a function, the
/// reference is taken to be valid for the whole call. A value here is dropped
/// *before* the buffer it points into, inside such a call - so the promise has
/// to be withdrawn. A `MaybeUninit` makes none, which is the whole of what this
/// wrapper is for (the same device as `yoke`'s `KindaSortaDangling`).
///
/// Safe in itself: it stores a value and hands it back by reference. It is
/// what a value holding stretched views has to be stored in, beside the handle
/// that keeps their buffer - as [`Held`] and [`Holding`] do.
pub struct Dangling<T>(std::mem::MaybeUninit<T>);

impl<T> Dangling<T> {
    pub fn new(value: T) -> Self {
        Dangling(std::mem::MaybeUninit::new(value))
    }

    pub fn get(&self) -> &T {
        // SAFETY: initialised in `new` and only taken apart in `drop`.
        unsafe { self.0.assume_init_ref() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: as in `get`.
        unsafe { self.0.assume_init_mut() }
    }
}

impl<T> Drop for Dangling<T> {
    fn drop(&mut self) {
        // SAFETY: initialised in `new`, dropped exactly once, here.
        unsafe { self.0.assume_init_drop() }
    }
}

impl<T: Clone> Clone for Dangling<T> {
    fn clone(&self) -> Self {
        Dangling::new(self.get().clone())
    }
}

/// Buffers whose views outlive the scope that made them.
///
/// **Append-only, and nothing in it ever moves**: a buffer is put behind an
/// `Arc` when it arrives and freed only when the `Keep` is, so a view of an
/// earlier buffer stays valid while later ones arrive. That is what lets one
/// `Keep` serve a loop over many files or a recursive include, with no
/// per-view count: one per buffer (ADR-tether-v1 D11).
///
/// **Each buffer has a place** ([`Place`]), handed out by [`Keep::put_at`] and
/// [`Keep::put_viewed_at`], and read back by it in O(1) with no lock
/// ([`Keep::view_in`], ADR-tether-v1 D15): the first buffer lives in a slot of
/// the keep itself, the rest in slots of chunks that double in size, each slot
/// set once. A put claims its place with one atomic step, so two tasks may put
/// through `&self` at once; nothing is ever moved or removed, so a slot once
/// set may be read for as long as the `Keep` is borrowed.
pub struct Keep {
    first: OnceLock<Entry>,
    rest: OnceLock<Box<Rest>>,
    /// How many places have been handed out.
    taken: AtomicUsize,
}

/// The places after the first: chunk `k` holds places `2^k .. 2^(k+1)`,
/// allocated when the first of them is taken.
struct Rest {
    chunks: [OnceLock<Box<[OnceLock<Entry>]>>; usize::BITS as usize],
}

/// **Where a buffer lives in its [`Keep`]**: handed out when it is put there,
/// so that a view of it can be checked against that one buffer
/// ([`Keep::view_in`]) instead of searched for among them all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place(usize);

/// The chunk of [`Rest`] place `at` (at least 1) is in, and its index there.
fn chunk_of(at: usize) -> (usize, usize) {
    let k = (usize::BITS - 1 - at.leading_zeros()) as usize;
    (k, at - (1 << k))
}

/// One buffer.
///
/// An `Arc` and not a `Box`: moving a `Box` asserts it is the only way to its
/// contents, which would invalidate every view already handed out - moving the
/// `Arc` touches nothing it points at.
enum Entry {
    Opaque(#[allow(dead_code)] Arc<dyn Any + Send + Sync>),
    /// Put in with [`Keep::put_viewed`]: one [`Keep::find`] can find a view in.
    Viewed(Arc<dyn Viewed>),
}

impl Entry {
    /// `text`, where it lies in this buffer: the same bytes, borrowed from it.
    fn within<'a>(&'a self, text: &str) -> Option<&'a str> {
        match self {
            Entry::Viewed(buffer) => within(buffer.bytes(), text),
            Entry::Opaque(_) => None,
        }
    }
}

impl Keep {
    pub fn new() -> Keep {
        Keep {
            first: OnceLock::new(),
            rest: OnceLock::new(),
            taken: AtomicUsize::new(0),
        }
    }

    /// The slot of place `at`, where it has been made.
    fn slot(&self, at: usize) -> Option<&OnceLock<Entry>> {
        if at == 0 {
            return Some(&self.first);
        }
        let (k, i) = chunk_of(at);
        self.rest.get()?.chunks[k].get()?.get(i)
    }

    /// The buffer at `at`, once it is there.
    fn entry(&self, at: Place) -> Option<&Entry> {
        self.slot(at.0)?.get()
    }

    fn keep(&self, entry: Entry) -> Place {
        let at = self.taken.fetch_add(1, Ordering::Relaxed);
        let slot = if at == 0 {
            &self.first
        } else {
            let (k, i) = chunk_of(at);
            let rest = self.rest.get_or_init(|| {
                Box::new(Rest {
                    chunks: std::array::from_fn(|_| OnceLock::new()),
                })
            });
            &rest.chunks[k].get_or_init(|| (0..1usize << k).map(|_| OnceLock::new()).collect())[i]
        };
        if slot.set(entry).is_err() {
            unreachable!("a place is handed out once");
        }
        Place(at)
    }

    /// Move a buffer in, and hand back the place it now lives, for as long as
    /// this `Keep` is borrowed.
    pub fn put<B: Any + Send + Sync>(&self, buffer: B) -> &B {
        self.put_at(buffer).0
    }

    /// [`Keep::put`], and the buffer's [`Place`] in this `Keep`.
    pub fn put_at<B: Any + Send + Sync>(&self, buffer: B) -> (&B, Place) {
        let shared: Arc<B> = Arc::new(buffer);
        let at: *const B = Arc::as_ptr(&shared);
        let place = self.keep(Entry::Opaque(shared));
        // SAFETY: the `Arc` was just stored and is never removed or handed out
        // until `self` is dropped, so the buffer it points at stays alive and
        // in place; nothing writes to it. So the reference is valid for as
        // long as `self` is borrowed, which is the lifetime it is given.
        (unsafe { &*at }, place)
    }

    /// [`Keep::put`], for a buffer views of text are cut from - which
    /// [`Keep::find`] can then find a view in.
    pub fn put_viewed<B: Viewed>(&self, buffer: B) -> &B {
        self.put_viewed_at(buffer).0
    }

    /// [`Keep::put_viewed`], and the buffer's [`Place`] in this `Keep`, which
    /// [`Keep::view_in`] checks a view against.
    pub fn put_viewed_at<B: Viewed>(&self, buffer: B) -> (&B, Place) {
        let shared: Arc<B> = Arc::new(buffer);
        let at: *const B = Arc::as_ptr(&shared);
        let place = self.keep(Entry::Viewed(shared));
        // SAFETY: as in `put_at`.
        (unsafe { &*at }, place)
    }

    /// The same text, as a view of a buffer this `Keep` holds - or `None`
    /// where `text` points into none of them.
    ///
    /// **Found by address**, not by content: the view handed back is the very
    /// bytes `text` is, now borrowed from this `Keep` rather than from
    /// wherever `text` was borrowed from. A search among the buffers, for a
    /// view the compiler names no buffer for; one it does goes to
    /// [`Keep::view_in`].
    pub fn find(&self, text: &str) -> Option<&str> {
        if text.is_empty() {
            return Some("");
        }
        let taken = self.taken.load(Ordering::Acquire);
        (0..taken).find_map(|at| self.slot(at)?.get()?.within(text))
    }

    /// The same text, as a view of this `Keep`: found in the buffer it points
    /// into, or - where it points into none of them, such as a literal - a
    /// copy of it put here. Never a reference to anything this `Keep` does
    /// not own.
    pub fn view(&self, text: &str) -> &str {
        match self.find(text) {
            Some(found) => found,
            None => self.copy(text),
        }
    }

    /// **The same text, as a view of the buffer at `at`**, which the compiler
    /// says it points into (ADR-tether-v1 D15): one comparison of addresses,
    /// no search. Where it is not there the plan was wrong, which is the
    /// compiler's defect: a program copies the text into this `Keep`, so it
    /// stays sound, and a run under `nikaia test` (which sets
    /// `NIKAIA_TEST_DIR` for every test it runs) panics at the caller's line
    /// instead, so the tests find a wrong plan rather than a copy hiding it.
    #[track_caller]
    pub fn view_in(&self, at: Place, text: &str) -> &str {
        self.checked(at, text, under_test())
    }

    /// [`Keep::view_in`], for a view the plan names this buffer for although
    /// it may be a literal there (`if c { w } else { "none" }`): where it is
    /// not in the buffer it is copied, in a test as in a program.
    pub fn view_near(&self, at: Place, text: &str) -> &str {
        self.checked(at, text, false)
    }

    #[track_caller]
    fn checked(&self, at: Place, text: &str, strict: bool) -> &str {
        if let Some(found) = self.entry(at).and_then(|entry| entry.within(text)) {
            return found;
        }
        if text.is_empty() {
            return "";
        }
        if strict {
            wrong_plan();
        }
        self.copy(text)
    }

    /// A copy of `text`, put here.
    fn copy(&self, text: &str) -> &str {
        self.put_viewed(text.to_owned()).as_str()
    }

    /// How many buffers are kept, for `--tethers` and for tests.
    pub fn len(&self) -> usize {
        self.taken.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Whether this run is one of `nikaia test`'s: the runner gives every test it
/// runs its own directory in `NIKAIA_TEST_DIR` (`nikaia_std::fs::TEST_DIR_VAR`,
/// ADR-247 D5). Read once, and only where a view missed its buffer.
fn under_test() -> bool {
    static UNDER_TEST: OnceLock<bool> = OnceLock::new();
    *UNDER_TEST.get_or_init(|| std::env::var_os("NIKAIA_TEST_DIR").is_some())
}

/// A view was not in the buffer the compiler named for it (ADR-tether-v1 D15).
#[track_caller]
#[cold]
fn wrong_plan() -> ! {
    panic!(
        "a view is not in the buffer the compiler planned for it (ADR-tether-v1 D15). This is a bug in Nikaia, not in your program: the compiler's plan of where its text lives was wrong. Please report it, with this message, at https://github.com/Nikaia-Foundation/Nikaia/issues."
    )
}

impl Default for Keep {
    fn default() -> Keep {
        Keep::new()
    }
}

/// `text`, where its bytes lie inside `whole`: the same bytes, borrowed from
/// `whole`.
fn within<'a>(whole: &'a [u8], text: &str) -> Option<&'a str> {
    let from = text.as_ptr().addr().checked_sub(whole.as_ptr().addr())?;
    let bytes = whole.get(from..from.checked_add(text.len())?)?;
    // SAFETY: `bytes` are the very bytes `text` is made of - the same
    // addresses, read through two shared borrows while neither is written -
    // and `text` is a `str`, so they are UTF-8.
    Some(unsafe { std::str::from_utf8_unchecked(bytes) })
}

/// **A buffer views of text are cut from**: owned text, or bytes that hold it.
///
/// What [`Keep::put_viewed`] takes, so that [`Keep::view`] can find a view in
/// it again by address.
pub trait Viewed: Any + Send + Sync {
    /// The buffer's bytes, where its views point.
    fn bytes(&self) -> &[u8];
}

impl Viewed for String {
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Viewed for Box<str> {
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Viewed for Arc<str> {
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Viewed for Vec<u8> {
    fn bytes(&self) -> &[u8] {
        self
    }
}

/// One view of text that carries its own handle on the buffer it points into
/// for a container that drops entries while the loop around it
/// keeps reading, where one `Keep` for the whole loop would keep every buffer
/// the loop ever read.
///
/// **Compared and hashed by content**, never by where it points: two files
/// that both say `Hamburg` are one key.
#[derive(Clone)]
pub struct Held {
    // Declared first, so it is dropped before the handle on its buffer.
    text: Dangling<&'static str>,
    /// `None` for a literal, which needs no keep (ADR-tether-v1 D8, D15).
    _keep: Option<Arc<Keep>>,
}

impl Held {
    fn text(&self) -> &str {
        self.text.get()
    }
}

impl Held {
    /// `text`, held by the keep it points into: the first of `keeps` that
    /// holds it, or - where none does - a copy of it put into the first.
    ///
    /// # Panics
    ///
    /// Where `keeps` is empty.
    pub fn new(keeps: &[Arc<Keep>], text: &str) -> Held {
        let (keep, view) = keeps
            .iter()
            .find_map(|keep| keep.find(text).map(|view| (keep, view)))
            .unwrap_or_else(|| {
                let keep = keeps.first().expect("a held view needs a keep");
                (keep, keep.view(text))
            });
        Held {
            // SAFETY: `view` points into `keep` - `find` and `view` hand back
            // nothing else - and the clone stored beside it keeps `keep`, and
            // so the buffer, alive and in place as long as this value.
            text: Dangling::new(unsafe { std::mem::transmute::<&str, &'static str>(view) }),
            _keep: Some(Arc::clone(keep)),
        }
    }

    /// `text`, held by `keep`, where the compiler says it lies in the buffer
    /// at `at` ([`Keep::view_in`]: one comparison; a miss is the compiler's
    /// fault, copied in a program and a panic under `nikaia test`).
    #[track_caller]
    pub fn in_place(keep: Arc<Keep>, at: Place, text: &str) -> Held {
        // SAFETY: as in `new` - `view_in` hands back nothing but a view of
        // `keep`'s own buffers, and `keep` is stored beside it.
        let view = unsafe { std::mem::transmute::<&str, &'static str>(keep.view_in(at, text)) };
        Held {
            text: Dangling::new(view),
            _keep: Some(keep),
        }
    }

    /// [`Held::in_place`], for a view that may be a literal where the plan
    /// names a buffer ([`Keep::view_near`]): a miss is copied, never a panic.
    pub fn near(keep: Arc<Keep>, at: Place, text: &str) -> Held {
        // SAFETY: as in `in_place`.
        let view = unsafe { std::mem::transmute::<&str, &'static str>(keep.view_near(at, text)) };
        Held {
            text: Dangling::new(view),
            _keep: Some(keep),
        }
    }

    /// **A literal, held with no keep and no copy** (ADR-tether-v1 D8, D15):
    /// it is `'static` already.
    pub fn literal(text: &'static str) -> Held {
        Held {
            text: Dangling::new(text),
            _keep: None,
        }
    }
}

impl std::ops::Deref for Held {
    type Target = str;
    fn deref(&self) -> &str {
        self.text()
    }
}

impl std::borrow::Borrow<str> for Held {
    fn borrow(&self) -> &str {
        self.text()
    }
}

impl AsRef<str> for Held {
    fn as_ref(&self) -> &str {
        self.text()
    }
}

impl PartialEq for Held {
    fn eq(&self, other: &Held) -> bool {
        self.text() == other.text()
    }
}

impl Eq for Held {}

impl PartialEq<str> for Held {
    fn eq(&self, other: &str) -> bool {
        self.text() == other
    }
}

impl PartialEq<&str> for Held {
    fn eq(&self, other: &&str) -> bool {
        self.text() == *other
    }
}

impl PartialOrd for Held {
    fn partial_cmp(&self, other: &Held) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Held {
    fn cmp(&self, other: &Held) -> std::cmp::Ordering {
        self.text().cmp(other.text())
    }
}

impl std::hash::Hash for Held {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text().hash(state)
    }
}

impl std::fmt::Display for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.text(), f)
    }
}

impl std::fmt::Debug for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.text(), f)
    }
}

/// **A struct of views, over any lifetime** — what [`Holding`] holds.
///
/// Implemented once per struct, always the same way:
///
/// ```
/// struct Record<'a> { name: &'a str }
/// enum RecordViews {}
/// impl tether::Views for RecordViews {
///     type Of<'a> = Record<'a>;
///     fn shorten<'long: 's, 's>(x: &'s Record<'long>) -> &'s Record<'s> { x }
/// }
/// ```
///
/// The body `x` compiles only where the struct is **covariant** in its
/// lifetime, which is the one property [`Holding`] relies on: a struct that
/// could be written through (a `Cell<&'a str>`, a `&'a mut`) is refused by
/// `rustc` right here.
pub trait Views {
    type Of<'a>;
    fn shorten<'long: 's, 's>(x: &'s Self::Of<'long>) -> &'s Self::Of<'s>;
}

/// **The keeps a [`Holding`] is built from**, borrowed for a lifetime the
/// closure building it cannot name - so what it builds can point into them
/// and nowhere else.
pub struct Keeps<'a, const N: usize> {
    keeps: [&'a Keep; N],
}

impl<'a, const N: usize> Keeps<'a, N> {
    /// `text`, as a view of the keep it points into - or, where it points into
    /// none of them, a copy put into the first ([`Keep::view`]).
    pub fn view(&self, text: &str) -> &'a str {
        const { assert!(N > 0, "a holding needs a keep") };
        self.keeps
            .iter()
            .find_map(|keep| keep.find(text))
            .unwrap_or_else(|| self.keeps[0].view(text))
    }

    /// `text`, as a view of the buffer at `at` in keep `keep`, which the
    /// compiler says it points into ([`Keep::view_in`]).
    #[track_caller]
    pub fn view_in(&self, keep: usize, at: Place, text: &str) -> &'a str {
        self.keeps[keep].view_in(at, text)
    }

    /// [`Keeps::view_in`], where the view may be a literal
    /// ([`Keep::view_near`]).
    pub fn view_near(&self, keep: usize, at: Place, text: &str) -> &'a str {
        self.keeps[keep].view_near(at, text)
    }

    /// One of the keeps.
    pub fn get(&self, at: usize) -> &'a Keep {
        self.keeps[at]
    }
}

/// **A value whose views are carried into [`Keeps`]**: the same value, each
/// view in it found again in the keep it points into.
///
/// Implemented here for a view of text and the containers of one a struct of
/// views holds; the Nikaia compiler writes it for each such struct, one
/// [`Rebase::rebase`] per field that holds a view.
pub trait Rebase<'a> {
    /// The same type, over `'a`.
    type At;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Self::At;

    /// The same, where the compiler names the buffer at `at` of the first
    /// keep for every view in it: each view of text is checked against that
    /// buffer ([`Keeps::view_near`]) instead of searched for. What has no such
    /// view, or is not written for it, is rebased as [`Rebase::rebase`] does.
    fn rebase_near<const N: usize>(self, keeps: &Keeps<'a, N>, at: Place) -> Self::At
    where
        Self: Sized,
    {
        let _ = at;
        self.rebase(keeps)
    }
}

impl<'a> Rebase<'a> for &str {
    type At = &'a str;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> &'a str {
        keeps.view(self)
    }
    fn rebase_near<const N: usize>(self, keeps: &Keeps<'a, N>, at: Place) -> &'a str {
        keeps.view_near(0, at, self)
    }
}

impl<'a, T: Rebase<'a>> Rebase<'a> for Option<T> {
    type At = Option<T::At>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Option<T::At> {
        self.map(|value| value.rebase(keeps))
    }
    fn rebase_near<const N: usize>(self, keeps: &Keeps<'a, N>, at: Place) -> Option<T::At> {
        self.map(|value| value.rebase_near(keeps, at))
    }
}

impl<'a, T: Rebase<'a>> Rebase<'a> for Vec<T> {
    type At = Vec<T::At>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Vec<T::At> {
        self.into_iter().map(|value| value.rebase(keeps)).collect()
    }
    fn rebase_near<const N: usize>(self, keeps: &Keeps<'a, N>, at: Place) -> Vec<T::At> {
        self.into_iter()
            .map(|value| value.rebase_near(keeps, at))
            .collect()
    }
}

/// **A value with no view in it is carried in as it is**: a number, a
/// `bool`, a `char`, owned text. What a task takes with it is often a map
/// from views to counts, and the counts have nowhere to point.
macro_rules! rebased_as_they_are {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> Rebase<'a> for $ty {
                type At = $ty;
                fn rebase<const N: usize>(self, _keeps: &Keeps<'a, N>) -> $ty {
                    self
                }
            }
        )*
    };
}

rebased_as_they_are!(
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    bool,
    char,
    String,
    ()
);

impl<'a, T: Rebase<'a>> Rebase<'a> for Box<T> {
    type At = Box<T::At>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Box<T::At> {
        Box::new((*self).rebase(keeps))
    }
}

impl<'a, K, V, S> Rebase<'a> for std::collections::HashMap<K, V, S>
where
    K: Rebase<'a>,
    K::At: Eq + std::hash::Hash,
    V: Rebase<'a>,
    S: std::hash::BuildHasher + Default,
{
    type At = std::collections::HashMap<K::At, V::At, S>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Self::At {
        self.into_iter()
            .map(|(key, value)| (key.rebase(keeps), value.rebase(keeps)))
            .collect()
    }
}

/// The table Nikaia's maps are (`nikaia-std`'s `hash.rs`), rebased as `std`'s
/// is.
impl<'a, K, V, S> Rebase<'a> for hashbrown::HashMap<K, V, S>
where
    K: Rebase<'a>,
    K::At: Eq + std::hash::Hash,
    V: Rebase<'a>,
    S: std::hash::BuildHasher + Default,
{
    type At = hashbrown::HashMap<K::At, V::At, S>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Self::At {
        self.into_iter()
            .map(|(key, value)| (key.rebase(keeps), value.rebase(keeps)))
            .collect()
    }
}

impl<'a, K, V> Rebase<'a> for std::collections::BTreeMap<K, V>
where
    K: Rebase<'a>,
    K::At: Ord,
    V: Rebase<'a>,
{
    type At = std::collections::BTreeMap<K::At, V::At>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Self::At {
        self.into_iter()
            .map(|(key, value)| (key.rebase(keeps), value.rebase(keeps)))
            .collect()
    }
}

impl<'a, T, S> Rebase<'a> for std::collections::HashSet<T, S>
where
    T: Rebase<'a>,
    T::At: Eq + std::hash::Hash,
    S: std::hash::BuildHasher + Default,
{
    type At = std::collections::HashSet<T::At, S>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Self::At {
        self.into_iter().map(|value| value.rebase(keeps)).collect()
    }
}

impl<'a, T> Rebase<'a> for std::collections::BTreeSet<T>
where
    T: Rebase<'a>,
    T::At: Ord,
{
    type At = std::collections::BTreeSet<T::At>;
    fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Self::At {
        self.into_iter().map(|value| value.rebase(keeps)).collect()
    }
}

macro_rules! rebased_tuples {
    ($(($($name:ident),+)),* $(,)?) => {
        $(
            #[allow(non_snake_case)]
            impl<'a, $($name: Rebase<'a>),+> Rebase<'a> for ($($name,)+) {
                type At = ($($name::At,)+);
                fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Self::At {
                    let ($($name,)+) = self;
                    ($($name.rebase(keeps),)+)
                }
            }
        )*
    };
}

rebased_tuples!(
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);

/// **A value put into a container that drops entries**, held by the keeps
/// of the buffers it points into: a view of text becomes a [`Held`], a
/// struct of views a [`Holding`].
///
/// The Nikaia compiler writes it for each struct of views, as
/// `Holding::new(keeps, move |k| Rebase::rebase(self, &k))`.
pub trait Hold<const N: usize> {
    type Held;
    fn hold(self, keeps: [Arc<Keep>; N]) -> Self::Held;

    /// The same, where the compiler names the buffer at `at` of the first
    /// keep for the views in it (ADR-tether-v1 D15): a view of text is checked
    /// against that buffer ([`Held::near`]) instead of searched for; what is
    /// not written for it is held as [`Hold::hold`] does.
    fn hold_near(self, keeps: [Arc<Keep>; N], at: Place) -> Self::Held
    where
        Self: Sized,
    {
        let _ = at;
        self.hold(keeps)
    }
}

impl<const N: usize> Hold<N> for &str {
    type Held = Held;
    fn hold(self, keeps: [Arc<Keep>; N]) -> Held {
        Held::new(&keeps, self)
    }
    fn hold_near(self, keeps: [Arc<Keep>; N], at: Place) -> Held {
        match keeps.into_iter().next() {
            Some(keep) => Held::near(keep, at, self),
            None => panic!("a held view needs a keep"),
        }
    }
}

impl<const N: usize, T: Hold<N>> Hold<N> for Option<T> {
    type Held = Option<T::Held>;
    fn hold(self, keeps: [Arc<Keep>; N]) -> Option<T::Held> {
        self.map(|value| value.hold(keeps))
    }
    fn hold_near(self, keeps: [Arc<Keep>; N], at: Place) -> Option<T::Held> {
        self.map(|value| value.hold_near(keeps, at))
    }
}

/// **A struct of views held beside the buffers it points into.**
///
/// One handle per buffer, `N` of them, and no allocation of its own: each
/// buffer lives in a [`Keep`] behind an `Arc` and does not move when this
/// value does. A read goes through [`Holding::get`], which hands the struct out
/// over a lifetime no longer than the read, so nothing derived from it can
/// outlive the buffers.
pub struct Holding<V: Views, const N: usize = 1> {
    // Declared first, so it is dropped before the buffers it points into.
    value: Dangling<V::Of<'static>>,
    keeps: [Arc<Keep>; N],
}

impl<V: Views, const N: usize> Holding<V, N> {
    /// Build the struct from the keeps. The closure is handed them and
    /// nothing else with a lifetime it could keep, and it has to work for
    /// **every** lifetime, so what it returns can point into them or at
    /// `'static` data and nowhere else.
    pub fn new(
        keeps: [Arc<Keep>; N],
        build: impl for<'a> FnOnce(Keeps<'a, N>) -> V::Of<'a>,
    ) -> Self {
        let value = build(Keeps {
            keeps: keeps.each_ref().map(|keep| &**keep),
        });
        let value = std::mem::ManuallyDrop::new(value);
        // SAFETY: `V::Of<'_>` and `V::Of<'static>` are one type up to a
        // lifetime, so the bits are the same value; `ManuallyDrop` keeps the
        // original from being dropped twice. The longer lifetime is never
        // observed: `value` is private, dropped before `keeps` (field order),
        // and handed out only through `get`, which shortens it back to a
        // borrow of `self` - and `self` holds the `Arc`s that keep each
        // `Keep`, and every buffer in it, where it is.
        let value = unsafe { std::mem::transmute_copy::<V::Of<'_>, V::Of<'static>>(&value) };
        Holding {
            value: Dangling::new(value),
            keeps,
        }
    }

    /// The struct, over a lifetime no longer than this borrow.
    pub fn get(&self) -> &V::Of<'_> {
        V::shorten(self.value.get())
    }

    /// The keeps the struct points into.
    pub fn keeps(&self) -> &[Arc<Keep>; N] {
        &self.keeps
    }

    /// **Write the struct**, through a closure handed it and the keeps over a
    /// lifetime it cannot name - so what it writes into a view field can only
    /// come from the keeps (a view made outside comes in through
    /// [`Keeps::view`]) or be `'static`. `self_cell`'s `with_dependent_mut`,
    /// for the same reason.
    ///
    /// A view of a local is refused by `rustc`, which is the whole argument:
    ///
    /// ```compile_fail
    /// # use std::sync::Arc;
    /// # use tether::{Hold, Holding, Keep, Keeps, Rebase, Views};
    /// # struct Name<'a>(&'a str);
    /// # enum NameViews {}
    /// # impl Views for NameViews {
    /// #     type Of<'a> = Name<'a>;
    /// #     fn shorten<'long: 's, 's>(x: &'s Name<'long>) -> &'s Name<'s> { x }
    /// # }
    /// let keep = Arc::new(Keep::new());
    /// let mut held: Holding<NameViews> = Holding::new([keep], |k| Name(k.view("a")));
    /// let local = String::from("gone soon");
    /// held.with_mut(|name, _| name.0 = &local);
    /// ```
    pub fn with_mut<R>(
        &mut self,
        write: impl for<'a> FnOnce(&mut V::Of<'a>, Keeps<'a, N>) -> R,
    ) -> R {
        let keeps = Keeps {
            keeps: self.keeps.each_ref().map(|keep| &**keep),
        };
        let value: *mut V::Of<'static> = self.value.get_mut();
        // SAFETY: the same value up to a lifetime, borrowed mutably for as
        // long as `self` is. The lifetime handed out is one the closure has to
        // treat as any lifetime at all: behind `&mut` it cannot be shortened,
        // so the closure can store into the struct only what lives at least as
        // long as the keeps do - a view of them, or `'static` data - and the
        // keeps live as long as `self`.
        let value: &mut V::Of<'_> = unsafe { &mut *value.cast() };
        write(value, keeps)
    }
}

impl<V: Views, const N: usize> Clone for Holding<V, N>
where
    for<'a> V::Of<'a>: Clone,
{
    fn clone(&self) -> Self {
        let copy = self.get().clone();
        let copy = std::mem::ManuallyDrop::new(copy);
        // SAFETY: as in `new` - the copy points where the original does, into
        // `self.keeps`, and the clones of the `Arc`s stored beside it keep
        // those buffers alive and in place.
        let value = unsafe { std::mem::transmute_copy::<V::Of<'_>, V::Of<'static>>(&copy) };
        Holding {
            value: Dangling::new(value),
            keeps: self.keeps.clone(),
        }
    }
}

impl<V: Views, const N: usize> std::fmt::Debug for Holding<V, N>
where
    for<'a> V::Of<'a>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.get(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Record<'a> {
        name: &'a str,
        parts: Vec<&'a str>,
    }
    enum RecordViews {}
    impl Views for RecordViews {
        type Of<'a> = Record<'a>;
        fn shorten<'long: 's, 's>(x: &'s Record<'long>) -> &'s Record<'s> {
            x
        }
    }

    impl<'a> Rebase<'a> for Record<'_> {
        type At = Record<'a>;
        fn rebase<const N: usize>(self, keeps: &Keeps<'a, N>) -> Record<'a> {
            Record {
                name: self.name.rebase(keeps),
                parts: self.parts.rebase(keeps),
            }
        }
    }

    impl<const N: usize> Hold<N> for Record<'_> {
        type Held = Holding<RecordViews, N>;
        fn hold(self, keeps: [Arc<Keep>; N]) -> Self::Held {
            Holding::new(keeps, move |k| self.rebase(&k))
        }
    }

    /// A keep of its own holding one buffer, and that buffer's text.
    fn kept(text: &str) -> Arc<Keep> {
        let keep = Arc::new(Keep::new());
        keep.put_viewed(text.to_string());
        keep
    }

    #[test]
    fn a_struct_of_views_outlives_the_scope_that_read_its_buffer() {
        let mut kept: std::collections::VecDeque<Holding<RecordViews>> = Default::default();
        for i in 0..20 {
            let keep = Arc::new(Keep::new());
            let text = keep.put_viewed(format!("  a{i} b{i} c{i}  "));
            kept.push_back(Holding::new([Arc::clone(&keep)], |k| Record {
                name: k.view(text.trim()),
                parts: text.split_whitespace().map(|p| k.view(p)).collect(),
            }));
            // The element is now the buffer's only owner.
            drop(keep);
            if kept.len() > 3 {
                kept.pop_front();
            }
        }
        // Every element moves.
        let moved: Vec<Holding<RecordViews>> = kept.into_iter().collect();
        let names: Vec<&str> = moved.iter().map(|h| h.get().name).collect();
        assert_eq!(names, ["a17 b17 c17", "a18 b18 c18", "a19 b19 c19"]);
        assert_eq!(moved[2].get().parts, ["a19", "b19", "c19"]);
    }

    /// The case `Dangling` exists for: the last owner of a buffer handed by
    /// value into a function that drops it there - for `Held` and `Holding`.
    #[test]
    fn the_last_owner_is_dropped_inside_a_call() {
        fn consume_held(held: Held) -> usize {
            let n = held.len();
            drop(held);
            n
        }
        fn consume_holding(holding: Holding<RecordViews>) -> usize {
            let n = holding.get().parts.len();
            drop(holding);
            n
        }
        let keep = Arc::new(Keep::new());
        let text = keep.put_viewed("one two".to_string());
        let held = Held::new(&[Arc::clone(&keep)], &text[..3]);
        drop(keep);
        assert_eq!(consume_held(held), 3);
        let keep = Arc::new(Keep::new());
        let text = keep.put_viewed("a b c".to_string());
        let holding = Record {
            name: text,
            parts: text.split(' ').collect(),
        }
        .hold([Arc::clone(&keep)]);
        drop(keep);
        assert_eq!(consume_holding(holding), 3);
    }

    /// **A map from views to counts is carried in whole**: each key found
    /// again in the keep by address, each count as it is - what a task takes
    /// with it most often (Nikaia's #84).
    #[test]
    fn a_map_of_views_is_rebased_without_a_copy() {
        enum CountViews {}
        impl Views for CountViews {
            type Of<'a> = std::collections::HashMap<&'a str, (i64, Vec<&'a str>)>;
            fn shorten<'long: 's, 's>(
                x: &'s std::collections::HashMap<&'long str, (i64, Vec<&'long str>)>,
            ) -> &'s std::collections::HashMap<&'s str, (i64, Vec<&'s str>)> {
                x
            }
        }
        let keep = Arc::new(Keep::new());
        let text = keep.put_viewed("a b a".to_string());
        let mut counts: std::collections::HashMap<&str, (i64, Vec<&str>)> =
            std::collections::HashMap::new();
        for word in text.split(' ') {
            let entry = counts.entry(word).or_insert((0, Vec::new()));
            entry.0 += 1;
            entry.1.push(word);
        }
        let held: Holding<CountViews> =
            Holding::new([Arc::clone(&keep)], move |keeps| counts.rebase(&keeps));
        // Found, not copied: the keep holds the one buffer it was given.
        assert_eq!(keep.len(), 1);
        drop(keep);
        assert_eq!(held.get()["a"].0, 2);
        assert_eq!(held.get()["b"].1, ["b"]);
    }

    /// **The same, in the table Nikaia's maps are** (#420).
    #[test]
    fn a_hashbrown_map_of_views_is_rebased_without_a_copy() {
        enum CountViews {}
        impl Views for CountViews {
            type Of<'a> = hashbrown::HashMap<&'a str, (i64, Vec<&'a str>), std::hash::RandomState>;
            fn shorten<'long: 's, 's>(
                x: &'s hashbrown::HashMap<
                    &'long str,
                    (i64, Vec<&'long str>),
                    std::hash::RandomState,
                >,
            ) -> &'s hashbrown::HashMap<&'s str, (i64, Vec<&'s str>), std::hash::RandomState>
            {
                x
            }
        }
        let keep = Arc::new(Keep::new());
        let text = keep.put_viewed("a b a".to_string());
        let mut counts: hashbrown::HashMap<&str, (i64, Vec<&str>), std::hash::RandomState> =
            hashbrown::HashMap::default();
        for word in text.split(' ') {
            let entry = counts.entry(word).or_insert((0, Vec::new()));
            entry.0 += 1;
            entry.1.push(word);
        }
        let held: Holding<CountViews> =
            Holding::new([Arc::clone(&keep)], move |keeps| counts.rebase(&keeps));
        // Found, not copied: the keep holds the one buffer it was given.
        assert_eq!(keep.len(), 1);
        drop(keep);
        assert_eq!(held.get()["a"].0, 2);
        assert_eq!(held.get()["b"].1, ["b"]);
    }

    #[test]
    fn a_held_struct_crosses_a_thread() {
        let keep = Arc::new(Keep::new());
        let text = keep.put_viewed("x y".to_string());
        let held: Holding<RecordViews> = Record {
            name: text,
            parts: text.split(' ').collect(),
        }
        .hold([Arc::clone(&keep)]);
        let got = std::thread::spawn(move || held.get().parts.len())
            .join()
            .unwrap();
        assert_eq!(got, 2);
    }

    /// What the compiler writes: the value is made first, with views of the
    /// buffer borrowed the ordinary way, and carried in afterwards - each view
    /// found again in the keep by address, so nothing is copied.
    #[test]
    fn a_value_made_outside_is_carried_in_without_a_copy() {
        let keep = kept("  oslo 12  ");
        let before = keep.len();
        // The same text in a *different* allocation is not found: the search
        // is by address, not by content.
        let elsewhere = String::from("oslo");
        assert!(keep.find(&elsewhere).is_none());
        assert_eq!(keep.find(""), Some(""));
        let held = Record {
            name: "static",
            parts: vec![],
        }
        .hold([Arc::clone(&keep)]);
        assert_eq!(held.get().name, "static");
        // The literal pointed into no buffer of the keep, so a copy of it
        // went in - the one case that pays.
        assert_eq!(keep.len(), before + 1);
    }

    #[test]
    fn a_view_of_the_keep_is_found_where_it_points() {
        let keep = Arc::new(Keep::new());
        let text = keep.put_viewed("alpha beta".to_string());
        let beta = &text[6..];
        let found = keep.find(beta).expect("a view of the keep is found");
        assert_eq!(found.as_ptr(), beta.as_ptr());
        assert_eq!(found, "beta");
        let held = Record {
            name: beta,
            parts: text.split(' ').collect(),
        }
        .hold([Arc::clone(&keep)]);
        assert_eq!(held.get().name.as_ptr(), beta.as_ptr());
        // Nothing was copied in.
        assert_eq!(keep.len(), 1);
    }

    /// Two buffers, one handle on each: the struct keeps both alive and
    /// neither is copied.
    #[test]
    fn a_struct_of_views_into_two_buffers_holds_both() {
        let mut kept: Vec<Holding<RecordViews, 2>> = Vec::new();
        for i in 0..5 {
            let names = Arc::new(Keep::new());
            let parts = Arc::new(Keep::new());
            let name = names.put_viewed(format!("name{i}"));
            let list = parts.put_viewed(format!("p{i} q{i}"));
            kept.push(
                Record {
                    name,
                    parts: list.split(' ').collect(),
                }
                .hold([Arc::clone(&names), Arc::clone(&parts)]),
            );
            drop((names, parts));
            if kept.len() > 2 {
                kept.remove(0);
            }
        }
        assert_eq!(kept[1].get().name, "name4");
        assert_eq!(kept[1].get().parts, ["p4", "q4"]);
        assert_eq!(kept[1].keeps()[0].len(), 1);
        assert_eq!(kept[1].keeps()[1].len(), 1);
        let copied = kept[0].clone();
        drop(kept);
        assert_eq!(copied.get().parts, ["p3", "q3"]);
    }

    /// A write through the handle: a field of its own is written as it is, a
    /// view made outside is found again in the keeps first.
    #[test]
    fn a_held_struct_is_written_through_its_handle() {
        let keep = Arc::new(Keep::new());
        let text = keep.put_viewed("alpha beta".to_string());
        let mut held = Record {
            name: &text[..5],
            parts: vec![],
        }
        .hold([Arc::clone(&keep)]);
        let beta = &text[6..];
        held.with_mut(|record, keeps| {
            record.name = keeps.view(beta);
            record.parts.push("static");
        });
        drop(keep);
        assert_eq!(held.get().name, "beta");
        assert_eq!(held.get().parts, ["static"]);
        assert_eq!(held.keeps()[0].len(), 1);
    }

    /// **A view is checked against the buffer named for it** (ADR-tether-v1
    /// D15): found there by its place, among many, with nothing copied.
    #[test]
    fn a_view_is_found_in_the_buffer_at_its_place() {
        let keep = Keep::new();
        let mut places = Vec::new();
        for n in 0..100 {
            let (text, at) = keep.put_viewed_at(format!("buffer {n}"));
            places.push((text.as_str(), at));
        }
        assert_eq!(keep.len(), 100);
        for (n, (text, at)) in places.iter().enumerate() {
            let tail = &text[7..];
            let found = keep.view_in(*at, tail);
            assert_eq!(found.as_ptr(), tail.as_ptr());
            assert_eq!(found, n.to_string());
        }
        // Nothing was copied in.
        assert_eq!(keep.len(), 100);
        assert_eq!(keep.view_in(places[3].1, ""), "");
    }

    /// **A view not in its named buffer is the compiler's fault**: a program
    /// copies it, a run under `nikaia test` panics.
    #[test]
    fn a_view_not_in_its_named_buffer_is_copied_or_panics_under_test() {
        let keep = Keep::new();
        let (first, at_first) = keep.put_viewed_at("first".to_string());
        let (_, at_second) = keep.put_viewed_at("second".to_string());
        // In a program: a copy the keep owns.
        let copied = keep.checked(at_second, &first[..3], false);
        assert_eq!(copied, "fir");
        assert_ne!(copied.as_ptr(), first.as_ptr());
        assert_eq!(keep.len(), 3);
        // Under `nikaia test`: a panic naming the compiler.
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            keep.checked(at_second, &first[..3], true);
        }));
        let said = caught.expect_err("a wrong plan panics under test");
        let said = said
            .downcast_ref::<String>()
            .map(String::as_str)
            .or(said.downcast_ref::<&str>().copied());
        assert!(
            said.unwrap_or("")
                .contains("bug in Nikaia, not in your program")
        );
        // The right buffer is found under test too.
        assert_eq!(keep.checked(at_first, &first[1..], true), "irst");
        // A literal where the plan may hand one is copied either way.
        assert_eq!(keep.view_near(at_first, "literal"), "literal");
    }

    /// Places stay right while tasks put through `&self` at once.
    #[test]
    fn buffers_put_from_two_threads_keep_their_places() {
        let keep = Arc::new(Keep::new());
        let workers: Vec<_> = (0..4)
            .map(|t| {
                let keep = Arc::clone(&keep);
                std::thread::spawn(move || {
                    let mut seen = Vec::new();
                    for n in 0..50 {
                        let (text, at) = keep.put_viewed_at(format!("{t}-{n}"));
                        seen.push((at, text.len()));
                    }
                    for (at, len) in seen {
                        let whole = keep.entry(at).and_then(|e| match e {
                            Entry::Viewed(b) => Some(b.bytes().len()),
                            Entry::Opaque(_) => None,
                        });
                        assert_eq!(whole, Some(len));
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(keep.len(), 200);
    }

    #[test]
    fn a_held_literal_takes_no_keep() {
        let held = Held::literal("static");
        assert_eq!(&*held, "static");
        assert!(held._keep.is_none());
        let keep = Arc::new(Keep::new());
        let (text, at) = keep.put_viewed_at("a b".to_string());
        let b = &text[2..];
        let held = Held::in_place(Arc::clone(&keep), at, b);
        assert_eq!(held.as_ptr(), b.as_ptr());
        drop(keep);
        assert_eq!(&*held, "b");
        let near = Held::near(Arc::new(Keep::new()), Place(0), "x");
        assert_eq!(&*near, "x");
    }

    #[test]
    fn a_later_buffer_does_not_move_an_earlier_one() {
        let keep = Keep::new();
        let first: &String = keep.put("first".to_string());
        let view: &str = &first[..3];
        for n in 0..1000 {
            keep.put(format!("buffer {n}"));
        }
        assert_eq!(view, "fir");
        assert_eq!(keep.len(), 1001);
    }

    // `Held` holds a handle on a `Keep`, whose slots are interior mutability -
    // but a key hashes and compares by its text alone, which never changes.
    #[allow(clippy::mutable_key_type)]
    #[test]
    fn a_held_view_outlives_the_scope_that_read_it_and_compares_by_content() {
        let mut seen: std::collections::HashSet<Held> = Default::default();
        for text in ["Hamburg;1", "Hamburg;2"] {
            let keep = Arc::new(Keep::new());
            let buffer = keep.put_viewed(text.to_string());
            let name = buffer.split(';').next().unwrap_or("");
            seen.insert(name.hold([Arc::clone(&keep)]));
        }
        assert_eq!(seen.len(), 1);
        assert!(seen.contains("Hamburg"));
    }
}

/// The README's example, run as a test.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
