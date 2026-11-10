# task-block

A task's memory block ([ADR-327](../../../docs/specification/adr/adr-327.md)
D4, D5, D10, D12; #490 step 3): 16 pages of 4 KiB, each counting what it
holds, and the lists and text whose growth the block may refuse. One
dependency, `allocator-api2` (`Vec<T, A>` on stable Rust); the features
`collect`, `tether` and `rayon` add the traits of `winnow-grammar`, `tether`
and `rayon` that `nikaia-std` asks of a list or a text.

```rust
use task_block::{enter, list, push, Block, List, Place};

let block = Block::default();       // a task's, taken when it first allocates
let _in = enter(&block);            // in place while the task is polled
let mut xs: List<i64> = list(Place::Block);
push(&mut xs, 7);                   // a full block ends the task, here
```

* **`Place`** is a list's handle: `Block`, the running task's block (outside
  where no task runs), or `Outside`, `std`'s global allocator. Where a value
  is freed is decided by its address: inside the running task's block it goes
  back to the block, anywhere else to the global allocator.
* **A full block refuses** (`AllocError`); `push`, `reserve` and `extend` end
  the task in the cold `full`, outside the allocator (D6). A plain
  `Vec::push` on a refusal would abort the process, so every growth goes
  through `try_reserve`.
* **`extend` copies a run with one `copy_nonoverlapping`**, where
  `allocator-api2`'s `extend_from_slice` clones one element at a time.
* **`Text` is valid UTF-8 in a `List<u8>`**, handed out as a `str` without
  being checked again: it only grows by whole `str`s.
* **`List` and `Text` are the ground of Nikaia's `List` and `String`** (D4):
  a newtype, since the orphan rule allows no `FromIterator` on
  `allocator-api2`'s `Vec`; `.collect()`, `Clone` and `Default` build
  outside (D9), `from_iter_in`, `clone_in` and `new_in` where told; hashes,
  order and printing are the slice's and the `str`'s, so a map keyed by them
  keeps `Vec`'s and `String`'s order. `text::{to_uppercase, to_lowercase,
  replace, replacen, repeat, join, format}` build the allocating text
  methods' result where the handle says, with no `String` between.
* **Toward Rust they cross as they are** (D8): `From`/`Into` between `List`
  and `Vec`, `Text` and `String` hand over the same memory where it is
  outside the running task's block; only a value in the block is copied
  out.
* **`Housed<T>` is a shared value's block** (D5, D6): a value with a block of
  its own, put in place by `inside` for a door, its construction and its
  rebuild, and around its drop. What a door places is the value's, not the
  task's; a full one ends the door in `full`, which the lock reads as a
  poison. What leaves the door is built outside (`Clone`, D9).
* **A block value never leaves its task** (D9): the caller places in the
  block only what cannot outlive the task, and frees it while the task is
  polled. The caller is `nikaia-std` (`src/task.rs` puts a task's block in
  place around each poll and around the drop of its body).

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `impl Send for Block` | a block's raw pointer crosses threads with its task | the block's memory is reached only through `CURRENT`, while the task that owns the block is polled; between polls nothing points into it |
| `Block::take`, `take_slow`, `run` | `start.add(at)` | `at + size` is checked to lie inside the page, or the run's pages are free and inside the block's `BLOCK` bytes |
| `Block::first` | `std::alloc::alloc(MEMORY)` | `MEMORY` has a non-zero size; a null answer is a refusal |
| `Block::grow` | `copy_nonoverlapping(ptr, moved, old)` | both are live cuts of at least `old` bytes, and two live cuts never share a byte: a cut's bytes return to a page only when its count frees the page |
| `Drop for Block`, `Drop for Spares` | `dealloc(memory, MEMORY)` | `memory` came from `alloc(MEMORY)` and nothing points into it any more: the task is over (its `Entered` is gone), or it is a spare |
| `current` | `CURRENT`'s pointer as `&Block` | it is set only by `enter` from a live `&Block` and reset when that `Entered` is dropped, which the block's owner holds for the poll |
| `Drop for Housed` | `ManuallyDrop::drop(&mut self.value)` | the value is dropped once, in the `Housed`'s own drop, with its block in place, and nothing reads it after; the block is dropped after it, as a field |
| `answered` | `NonNull::new_unchecked` | a block's cut is `start.add(at)` of a non-null `start` |
| `impl Allocator for Place` | the allocator's contract | a block cut stays valid until it is given back or the task ends, and no block cut outlives its task (D9); a pointer not inside the running block is the global allocator's, which answers for it |
| `Place::deallocate`, `grow` | `Global.deallocate`, `Global.grow` | reached only for a pointer not in the running task's block, so one `Global` allocated; the caller keeps the contract's layout |
| `List::extend_copied` (`extend`) | `copy_nonoverlapping` into spare capacity, `set_len` | `reserve` left room for the run, `T: Copy` has nothing to drop, and a list and the run it is extended from are two values |
| `List::set_len` | `Vec::set_len` | an `unsafe fn` with `Vec::set_len`'s contract, handed to its caller |
| `From<Vec<T>> for List<T>` | `Vec::from_raw_parts_in(.., Place::Outside)` | `std`'s `Vec` allocated with the global allocator (or not at all, for no room or a zero-sized `T`, which both `Vec`s treat alike); `Place::Outside` frees with the same allocator, and the `std` `Vec` is never dropped |
| `From<List<T>> for Vec<T>` | `Vec::from_raw_parts`, or a move out of the block | outside the running block (`is_outside`) the memory is the global allocator's, which `std`'s `Vec` frees; a block list's elements are moved with `copy_nonoverlapping` into a new `Vec` with room for them, and the block list is rebuilt with length 0, so it frees only its memory |
| `Text::as_str`, `as_mut_str` | `str::from_utf8_unchecked` | every append is a whole `str` (or a `char` encoded as one), and `make_ascii_lowercase` changes only ASCII bytes; a `&mut str` can only be changed into UTF-8, and `insert_str`, `remove`, `truncate`, `pop` and `split_off` cut only at character boundaries |
| `From<Text> for String` | `String::from_utf8_unchecked` | the bytes are the text's, UTF-8 |

## How it is checked

Its own workspace, so its lock file and its checks are its own:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo +nightly miri test                                     # Stacked Borrows
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test
```

The tests run the block's paths under both of Miri's aliasing models: a list
outside any task, a cut in the block and one outside, a megabyte allocated a
kilobyte at a time under the 64 KiB block (D10), runs of pages given back, a
full block that refuses and a freed one that takes again, growth in place and
by a move, a task polled inside another, a block reused by the next task, an
alignment past a page, and text.
