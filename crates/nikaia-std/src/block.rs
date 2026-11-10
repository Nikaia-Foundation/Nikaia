//! **A task's memory block** ([ADR-327](../../../docs/specification/adr/adr-327.md)
//! D4, D5, D10, D12; #490 step 3).
//!
//! A task allocates from a block of its own, and the block is its budget: 16
//! pages of 4 KiB, each counting what it holds, so what the task frees it can
//! allocate again (D10) and its end hands the block back whole. The pages,
//! the handle and the lists and text whose growth the block may refuse are
//! `task-block`'s (`crates/unsafe/task-block`, ADR-218); [`crate::task`] puts
//! a task's block in place while the task is polled.
//!
//! **Where a value lives is its handle, [`Place`]**: the running task's block,
//! or outside, where `std`'s global allocator answers (D8, D9). Which
//! allocation sites are in the block is the emitter's decision (#490 step 5);
//! until it decides, every site is outside.
//!
//! **[`List`] and [`Text`] are the ground of Nikaia's `List` and `String`**
//! (D4): a list or text placed by its handle, `FromIterator` for `.collect()`
//! (a newtype, since the orphan rule allows none on `allocator-api2`'s `Vec`),
//! and the allocating text methods built where the handle says, with no
//! `String` between ([`to_uppercase`], [`to_lowercase`], [`replace`],
//! [`replacen`], [`repeat`], [`join`], [`format`]). Toward Rust they convert
//! with `From`/`Into`, the same memory where it is outside the running
//! task's block (D8). The emitter still writes `Vec` and `String` until it
//! switches to them (#490).
//!
//! **A full block ends the task in [`full`]**, outside the allocator, which
//! only refuses (D6); so every growth of a [`List`] or a [`Text`] goes
//! through `try_reserve` ([`push`], [`reserve`], [`extend`]).

pub use task_block::text::{format, join, repeat, replace, replacen, to_lowercase, to_uppercase};
pub use task_block::{
    BLOCK, Block, Counting, Entered, Housed, List, Place, Text, count, enter, extend, full,
    hand_over, held, leftover, list, push, reserve, take_over,
};

use std::cell::Cell;
use std::sync::atomic::{AtomicI64, Ordering};

thread_local! {
    /// **The budget of the next task this thread starts** (ADR-327 D15), as
    /// the decision that admitted it gave it; `-1` where none did.
    static NEXT: Cell<i64> = const { Cell::new(-1) };
    /// Task blocks made on this thread since the program's budget was last
    /// decided here.
    static BEAT: Cell<u32> = const { Cell::new(0) };
}

/// **The program's budget** (ADR-327 D7, D15): what a task gets that no
/// decision of its own admitted, `std`'s strategy deciding it before `main`
/// runs and again on the beat. Unbounded until the first decision.
static PROGRAM: AtomicI64 = AtomicI64::new(i64::MAX);

/// How many task blocks a thread makes between two decisions of the
/// program's budget.
const BEAT_EVERY: u32 = 1024;

/// A budget in bytes as a block takes it.
fn bytes(budget: i64) -> usize {
    usize::try_from(budget.max(0)).unwrap_or(usize::MAX)
}

/// **A decision admitted the next task with `budget` bytes** (ADR-327 D15):
/// the next task block this thread makes takes it.
pub(crate) fn granted(budget: i64) {
    NEXT.with(|next| next.set(budget.max(0)));
}

/// The running task's block takes the decision's budget (a detached task,
/// whose strategy is asked inside it before its body runs).
pub(crate) fn budget_running(budget: i64) {
    task_block::budget_running(bytes(budget));
}

/// **The program's budget, decided now by `std`'s strategy** (ADR-327 D14,
/// D15): before `main` runs, its block's, and on the beat.
pub fn decide_program() -> i64 {
    let tasks = i64::try_from(held() / BLOCK).unwrap_or(i64::MAX);
    let decision = crate::task::Adaptive().decide(crate::task::gate::load_of(tasks, 1));
    PROGRAM.store(decision.budget, Ordering::Relaxed);
    decision.budget
}

