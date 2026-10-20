# low-bits

A word with only its lowest bits kept, as one BMI2 `bzhi` where the build's
CPU level has it. No dependencies, `no_std`.

```rust
// with RUSTFLAGS="-C target-cpu=x86-64-v3"
assert_eq!(low_bits::u64(0xabcd, 8), 0xcd);
assert_eq!(low_bits::u64(0xabcd, 200), 0xabcd);
assert_eq!(low_bits::u32(0xabcd, 0), 0);
```

The caller is `nikaia-std`'s `num::LowBits`, `x.low_bits(n)` (Part I,
[ADR-371](../../../docs/specification/adr/adr-371.md) D2), which saturates
its count to `0..=255` and calls this where `cfg` says the build has BMI2
(`--cpu=x86-64-v3`); everywhere else it keeps its own portable form (a mask
and a select). The functions exist only in a build with BMI2: nothing decides
at run time.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `u64`, `u32` | a call of the `#[target_feature(enable = "bmi2")]` functions `bzhi64`, `bzhi32` | the `cfg` around both says the build enables BMI2 for every function; `core::arch` asks for `unsafe` all the same |
| the test `bzhi_keeps_the_lowest_bits` | the same calls | the test returns first where `is_x86_feature_detected!("bmi2")` is false |

`_bzhi_u64` and `_bzhi_u32` touch no memory: the only condition is the
instruction's presence.

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
