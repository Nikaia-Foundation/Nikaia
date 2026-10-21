# crc32c

One CRC-32C step over a 64-bit word, as one SSE 4.2 `crc32` where the
build's CPU level has it. No dependencies, `no_std`.

```rust
// with RUSTFLAGS="-C target-cpu=x86-64-v3"
let h = crc32c::u64(0, 0x0123_4567_89ab_cdef);
```

The caller is `nikaia-std`'s `hash::crc32c(crc, word)`
([ADR-371](../../../docs/specification/adr/adr-371.md) D1, D5), which calls
this where `cfg` says the build has SSE 4.2 (`--cpu=x86-64-v3`); everywhere
else it computes the same value with a table. The function exists only in a
build with SSE 4.2: nothing decides at run time.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `u64` | a call of the `#[target_feature(enable = "sse4.2")]` function `step` | the `cfg` around it says the build enables SSE 4.2 for every function; `core::arch` asks for `unsafe` all the same |
| the test `the_instruction_is_the_definition` | the same call | the test returns first where `is_x86_feature_detected!("sse4.2")` is false |

`_mm_crc32_u64` touches no memory: the only condition is the instruction's
presence.

## How it is checked

Its own workspace, so its lock file and its checks are its own:

```sh
cargo test
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo test   # the build's own path
cargo clippy --all-targets -- -D warnings
cargo +nightly miri test                                     # Stacked Borrows
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test
```

`scripts/check-unsafe-crates.sh` runs these (but the `x86-64-v3` one) for
every crate under `crates/unsafe/`, and CI runs that script.