/// **`main`'s block** (ADR-327 D14): the first decision's budget, taken
/// before `main` runs.
pub fn for_main() -> Block {
    Block::with_budget(bytes(decide_program()))
}

/// **A new task's block** (ADR-327 D15): the budget of the decision that
/// admitted the task, else the program's. The task keeps it until it ends.
pub fn for_a_task() -> Block {
    let next = NEXT.with(|next| next.replace(-1));
    if next >= 0 {
        return Block::with_budget(bytes(next));
    }
    let beat = BEAT.with(|beat| {
        let now = beat.get().wrapping_add(1);
        beat.set(now);
        now
    });
    if beat.is_multiple_of(BEAT_EVERY) {
        decide_program();
    }
    Block::with_budget(bytes(PROGRAM.load(Ordering::Relaxed)))
}

/// **A map whose table may be refused**: `std`'s own map
/// ([`crate::map::HashMap`]), whose table is placed by its handle and grows
/// in a block only through `try_reserve`.
pub type Map<K, V, S> = crate::map::HashMap<K, V, S>;

/// **The block types under the names a program's Rust writes** (#490,
/// staged): `Vec` is [`List`], `String` is [`Text`], `vec![…]` builds a
/// [`List`] and `format!(…)` a [`Text`], so a program lowers to the block
/// types where its file imports these names (`lowers_to_block_types` in
/// `tools/emit_needs.nika`, the one place that decides it).
pub mod names {
    pub use super::CopyOwned;
    pub use super::List as Vec;
    pub use super::Text as String;
    pub use super::ToText;
    pub use crate::__nikaia_list as vec;
    pub use crate::__nikaia_text as format;
}

/// **The text form of a value as a block's text** (#490): what
/// `ToString::to_string` is in the compiler's own Rust, which is on the
/// block types (`names`), where `to_string` would answer Rust's `String`.
/// Outside every block (D9).
pub trait ToText {
    /// The value written with `Display`, into a new [`Text`].
    fn to_text(&self) -> Text;
}

impl<T: std::fmt::Display + ?Sized> ToText for T {
    fn to_text(&self) -> Text {
        use std::fmt::Write as _;
        let mut text = Text::new();
        let _ = write!(text, "{self}");
        text
    }
}

/// **A copy of a lent list or text that is a block's own** (#490): what
/// `.clone()` of a `&[T]` or a `&str` is on the block types, where Rust's
/// `to_owned` would answer a `Vec` or a `String`; of any other value, its
/// clone, as `to_owned` is. Outside every block (D9)
/// until the emitter places sites.
pub trait CopyOwned {
    /// The owned value: a [`List`] or a [`Text`].
    type Owned;
    /// The copy.
    fn copy_owned(&self) -> Self::Owned;
}

impl<T: Clone> CopyOwned for T {
    type Owned = T;
    fn copy_owned(&self) -> T {
        self.clone()
    }
}

impl<T: Clone> CopyOwned for [T] {
    type Owned = List<T>;
    fn copy_owned(&self) -> List<T> {
        List::from(self)
    }
}

impl CopyOwned for str {
    type Owned = Text;
    fn copy_owned(&self) -> Text {
        Text::from(self)
    }
}

/// `vec![…]` for a [`List`]: empty, `n` clones of one item, or the items
/// listed, all outside every block (D9) until the emitter places sites.
#[macro_export]
#[doc(hidden)]
macro_rules! __nikaia_list {
    () => {
        $crate::block::List::new()
    };
    ($item:expr; $n:expr) => {
        $crate::block::List::from_elem($item, $n)
    };
    ($($item:expr),+ $(,)?) => {
        $crate::block::List::from_array([$($item),+])
    };
}

/// `format!(…)` for a [`Text`], built with no `String` between ([`format`]),
/// outside every block (D9) until the emitter places sites.
#[macro_export]
#[doc(hidden)]
macro_rules! __nikaia_text {
    // In a block of its own, as `std`'s `format!` is: the arguments
    // `format_args!` borrows end there, so a `format!` handed to a call that
    // is awaited keeps no `!Send` temporary across the await.
    ($($part:tt)*) => {{
        let text = $crate::block::format($crate::block::Place::Outside, ::std::format_args!($($part)*));
        text
    }};
}

