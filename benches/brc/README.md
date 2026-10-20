# The One Billion Row Challenge: Nikaia against Rust

[`examples/1brc.nika`](../../examples/1brc.nika) describes the whole input file
as a grammar and aggregates it with a fold. This bench measures it, the
program a Nikaia programmer writes first, and one written for speed, against
the same aggregation written in Rust:

| program | what it is |
|---|---|
| [`naive.rs`](src/bin/naive.rs) | what a competent Rust programmer writes first: `read_to_string`, `str::lines`, `split_once`, a `HashMap` with the default hasher |
| [`naive.nika`](naive.nika) | the same in Nikaia: `fs::read_to_string`, `lines`, `find` and `split_at` at the `;`, the temperature read digit by digit as `naive.rs` reads it, `std`'s `HashMap` (`hashbrown`'s table, with Fx hashing) |
| [`ai1.rs`](src/bin/ai1.rs), AI variant 1 | written by an AI assistant at medium effort, in one request, as an optimised solution; what a 1BRC entry looks like before it is parallelised: `mmap` (`unsafe`), raw bytes with **no UTF-8 validation**, one `memchr` searcher over the whole file, the separator found by walking back from the line's end, integer temperatures, a word-at-a-time hash |
| [`ai2.rs`](src/bin/ai2.rs), AI variant 2 | written by the same AI assistant over several rounds, each measured with callgrind and improved: a SIMD compare finds the `;` (SSE2; AVX2 where the CPU level has it), a word hash of the name (crc32c where the CPU level has it), a branch-free temperature parse, an own open-addressing table, the end of the input run from a padded copy so that nothing reads past the end; `mmap` and `unsafe` throughout, **no UTF-8 validation** |
| [`examples/1brc.nika`](../../examples/1brc.nika) | a grammar over the whole file, a `par_fold`, names that are slices of a mapped file, `std`'s `HashMap` |
| [`fast.nika`](fast.nika) | the building blocks of AI variant 2 as far as today's `std` has them: the mapped file read as `Bytes`, `find_all` over a `bytes::ByteSet` of `;` alone (the line's end follows from the temperature's shape), the name's first sixteen bytes read as two words with `u64_le` as its hash and key, the temperature from one word without a branch, an own open-addressing table in a `Vec`, the last rows read from a copy with zeros behind them; no `unsafe`, each station's name checked as UTF-8 once, where it is printed. On more than one core it cuts the file with `par_pieces` and folds the pieces with `Par::fold` |

Each pair is compared like for like: `naive.nika` with `naive.rs`, `1brc.nika`
with the two AI variants, `fast.nika` with AI variant 2. They are the
comparison that counts for speed.

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
  with `naive`'s, at each setting, including Nikaia's at
  `user-parallelism = "yes"`, before anything is counted.
* **Release builds on both sides.** The Rust programs are `cargo build
  --release` (`opt-level = 3`, no overflow checks). Nikaia's is a project built
  with `nikaia build` at `opt-level = 3`, every `optimization` on
  (`remove-bounds-checks`, `remove-overflow-checks`, `skip-free-at-exit`), no
  debug assertions; it keeps the overflow checks the language promises. **Neither side
  uses link-time optimisation.**
