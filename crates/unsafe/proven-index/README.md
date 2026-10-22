# proven-index

An element of a slice read or written, or a run of it cut or read as an array,
without the bounds check, at a position the caller has proved is inside; and a
whole number's `+`, `-` or `*` without the overflow check, where the caller has
proved it stays inside its type. No dependencies, `no_std`.

```rust
let items = [10, 20, 30];
for at in 0..items.len() {
    // SAFETY: `at` runs over the length.
    let item = unsafe { proven_index::read(&items, at) };
    assert_eq!(*item, items[at]);
}
```

The caller is the code the Nikaia compiler generates, at an index its
`--optimization=remove-bounds-checks` pass proved inside: by a loop over the
list's own length whose body cannot change that length (`basic`), or by a
certificate of the solver that its checker accepted (`aggressive`) -
[ADR-306](../../../docs/specification/adr/adr-306.md). The argument for each
call is the proof; this crate holds the one operation the proof licenses.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `read` | `get_unchecked` | the caller guarantees `at < items.len()`; a debug build asserts it |
| `cut` | `get_unchecked(start..end)` | the caller guarantees `start <= end <= items.len()`; a debug build asserts it |
| `array` | a `&[T; N]` cast from the pointer at `at` | the caller guarantees `at + N <= items.len()`, and `[T; N]` is `N` consecutive `T`; a debug build asserts it |
| `slot` | `get_unchecked_mut` | the same guarantee, through `&mut` |
| `Whole::unchecked`, through `add`, `sub`, `mul` | `unchecked_add`, `unchecked_sub`, `unchecked_mul` | the caller guarantees the operation does not overflow its type; a debug build asserts it with `checked_*` |
| `write` | a store through `slot` | `slot`'s guarantee; the old element is dropped by the assignment, as `items[at] = value` drops it |

## How it is checked

Its own workspace, so its lock file and its checks are its own:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo +nightly miri test                                     # Stacked Borrows
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test
```

`scripts/check-unsafe-crates.sh` runs these for every crate under
`crates/unsafe/`, and CI runs that script.