/// **What a value holds on the heap** (ADR-327 D9): the walk a hand-over
/// measures it with, to move its deep size from the giver's account to the
/// receiver's. The emitter writes it for every declared struct and enum; a
/// closure and a Rust-owned type count their own size only, which holds
/// nothing beyond the value itself (decision (a), #1543).
pub trait DeepSize {
    /// The bytes the value holds beyond its own size.
    fn heap(&self) -> usize;
}

/// **A value's deep size as it crosses** (D9): what it holds on the heap.
/// Its own bytes move with it, from one frame or slot to another, and were
/// never counted outside: a closure or a Rust-owned type has no deep size
/// beyond its own.
pub fn deep_size<T: DeepSize>(value: &T) -> usize {
    value.heap()
}

/// **A value that does not fit where it is handed** (ADR-327 D9, decision
/// (c)): the receiving task panics at the take and ends `task::Crashed`; a
/// shared value is poisoned (D6). It unwinds without the panic hook, as
/// `OverBudget` does: the sentence is the payload `task::Crashed` carries,
/// not a line on stderr naming a runtime file and thread.
#[cold]
#[inline(never)]
pub fn does_not_fit(size: usize) -> ! {
    std::panic::resume_unwind(Box::new(format!(
        "a value of {size} bytes handed over does not fit the receiver's memory budget"
    )))
}

/// **Take a value of `size` bytes over** (D9) into the running task or
/// shared value: counted, and a panic where it does not fit.
#[inline]
#[track_caller]
pub fn received(size: usize) {
    if !take_over(size) {
        does_not_fit(size)
    }
}

/// **A list handed to a Rust owner leaves the giver's account** at the call
/// (D8, D9; #1541): the owner keeps it, and what it frees later counts
/// against whoever runs. Its elements are Rust's to measure, so only its memory
/// moves. What Rust hands back was allocated by [`Counting`] in the task
/// that called it, and is in that task's account already.
#[inline]
pub fn handed_list<T>(list: impl Into<Vec<T>>) -> Vec<T> {
    let list = list.into();
    hand_over(list.capacity() * std::mem::size_of::<T>());
    list
}

/// [`handed_list`], for a text.
#[inline]
pub fn handed_text(text: impl Into<String>) -> String {
    let text = text.into();
    hand_over(text.capacity());
    text
}

macro_rules! shallow_only {
    ($($t:ty),* $(,)?) => {
        $(impl DeepSize for $t {
            #[inline]
            fn heap(&self) -> usize {
                0
            }
        })*
    };
}

shallow_only!(
    (),
    bool,
    char,
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
    std::time::Duration,
);

impl DeepSize for String {
    fn heap(&self) -> usize {
        self.capacity()
    }
}

impl DeepSize for Text {
    fn heap(&self) -> usize {
        if self.is_outside() {
            self.capacity()
        } else {
            0
        }
    }
}

impl<T: DeepSize> DeepSize for Vec<T> {
    fn heap(&self) -> usize {
        self.capacity() * std::mem::size_of::<T>() + self.iter().map(DeepSize::heap).sum::<usize>()
    }
}

impl<T: DeepSize> DeepSize for List<T> {
    fn heap(&self) -> usize {
        let own = if self.is_outside() {
            self.capacity() * std::mem::size_of::<T>()
        } else {
            0
        };
        own + self.iter().map(DeepSize::heap).sum::<usize>()
    }
}

impl<T: DeepSize> DeepSize for std::collections::VecDeque<T> {
    fn heap(&self) -> usize {
        self.capacity() * std::mem::size_of::<T>() + self.iter().map(DeepSize::heap).sum::<usize>()
    }
}

impl<T: DeepSize> DeepSize for Option<T> {
    fn heap(&self) -> usize {
        self.as_ref().map_or(0, DeepSize::heap)
    }
}

impl<T: DeepSize, E: DeepSize> DeepSize for Result<T, E> {
    fn heap(&self) -> usize {
        match self {
            Ok(value) => value.heap(),
            Err(error) => error.heap(),
        }
    }
}

