# text-key

Whether a key type is text, decided per type when compiling, and the key's
bytes where it is. No dependencies, `no_std` with `alloc`.

```rust
assert_eq!(text_key::bytes(&"Hamburg"), Some(&b"Hamburg"[..]));
assert_eq!(text_key::bytes(&String::from("x")), Some(&b"x"[..]));
assert_eq!(text_key::bytes(&7_i64), None);
assert!(text_key::same::<&'static str, &str>());
```

The caller is `nikaia-std`'s map (`src/map.rs`,
[ADR-296](../../../docs/specification/adr/adr-296.md) D42): one type for every
key, which hashes and compares a text key (`str`, `&str`, `String`,
`&String`, `Box<str>`, `Rc<str>`, `Arc<str>`, `Cow<str>`) a word at a time
instead of through `str`'s `==`, a call of `memcmp`. Rust has no
specialisation, so the answer is a comparison of two type ids, constants once
the call is inlined. The type id of a type that may hold a lifetime is taken
as `dtolnay`'s `typeid` crate takes it.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `of` | a `&dyn NonStaticAny` transmuted to `&'static dyn NonStaticAny` | only the trait object's lifetime bound changes; the object is a `PhantomData` (no value), and the one call made through it returns a `TypeId`, which holds no reference |
| `bytes`, `str` | `transmute_copy` of `&T` to `&str` | reached only where `T`'s id is `str`'s, so `&T` is `&str`, metadata included |
| `bytes`, the sized types | `*const T` read as `*const U` for `U` one of `&str`, `String`, `&String`, `Box<str>`, `Rc<str>`, `Arc<str>`, `Cow<str>` | reached only where `T`'s id is `U`'s: the same type up to a lifetime inside it, which changes no layout; the bytes handed back are borrowed for the borrow of `x`, which the referent outlives |

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
