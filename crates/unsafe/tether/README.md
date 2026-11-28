# tether

Views that outlive the scope that read their buffer, without copying the text.

| type | what it is |
| :--- | :--- |
| `Keep` | an append-only home for buffers; a buffer stays at its address until the `Keep` is dropped, one of up to four words (owned text) in the keep's own slot |
| `Keep::put_viewed` / `Keep::view` | a buffer views of text are cut from (`Viewed`), and a view found again in it **by address** — or, where it points into no buffer of the keep, a copy the keep owns |
| `Place` / `Keep::put_viewed_at` / `Keep::view_in` | where a buffer lives in its keep, handed out when it is put there, and a view checked against **that one buffer** by one comparison (Nikaia's ADR-tether-v1 D15); not there, a program copies it and a run under `nikaia test` panics: the plan was the compiler's |
| `Held` | one view of text carrying its own handle on its buffer; compared, hashed and printed as the text; a **safe** constructor |
| `Holding<V, N>` / `Views` | a struct of views beside the `N` keeps it points into, with a **safe** constructor |
| `Keeps` / `Rebase` / `Hold` | how a value made *outside* the constructor is carried in: each view found again in its keep |
| `Spare<B>` / `Keep::returning` | a buffer a keep hands back when it is dropped, at the last view of it, for the next read of the loop to read into (Nikaia's #2859, Perceus's reuse) |
| `Dangling<T>` | storage for a value whose views are stretched, beside the handle that keeps their buffer — what `Held` and `Holding` keep it in |

```rust
use std::sync::Arc;
use tether::{Holding, Keep, Views};

struct Record<'a> { name: &'a str, size: usize }
enum RecordViews {}
impl Views for RecordViews {
    type Of<'a> = Record<'a>;
    fn shorten<'long: 's, 's>(x: &'s Record<'long>) -> &'s Record<'s> { x }
}

let keep = Arc::new(Keep::new());
let text = keep.put_viewed(String::from("  oslo  "));
// Made the ordinary way, borrowing `text` ...
let name = text.trim();
// ... and carried in: `k.view` finds `name` in the keep by its address.
let held: Holding<RecordViews> =
    Holding::new([Arc::clone(&keep)], |k| Record { name: k.view(name), size: text.len() });
drop(keep);
assert_eq!(held.get().name, "oslo");
```

No dependencies. Written for the Nikaia compiler's generated code, usable by
anything that needs the same.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `Dangling::get`, `Dangling::drop` | `MaybeUninit::assume_init_ref` / `assume_init_drop` | the value is written in `new` and taken apart only in `drop`, once |
| `Keep::put_at`, `Keep::put_viewed_at` | `&*at` from `Arc::as_ptr` | the `Arc` is stored in a slot set once and never emptied until the `Keep` is dropped, so the buffer stays alive and in place; the reference is tied to the borrow of the `Keep` |
| `Keep::put_viewed_at`, `Inline`, `drop_inline` | a buffer of up to four words written into the slot's `MaybeUninit`, read back as `&B`, dropped through the function made for its type | `fits` checks size and alignment; the buffer is moved in before any view of it exists, the slot never moves while the `Keep` is borrowed and is never written again, and `Inline::drop` runs once, with the slot. So a keep of one text is one allocation (Nikaia's #2843) |
| `Keep::drop`, `Spare::give` (`Returns`) | the first slot's entry read with `assume_init_mut`; its inline buffer read out with `ptr::read`, its `drop` set to `moved_out` | the entry is read only after `ready`, which is set after it was written; the buffer is moved out only where the slot's type id is the spare's, once, and the slot then drops nothing, so it is dropped once, by the spare. No view of it is left: every view borrows the `Keep` or holds an `Arc` of it, and this runs as the last one goes |
| `Slot` (`Send`, `Sync`, `write`, `publish`, `bytes`) | a hand-made `OnceLock`: the entry and its bytes written through `UnsafeCell`, read back with `from_raw_parts` | the place is claimed by one `fetch_add`, so one put alone writes a slot, once, before `ready` is stored with `Release`; a reader looks only after seeing `ready` with `Acquire`; the bytes are the buffer's own, read where it lives, and it is neither moved nor written until the slot is dropped |
| `within` (inside `find` and `view_in`) | `str::from_utf8_unchecked` | the bytes are found **by address**: they are the very bytes the `&str` asked about is made of, so they are UTF-8 |
| `Held::new`, `Held::in_place`, `Held::near` | `&str` → `&'static str` | the view came from `Keep::find`, `Keep::view`, `Keep::view_in` or `Keep::view_near`, which hand back nothing but a view of that keep's own buffers, and the keep stored beside it keeps them alive |
| `Holding::new`, `Holding::clone` | `transmute_copy` of `V::Of<'_>` to `V::Of<'static>` | the closure must work for every lifetime and is handed only the keeps, so the struct can point only into them or at `'static` data; it is private, dropped before the keeps, and handed out only through `get`, which shortens it back |

**Two things the compiler checks for the caller**, so neither is a comment:

* `Views::shorten`'s body is `x`, which compiles only where the struct is
  **covariant** in its lifetime. A struct that could be written through
  (`Cell<&'a str>`, `&'a mut`) is refused there.
* `Holding::new`'s closure is `for<'a> FnOnce(Keeps<'a, N>) -> V::Of<'a>`, so a
  view of anything but the keeps (a local, a captured borrow) is refused. A
  view made outside comes in only through `Keeps::view`, which checks where it
  points.

**What is no longer here: `hold` and `forever`.** Two `unsafe fn`s whose
caller promised what a view pointed into: `hold`, that it pointed into the
keep, and `forever`, that everything derived from a `Keep` stretched to
`'static` was dropped before the last clone of its `Arc`. `Held::new` and
`Holding::new` ask the keep instead, so the promise is checked rather than
trusted, and generated code writes no `unsafe` for a held view or for a value a
task takes with it (Nikaia's ADR-218 D4, #84).

**Why `Dangling`.** A struct holding a `&'static str` promises that the text is
valid wherever the struct goes; passed by value into a function, the reference
is taken to be valid for the whole call. `Held` and `Holding` drop their value *before* the buffer, possibly inside
such a call, so the promise must not be made: the value lives in a
`MaybeUninit`, which makes none. Miri found both this and the `Box` in
`Keep::put` (moving a `Box` invalidates references into it; an `Arc` does not).

## How it is checked

It is its own workspace, so its lock file, its checks and the toolchain for
them are its own:

```sh
cd crates/unsafe/tether
cargo test                                                   # stable
cargo clippy --all-targets -- -D warnings
cargo +nightly miri test                                     # Stacked Borrows
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test       # Tree Borrows
```

`scripts/check-unsafe-crates.sh` runs all four for every crate under
`crates/unsafe/`, and CI runs that script.