impl<T: DeepSize + ?Sized> DeepSize for Box<T> {
    fn heap(&self) -> usize {
        std::mem::size_of_val::<T>(self) + T::heap(self)
    }
}

/// **A view has no deep size**: what it shows is its owner's.
impl<T: ?Sized> DeepSize for &T {
    fn heap(&self) -> usize {
        0
    }
}

/// **A shared value has no deep size where it crosses**: what it holds stays
/// counted where it was built, and another holder keeps it alive.
impl<T: ?Sized> DeepSize for std::sync::Arc<T> {
    fn heap(&self) -> usize {
        0
    }
}

impl<T: ?Sized> DeepSize for std::rc::Rc<T> {
    fn heap(&self) -> usize {
        0
    }
}

/// **A closure counts its own size** (decision (a)): its captures stay
/// counted where it was made.
impl<F: ?Sized> DeepSize for crate::func::Kept<F> {
    fn heap(&self) -> usize {
        0
    }
}

impl<K: DeepSize, V: DeepSize, S> DeepSize for std::collections::HashMap<K, V, S> {
    fn heap(&self) -> usize {
        // The table's slots and a control byte each, which is what
        // `hashbrown` allocates, near enough for a budget.
        self.capacity() * (std::mem::size_of::<(K, V)>() + 1)
            + self.iter().map(|(k, v)| k.heap() + v.heap()).sum::<usize>()
    }
}

impl<K: DeepSize, S> DeepSize for std::collections::HashSet<K, S> {
    fn heap(&self) -> usize {
        self.capacity() * (std::mem::size_of::<K>() + 1)
            + self.iter().map(DeepSize::heap).sum::<usize>()
    }
}

impl<K: DeepSize, V: DeepSize> DeepSize for std::collections::BTreeMap<K, V> {
    fn heap(&self) -> usize {
        self.iter()
            .map(|(k, v)| std::mem::size_of::<(K, V)>() + k.heap() + v.heap())
            .sum()
    }
}

impl<K: DeepSize> DeepSize for std::collections::BTreeSet<K> {
    fn heap(&self) -> usize {
        self.iter()
            .map(|k| std::mem::size_of::<K>() + k.heap())
            .sum()
    }
}

macro_rules! deep_size_tuples {
    ($(($($name:ident),+)),* $(,)?) => {
        $(impl<$($name: DeepSize),+> DeepSize for ($($name,)+) {
            #[allow(non_snake_case)]
            fn heap(&self) -> usize {
                let ($($name,)+) = self;
                0 $(+ $name.heap())+
            }
        })*
    };
}

deep_size_tuples!(
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, G),
);

/// **The field walk the emitter writes** (decision (a)): a field whose type
/// has [`DeepSize`] is walked, any other - a Rust-owned type - counts its own
/// size only. Picked by method resolution on the field's concrete type.
pub struct Sizing<'a, T: ?Sized>(pub &'a T);

/// The walk, where the field's type has [`DeepSize`].
pub trait DeepSizeKnown {
    fn deep_size(&self) -> usize;
}

impl<T: DeepSize + ?Sized> DeepSizeKnown for Sizing<'_, T> {
    #[inline]
    fn deep_size(&self) -> usize {
        self.0.heap()
    }
}

/// Its own size only, where the field's type has no [`DeepSize`].
pub trait DeepSizeOwn {
    fn deep_size(&self) -> usize;
}

impl<T: ?Sized> DeepSizeOwn for &Sizing<'_, T> {
    #[inline]
    fn deep_size(&self) -> usize {
        0
    }
}

/// `nikaia_std::__nikaia_deep_size!(self.field)`: what a field holds on the
/// heap, as the emitter's `DeepSize` impls write it.
#[doc(hidden)]
#[macro_export]
macro_rules! __nikaia_deep_size {
    ($e:expr) => {{
        #[allow(unused_imports)]
        use $crate::block::{DeepSizeKnown as _, DeepSizeOwn as _};
        (&$crate::block::Sizing(&$e)).deep_size()
    }};
}