* **Two codegen-unit settings, the same on both sides each time: 1 and 16**
  (release's default). Which unit rustc puts a function in decides whether
  LLVM inlines it into the hot loop: `hashbrown`'s `entry` on both sides, and on
  Nikaia's also the grammar runtime's `advance`. That moves a row by about 40
  instructions, and not the same way on both sides: AI variant 1 inlines the entry
  at 1 unit and not at 16, `1brc.nika` the other way round. One setting alone
  would pick the winner, so the table shows both. Making Nikaia's side not
  depend on the partition is #695. `brc.sh` sets the count per run:
  `CARGO_PROFILE_RELEASE_CODEGEN_UNITS` for the Rust programs, each setting in
  a target directory of its own, and `CARGO_PROFILE_DEV_CODEGEN_UNITS` for the
  Nikaia project (a project cannot say it yet, #692). Each Rust program is a
  binary of its own, for the same reason.
* **Nikaia without overflow checks** is the same project built with
  `RUSTFLAGS="-C overflow-checks=off"`, to say what the checks cost.
* **One core.** AI variant 2 runs on one, so `fast.nika` is counted at
  `user-parallelism = "no"`, where `task::cores()` is 1 and it scans the file
  in one loop. Its build at `"yes"`, which folds `par_pieces`, is checked for
  its output and not counted.
* **The CPU level** is `x86-64` (SSE2), the default on both sides. The rows
  marked `x86-64-v3` (AVX2, BMI2, SSE 4.2) build AI variant 2 with
  `RUSTFLAGS="-C target-cpu=x86-64-v3"` and `fast.nika` with `nikaia build
  --cpu=x86-64-v3`, at the same unit counts.
* **Length in tokens**, by [`scripts/tokens.nika`](../../scripts/tokens.nika), the
  same rule for both languages: identifiers, keywords, numbers, literals and
  operators count, comments and whitespace do not, `use` lines do. Each file is
  counted whole.
* **The input** is `gen`'s: 413 stations, the count the real file has, about a
  quarter of the names non-ASCII, from a fixed seed.

## The table

Nikaia 0.0.1158, rustc 1.97.0, Intel Xeon @ 2.10 GHz; callgrind, 1 000 000 rows,
per row:

| | instructions, 1 unit | mispredictions, 1 unit | instructions, 16 units | mispredictions, 16 units | tokens |
|---|---:|---:|---:|---:|---:|
| Rust, `naive` | 840.1 | 9.19 | 859.0 | 8.62 | 556 |
| **Nikaia**, `naive.nika` | **570.2** | **8.24** | **569.1** | **8.44** | **375** |
| Rust, AI variant 1 (`ai1`) | 270.9 | 3.70 | 312.4 | 3.61 | 879 |
| Rust, AI variant 2 (`ai2`) | 147.4 | 0.38 | 147.4 | 0.38 | 1775 |
| **Nikaia**, `examples/1brc.nika` | **273.9** | **1.12** | **231.2** | **1.12** | **642** |
| Nikaia, overflow checks off | 275.0 | 1.12 | 231.3 | 1.13 | |
| **Nikaia**, `fast.nika` | **163.4** | **0.95** | **163.6** | **0.95** | **1353** |
| Nikaia, `fast.nika`, overflow checks off | 147.6 | 0.95 | 147.8 | 0.95 | |
| Rust, AI variant 2 (`ai2`), `x86-64-v3` | 132.7 | 0.27 | 132.7 | 0.27 | |
| Nikaia, `fast.nika`, `x86-64-v3` | 151.8 | 0.95 | 152.0 | 0.95 | |

The naive programs: Nikaia's is a third shorter and runs a third fewer
instructions at both settings, from the same algorithm (below).

Against AI variant 1, the instructions are level within what the inliner
decides: Nikaia runs 26 % fewer at 16 units, AI variant 1 1 % fewer at 1 unit.
The mispredictions are a third of AI variant 1's at both settings. Nikaia's program is a
quarter fewer tokens, with UTF-8 and every overflow checked
or proved; the same file also runs on every core. The temperature's checks are
proved (ADR-314), and so are the counter's, `count += 1` and the merge's
`count += other.count`, from the input's length (ADR-367): the counter is one
`incq`. What is left is `sum += temp`, one `jo` after its `add`; the build
without it runs one branch fewer a row at both settings and the same
instructions within one, laid out differently.

The mispredictions depend on where the code lies, the instructions do not:
callgrind's simulated predictor is indexed by address. Every build of
`1brc.nika` in the table stays at 1.09 to 1.12, a third of AI variant 1's 3.6
to 3.7.

AI variant 2 runs 36 % fewer instructions than `1brc.nika` at 16 units and
46 % fewer at 1 unit, and a third of its mispredictions, in 2.8 times its
tokens, with `unsafe`, target-specific intrinsics and no UTF-8 check.

`fast.nika` runs 11 % more instructions than AI variant 2 at both unit
counts, as many within 0.3 % without its overflow checks, and 14 % more at
`x86-64-v3`; it has two and a half times AI variant 2's mispredictions. It is
24 % fewer tokens, with every index and overflow checked or proved, and a row without a `.` where the temperature's shape puts one stops
the program. Where the difference lies is below.

## Where Nikaia's instructions go

Callgrind's attribution per row for the table's Nikaia row **at 16 units**,
built again with `debug = 1` (line tables) to have one: its instructions and
mispredictions are the table's. Each instruction goes to the innermost crate it
was inlined from that is not `core`, `alloc` or `std` (by `addr2line -i`);
what `core` inlines into the generated rule is its own row.

| | instructions | mispredictions |
|---|---:|---:|
| the byte reads, cuts, digit tests and arithmetic `core` inlines into the generated rules | 42.4 | 0.26 |
| the hash table (`hashbrown`; `std`'s `HashMap` is its table, directly) | 39.3 | 0.05 |
| the grammar's runtime (`winnow-grammar`, `winnow`) | 32.0 | 0.00 |
| comparing a name with the one in the table (`memcmp`, the slice comparison) | 27.8 | 0.20 |
| finding the `;` and the line's end (`memchr`, with its SIMD intrinsics) | 27.0 | 0.00 |
| the generated rules and actions, the stats' update among them | 24.6 | 0.01 |
| hashing a name (`nikaia-std`'s byte hash) | 24.4 | 0.49 |
| the UTF-8 check (`simdutf8`) | 10.3 | 0.09 |
| the rest: start-up, the output | 3.5 | 0.03 |

The AI variants are one function after inlining, so callgrind cannot split
them the same way.

## Where `fast.nika` differs from AI variant 2

**A station's first row is `@inline(never)`** (ADR-370): `claim`, which
copies the name into the table once per station, stands out of the loop that
runs once per row. Left inline, its allocation and copy keep values alive
across the probe, and the loop stores and reloads them on the stack: 160
stack accesses in `run` against 121 without it, 7.7 instructions a row at 1
unit and 6.7 at 16. The same split without the attribute counts exactly what
the code before it did, since LLVM inlines it back; `@inline(always)` on
`slot_of`, inlined already, changes nothing. The two tables below were
counted before `claim` moved out, at 173.3 instructions a row.

Callgrind's attribution per row at 16 units, `x86-64`, by the instructions of
each building block in the hot loop (both loops are one function after
inlining, so the blocks are address ranges of it, read off the disassembly).

| | `fast.nika` | `ai2.rs` |
|---|---:|---:|
| finding the `;` and stepping to the next row | 24.0 | 19.8 |
| the name's two words, cut to its length | 27.1 | 28.0 |
| the temperature, values kept on the stack among them | 50.0 | 26.0 |
| the hash | 9.0 | 13.0 |
| the table's probe, the name compared (`memcmp` included) | 39.4 | 43.2 |
| the stats' update | 20.0 | 14.0 |
| the rest: start-up, the output | 3.8 | 3.4 |

`fast.nika` compares a name of up to sixteen bytes as its two words and calls
`memcmp` only for a longer one; AI variant 2 calls it for every row (19.7 a
row). What `fast.nika` runs more is checks and what they cost around them:
each word read tests its position (three a row, two or three branches each),
the temperature's `.` is tested once (`28 - dot` and the shift by it are
then proved, ADR-306 D18), the slice of a
long name is computed and tested every row though only a long name uses it,
the table's index is tested, and six additions test for overflow. The
values the checks keep alive are stored on the stack and read back. The mispredictions are where `find_all` runs out of
separators in a 64-byte block (0.25 a row), the name's length (`<= 8` or
not, 0.5 a row, where AI variant 2 has a `cmov`), and where the predictor
indexes by address (see the note on `1brc.nika` above).

What each check costs, at 16 units, `x86-64`: the lowered Rust of `fast.nika`
edited by hand to drop exactly that check (an unchecked read, `wrapping_*`,
`get_unchecked`), built the same way, its output compared with `naive`'s and
counted. None of them is proved today; the last column says what a proof
needs. The shift by `28 - dot` is proved from the program's guard on `dot`
(ADR-306 D18) and is no longer among them.

| check | instructions a row it costs | what proving it needs |
|---|---:|---|
| the table's index `self.slots[at]`, in the probe and in the update | 16.1 | `slots.len() == SLOTS` as `Table`'s invariant (ADR-369, not built), `at`'s range carried through the probe's turns, `slot_of`'s result below the length |
| the name's two word reads, `u64_le(pos)` and `u64_le(pos + 8)` | 15.6 | `0 <= pos < stop` at the read, from the `break` that ends the previous turn (the walk forgets it at the loop's head), and `stop + MARGIN <= buf.len()` stated at `run`'s top |
| the long name's slice `buf[pos..<pos + len]`, hoisted onto every row | 13.7 | the same facts as the word reads, a separator inside `buf`, and `len >= 0` |
| the separator's arithmetic, `sep - pos` and `+ (dot >> 3)` | not counted alone | `sep + 1` and `sep + 4` are proved from `0 <= sep < buf.len()`, which each position `find_all` yields (ADR-375 D5) and `buf` keeps across `slot_of`, which takes it `ref` (#771): together the three cost 9.1, and the two cost 2.0. `sep - pos` needs `pos >= 0` across the turns |
| `sum += v`, `count += 1` | 2.0 | not provable: the table is scanned again and merged into, so no step count bounds them |
| `8 * len`, `64 - 8 * len` and the shift by it | 0.6 | `len >= 0`, which a malformed file breaks: only a guard the program writes gives it |

With every check above dropped at once, the last row's by one guard
`0 <= len <= 100` in their place, `fast.nika` runs 132.0 instructions a row,
below AI variant 2's 147.4. What stays then guards a malformed file: the
`.` not where the shape says (the program's own guard), and the temperature's word read
past the end after a name longer than `MARGIN`.

## Where `naive.nika` differs from `naive.rs`

Callgrind's attribution per row at 16 units, by function. `naive.nika`'s loop
has the table's entry inlined into it, so there its instructions are split by
the crate they were inlined from, as above (built again with `debug = 1`, the
same count as the table's).

| | `naive.nika` | `naive.rs` |
|---|---:|---:|
| finding the line's end (`memchr`) | 147.7 | 147.7 |
| finding the `;` (`CharSearcher`); `naive.rs` also finds the `.` | 163.7 | 247.6 |
| hashing a name: Fx in Nikaia's `std`, SipHash in Rust's default | 24.4 | 185.3 |
| the hash table's entry | 44.1 | 74.8 |
| comparing a name with the one in the table (`memcmp`) | 39.7 | 49.8 |
| the loop: cutting, the digits, the stats | 136.5 | 116.3 |
| the UTF-8 check: `simdutf8`, `str::from_utf8` | 10.3 | 35.5 |

The same algorithm on both sides; what differs is the default each language
chose: the hasher, the UTF-8 check, and Nikaia's one-character pattern as a
`char` (#617).

## The scan alone

`scan.nika` and `src/bin/scan.rs` find the separators of every row and nothing
else: Nikaia by `find_all` over `bytes::ByteSet([';', '\n'])`, Rust the way
`ai2.rs` does (the `;` by a vector compare, the `\n` from the temperature's
shape). Both print the count of separators and the xor of their positions,
the same bytes. Counted by hand, not by `brc.sh`: `cargo build --release -p
brc-bench` with `RUSTFLAGS="-C target-cpu=..."`, `scan.nika` as a project's
`src/main.nika` with `brc.sh`'s `nikaia.toml` at 16 units, `nikaia build
--cpu=...`, then `valgrind --tool=callgrind --branch-sim=yes` over 1 000 000
rows.

| per row | instructions | mispredictions |
|---|---:|---:|
| Rust, `scan.rs`, `x86-64` | 31.3 | 0.12 |
| Nikaia, `scan.nika`, `x86-64` | 37.7 | 0.26 |
| Rust, `scan.rs`, `x86-64-v3` | 29.4 | 0.01 |
| Nikaia, `scan.nika`, `x86-64-v3` | 29.5 | 0.26 |

Per separator `find_all`'s `next` is a `tzcnt`, an add, the lowest bit cleared
and the loop's test; per 64-byte block one length test, the set's kind (a
jump table) and the compares. The rest of Nikaia's row is the program's: `n
+= 1` keeps its overflow check, and `n` and `x` are stored at every
separator because `println` takes their address. The mispredictions are one
a block, where the separators of a block run out: their number varies, so
the loop's exit is not predicted. `scan.rs` does not search for the `\n` at
all, which a set search cannot know to skip.

## What is left to build

| what | where | issue |
|---|---|---|
| the sum's check, proved below a length checked once before the fold (ADR-367 D4; the count's is proved); one branch a row, no instruction in today's layout | the prover, the lowering | [#432](https://github.com/Nikaia-Foundation/Nikaia/issues/432) |
| the hot loop's inlining not left to the codegen-unit partition: `hashbrown`'s `entry` and the grammar runtime's `advance` inlined at 1 unit as at 16 (about 40 a row) | the lowering or `winnow-grammar` | [#695](https://github.com/Nikaia-Foundation/Nikaia/issues/695) |
| no whole-file UTF-8 check (10.3 a row): text checked only where it is new, a mapping as bytes, maybe a type for unchecked text (a grammar over `Bytes` is ADR-366, #422) | language, `winnow-grammar`, `std` | [#664](https://github.com/Nikaia-Foundation/Nikaia/issues/664) |

## Why two AI variants

AI variant 1 shows the state after one request: an AI assistant asked once for
an optimised solution. AI variant 2 is the same assistant after several rounds
of research and improvement, each round measured with callgrind. The gap
between them in the table - 147.4 instructions a row against 270.9 and 312.4,
0.38 mispredictions against 3.70 and 3.61 - shows that today an AI-written
program is not optimal from one prompt: it needs iterations, review and
measurement, as Nikaia itself is built in many rounds. That is why Nikaia
leaves to its compiler what would otherwise cost rounds of prompting and
fixing. This describes the state at the time of writing; whether stronger
models change it is left open.

## What this does not claim

Nothing about the 1BRC leaderboard. The entries there are SWAR over raw bytes,
hash tables sized to the key count with the keys stored inline, no UTF-8
anywhere, and every core. AI variant 1 is a single-threaded, readable version
of that idea and stops short of it; AI variant 2 goes most of the way, still on
one core, and `fast.nika` takes its building blocks as far as `std` has them,
counted on one core too. The questions this bench answers are whether a
grammar that describes the file costs you the loop a specialist writes, and
what that loop costs when it is written in Nikaia. Nothing here is a time:
the counts say how much work each program does, not how fast it runs on a
given machine, and `fast.nika` on every core is checked for its output only.

## The files

* `src/bin/naive.rs`, `src/bin/ai1.rs`, `src/bin/ai2.rs` - the three Rust
  programs, each a binary of its own.
* `naive.nika` - the naive Nikaia program; `examples/1brc.nika` is the other.
* `fast.nika` - the Nikaia program written for speed, against `ai2.rs`.
* `scan.nika`, `src/bin/scan.rs` - the scan alone (above).
* `src/bin/gen.rs` - the input: `gen <rows> <file>`.
* `brc.sh` - builds everything, checks the outputs agree, counts, prints the
  table.
