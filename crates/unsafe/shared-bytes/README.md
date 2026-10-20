# shared-bytes

A shared, immutable run of bytes that carries a counted handle on whatever
owns its memory (Nikaia's ADR-373 D1): a `Vec<u8>` handed over, a mapped
file, any value that lends its bytes through `AsRef<[u8]>`, or static data,
which needs no handle. A `Shared` is a pointer, a length and the handle, so a
read never looks at the owner; a clone and a cut are a count, and the owner
is dropped when the last holder goes. No dependencies.

```rust
use shared_bytes::Shared;

let whole = Shared::from_owner(b"hello world".to_vec()); // nothing copied
let word = whole.cut(6..);
drop(whole);
assert_eq!(&*word, b"world"); // the vector lives as long as the cut
```

The owner is an `Arc` of a trait object rather than an enum of the owners
there are: reads never touch it, so it decides only what a clone and a drop
cost, and that is one count whatever owns the bytes.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `as_slice` | `slice::from_raw_parts(ptr, len)` | `ptr` and `len` were taken from a `&[u8]` that is either `'static` (`from_static`) or lent by the owner this value holds a count on (`from_arc`, `cut` narrows it with the slice's own bounds check). The owner sits in an `Arc` and never moves; while this value holds a count nobody can get `&mut` to it (`Arc::get_mut` and `try_unwrap` need the only count), so the memory it lent through `&self` stays valid and unchanged. Safe code cannot lend memory through `AsRef::as_ref(&self)` that it can free or change without `&mut` (a `RefCell` or `Mutex` lends only through a guard), which is the argument the `bytes` crate's `from_owner` makes |
| `Send`, `Sync` | `unsafe impl` | the pointer is only read, and only for as long as the owner lives; the owner is `Send + Sync` by its type, and static data is `Sync` |

## How it is checked

Its own workspace, so its lock file and its checks are its own:

```sh
cargo test && cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo +nightly miri test --all-features                       # Stacked Borrows
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test --all-features
```

`scripts/check-unsafe-crates.sh` runs these for every crate under
`crates/unsafe/`, and CI runs that script.
