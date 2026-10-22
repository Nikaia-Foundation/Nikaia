# byte-blocks

Which bytes of a 64-byte block, or of half of one, belong to a set, one bit a
byte, at the vector width the build's CPU level allows. No dependencies, `no_std`.

```rust
let tables = byte_blocks::Tables::new([1 << b';' | 1 << b'\n', 0, 0, 0]);
let mut block = [b'x'; 64];
block[3] = b';';
block[9] = b'\n';
assert_eq!(byte_blocks::in_tables(&block, &tables), 1 << 3 | 1 << 9);
assert_eq!(byte_blocks::any_of(&block, [b';', b'\n']), 1 << 3 | 1 << 9);
```

The caller is `nikaia-std`'s `bytes::ByteSet`
([ADR-375](../../../docs/specification/adr/adr-375.md)), which reads a buffer a
block at a time and copies the last, short block into one of zeros. The width
is the build's (`cfg`), never decided at run time: AVX2 at `x86-64-v3`, SSE2
(and SSSE3's byte shuffle where the build has it) at x86-64's baseline, eight
bytes a word on any other target.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `any_of`, `in_tables` | a call of a `#[target_feature]` function | the `cfg` around the call says the build enables that feature for every function; `core::arch` asks for `unsafe` all the same |
| `avx2::bits`, `sse2::bits` | `_mm256_loadu_si256`, `_mm_loadu_si128` | each reads one part from `as_chunks::<32>()` or `as_chunks::<16>()` of the block, so exactly that part's 32 or 16 bytes; the loads take any alignment |
| `high_bits` | a call of a `#[target_feature]` function | as for `any_of`: AVX2 by the `cfg`, SSE2 as x86-64's baseline |
| `avx2::bits` (from `avx2::high`), `sse2_high` | `_mm256_loadu_si256`, `_mm_loadu_si128` | each reads one part from `as_chunks::<32>()` or `as_chunks::<16>()` of `lanes`, exactly that part's bytes, any alignment |
| `avx2::table`, `sse2::tables` | `_mm_loadu_si128` | reads the 16 bytes of a `&[u8; 16]`, any alignment |

## How it is checked

Its own workspace, so its lock file and its checks are its own:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo +nightly miri test                                     # Stacked Borrows
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test
```

`scripts/check-unsafe-crates.sh` runs these for every crate under
`crates/unsafe/`, and CI runs that script. The AVX2 path is the build's at
`x86-64-v3`: `RUSTFLAGS="-C target-cpu=x86-64-v3" cargo test`.
