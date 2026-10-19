# The One Billion Row Challenge: Nikaia against Rust

[`examples/1brc.nika`](../../examples/1brc.nika) describes the whole input file
as a grammar and aggregates it with a fold. This bench measures it, and the
program a Nikaia programmer writes first, against the same aggregation written
in Rust:

| program | what it is |
|---|---|
| [`naive.rs`](src/bin/naive.rs) | what a competent Rust programmer writes first: `read_to_string`, `str::lines`, `split_once`, a `HashMap` with the default hasher |
| [`naive.nika`](naive.nika) | the same in Nikaia: `fs::read_to_string`, `lines`, `find` and `split_at` at the `;`, the temperature read digit by digit as `naive.rs` reads it, `std`'s `HashMap` (`hashbrown`'s table, with Fx hashing) |
| [`tuned.rs`](src/bin/tuned.rs) | what a 1BRC entry looks like before it is parallelised: `mmap` (`unsafe`), raw bytes with **no UTF-8 validation**, one `memchr` searcher over the whole file, the separator found by walking back from the line's end, integer temperatures, a word-at-a-time hash |
| [`examples/1brc.nika`](../../examples/1brc.nika) | a grammar over the whole file, a `par_fold`, names that are slices of a mapped file, `std`'s `HashMap` |

Each pair is compared like for like: `naive.nika` with `naive.rs`, `1brc.nika`
with `tuned.rs`. `tuned` is the comparison that counts for speed.

## How it is measured

```sh
benches/brc/brc.sh            # the table below, 1 000 000 rows
benches/brc/brc.sh 200000     # quicker
```

* **Counts, not a clock.** Instructions and mispredicted branches per row,
  counted by callgrind (`--branch-sim=yes`) over the whole process. On a shared
  machine the same binaries swap places from one timed round to the next; a
  count does not move.
* **The same answer first.** Every program's output is compared byte for byte
  with `naive`'s, including Nikaia's at `user-parallelism = "yes"`, before
  anything is counted.
* **Release builds on both sides.** The Rust programs are `cargo build
  --release` (`opt-level = 3`, no overflow checks). Nikaia's is a project built
  with `nikaia build` at `opt-level = 3`, every `optimization` on
  (`remove-bounds-checks`, `remove-overflow-checks`, `skip-free-at-exit`), no
  debug assertions; it keeps the overflow checks the language promises. **Neither side
  uses link-time optimisation.** Each Rust program is a binary of its own: in one
  binary with the others, LLVM did not inline `tuned`'s hash-table entry and the
  count rose by 40 a row.
* **Nikaia without overflow checks** is the same project built with
  `RUSTFLAGS="-C overflow-checks=off"`, to say what the checks cost.
* **Length in tokens**, by [`scripts/tokens.nika`](../../scripts/tokens.nika), the
  same rule for both languages: identifiers, keywords, numbers, literals and
  operators count, comments and whitespace do not, `use` lines do. Each file is
  counted whole.
* **The input** is `gen`'s: 413 stations, the count the real file has, about a
  quarter of the names non-ASCII, from a fixed seed.

## The table

Nikaia 0.0.1042, rustc 1.97.0, Intel Xeon @ 2.10 GHz; callgrind, 1 000 000 rows,
per row:

| | instructions | mispredictions | tokens |
|---|---:|---:|---:|
| Rust, `naive` | 860.7 | 8.64 | 486 |
| **Nikaia**, `naive.nika` | **598.8** | **8.27** | **320** |
| Rust, `tuned` | 272.2 | 3.70 | 809 |
| **Nikaia**, `examples/1brc.nika` | **239.3** | **1.08** | **618** |
| Nikaia, overflow checks off | 236.3 | 2.05 | |

The naive programs: Nikaia's is a third shorter and runs 30 % fewer
instructions, from the same algorithm (below).

Against `tuned`, Nikaia runs 12 % fewer instructions and less than a third of
the mispredictions, from a quarter fewer tokens, with no `unsafe`, with UTF-8
and every overflow checked or proved; the same file also runs on every core.
The overflow checks cost 3.0 instructions a row: 2.0 are the accumulator's two
checks, `sum += temp` and `count += 1`, and 1.0 is LLVM laying out the grammar's
loop differently without them. The temperature's checks are proved (ADR-314)
and cost nothing in either build.

The mispredictions depend on where the code lies, the instructions do not:
callgrind's simulated predictor is indexed by address. The build without
overflow checks has 2.05, the same program as the table's row built with line
tables 1.77; in both the difference is in `memcmp` (0.68 or 0.78 a row against
0.18) and in the parsing loop, with the same instructions executed. Every build
of `1brc.nika` stays below `tuned`'s 3.70.

## Where Nikaia's instructions go

Callgrind's attribution per row for the table's Nikaia row, built again with
`debug = 1` (line tables) to have one: its instructions are the table's within
0.02 a row; its mispredictions total 1.77 (see above). Each instruction goes to
the innermost crate it was inlined from that is not `core`, `alloc` or `std`;
what `core` inlines into the generated rule is its own row.

| | instructions | mispredictions |
|---|---:|---:|
| the grammar's runtime (`winnow-grammar`, `winnow`) | 47.0 | 0.18 |
| the temperature: byte reads, digit tests and arithmetic `core` inlines into the generated rule | 36.6 | 0.26 |
| the hash table (`hashbrown`; `std`'s `HashMap` is its table, directly) | 33.8 | 0.03 |
| comparing a name with the one in the table (`memcmp`, the slice comparison) | 28.7 | 0.70 |
| finding the `;` and the line's end (`memchr`, with its SIMD intrinsics) | 28.0 | 0.00 |
| the generated rules and actions, the stats' update among them | 27.6 | 0.01 |
| hashing a name (`nikaia-std`'s byte hash) | 23.4 | 0.48 |
| the UTF-8 check (`simdutf8`) | 10.3 | 0.09 |
| the rest: start-up, the output | 4.0 | 0.02 |

`tuned` is one function after inlining, so callgrind cannot split it the same
way.

## Where `naive.nika` differs from `naive.rs`

Callgrind's attribution by function, per row:

| | `naive.nika` | `naive.rs` |
|---|---:|---:|
| finding the line's end (`memchr`) | 147.7 | 147.7 |
| finding the `;` (`CharSearcher`); `naive.rs` also finds the `.` | 163.7 | 247.6 |
| hashing a name: Fx in the table's entry, SipHash in Rust's default | in the entry | 185.4 |
| the hash table's entry | 91.9 | 75.0 |
| comparing a name with the one in the table (`memcmp`) | 39.7 | 49.8 |
| the loop: cutting, the digits, the stats | 142.5 | 117.3 |
| the UTF-8 check: `simdutf8`, `str::from_utf8` | 10.3 | 35.5 |

The same algorithm on both sides; what differs is the default each language
chose: the hasher, the UTF-8 check, and Nikaia's one-character pattern as a
`char` (#617).

## What is left to build

| what | where | issue |
|---|---|---|
| a count per row and a sum over the file, bounded by the input's size, to prove the accumulator's two overflow checks away (2.0 a row) | the prover | [#432](https://github.com/Nikaia-Foundation/Nikaia/issues/432) |
| no whole-file UTF-8 check (10.3 a row): text checked only where it is new, a mapping as bytes, maybe a type for unchecked text (a grammar over `Bytes` is ADR-366, #422) | language, `winnow-grammar`, `std` | [#664](https://github.com/Nikaia-Foundation/Nikaia/issues/664) |
| an insert-only table for a map nothing removes from; decision pending | `std` | [#434](https://github.com/Nikaia-Foundation/Nikaia/issues/434) |
| fusing a caller's literals with a called fixed-shape rule (about 7 instructions a row), against ADR 24 §8's rule locality; decision open | `winnow-grammar`, the grammar lowering | decision open |

## What this does not claim

Nothing about the 1BRC leaderboard. The entries there are SWAR over raw bytes,
hash tables sized to the key count with the keys stored inline, no UTF-8
anywhere, and every core. `tuned` is a single-threaded, readable version of that
idea and stops short of it. The question this bench answers is whether a
grammar that describes the file costs you the loop a specialist writes.

## The files

* `src/bin/naive.rs`, `src/bin/tuned.rs` - the two Rust programs, each a binary
  of its own.
* `naive.nika` - the naive Nikaia program; `examples/1brc.nika` is the other.
* `src/bin/gen.rs` - the input: `gen <rows> <file>`.
* `brc.sh` - builds everything, checks the outputs agree, counts, prints the
  table.
