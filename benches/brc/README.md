# The One Billion Row Challenge: Nikaia against Rust

[`examples/1brc.nika`](../../examples/1brc.nika) describes the whole input file
as a grammar and aggregates it with a fold. This bench measures it, the
program a Nikaia programmer writes first, and one written for speed, against
the same aggregation written in Rust:

| program | what it is |
|---|---|
| [`naive.rs`](src/bin/naive.rs) | what a competent Rust programmer writes first: `read_to_string`, `str::lines`, `split_once`, a `HashMap` with the default hasher |
| [`naive.nika`](naive.nika) | the same in Nikaia: `fs::read_to_string`, `lines`, `find` and `split_at` at the `;`, the temperature read digit by digit as `naive.rs` reads it, `std`'s `HashMap` (its own type over `hashbrown`'s table, with Fx hashing and text keys compared by words) |
| [`ai1.rs`](src/bin/ai1.rs), AI variant 1 | written by an AI assistant at medium effort, in one request, as an optimised solution; what a 1BRC entry looks like before it is parallelised: `mmap` (`unsafe`), raw bytes with **no UTF-8 validation**, one `memchr` searcher over the whole file, the separator found by walking back from the line's end, integer temperatures, a word-at-a-time hash |
| [`ai2.rs`](src/bin/ai2.rs), AI variant 2 | written by the same AI assistant over several rounds, each measured with callgrind and improved: a SIMD compare finds the `;` (SSE2; AVX2 where the CPU level has it), a word hash of the name (crc32c where the CPU level has it), a branch-free temperature parse, an own open-addressing table, the end of the input run from a padded copy so that nothing reads past the end; `mmap` and `unsafe` throughout, **no UTF-8 validation** |
| [`examples/1brc.nika`](../../examples/1brc.nika) | a grammar over the whole file, a `par_fold`, names that are slices of a mapped file, `std`'s `HashMap` |
| [`fast.nika`](fast.nika) | the building blocks of AI variant 2 as far as today's `std` has them: the mapped file read as `Bytes`, each row's `;` looked for in the 32 bytes from its start with `simd::Fixed[u8, 32]` (`simd::load`, `eq`, `first()`), each load proved inside the input (the line's end follows from the temperature's shape), the name's first sixteen bytes read as two words with `u64_le`, cut to the name with `low_bits` and no branch, as its hash and key, the temperature from one word without a branch, an own open-addressing table in a `Vec`, the last rows read from a copy with zeros behind them; no `unsafe`, each station's name checked as UTF-8 once, where it is printed. On more than one core it cuts the file with `par_pieces` and folds the pieces with `Par::fold` |
| [`fast-unsafe.nika`](fast-unsafe.nika) | `fast.nika` with the claims a programmer who knows the 1BRC format vouches for, each in `unsafe { assume(…) }` (ADR-376): the table's index in range, a name of at most 100 bytes inside the input, the `;` at least 16 bytes before the end, the temperature's `.` as its second, third or fourth byte, its value at most 99.9, fewer than 10 000 stations, the sums and counts below 10^15 and 10^12. Every one holds for a valid 1BRC file. Where the CPU level has `crc32` (`x86-64-v3`), it hashes the name's two words with `hash::crc32c`, chosen while it is built |
| human-written Rust ([`human/humans.sh`](human/humans.sh)) | [beowolx `fast_1brc`](https://github.com/beowolx/fast_1brc), fetched at a fixed commit (it has no licence, so nothing of it is in the repository), and automataIA's [`v20_safe`](https://github.com/automataIA/1brc-rs) and `v16_flat_table` (MIT, in [`human/automataia`](human/automataia), `v16` derived and corrected by its patch); all three run on every core |

Each pair is compared like for like: `naive.nika` with `naive.rs`, `1brc.nika`
with the two AI variants, `fast.nika` with AI variant 2, `fast-unsafe.nika` with the human-written
programs. They are the comparison that counts for speed.

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
  (`remove-bounds-checks`, `remove-overflow-checks`, `prove-through-loops`,
  which asks a check a loop leaves open through the loop's invariant,
  `skip-free-at-exit`), no
  debug assertions; it keeps the overflow checks the language promises. **Neither side
  uses link-time optimisation.**
* **Two codegen-unit settings, the same on both sides each time: 1 and 16**
  (release's default). Which unit rustc puts a function in decides whether
  LLVM inlines it into the hot loop: `hashbrown`'s `entry` on both sides, and on
  Nikaia's also the grammar runtime's `advance`. AI variant 1 inlines the
  entry at 1 unit and not at 16, 42 instructions a row apart. `1brc.nika`
  inlines the entry at both since #695 (what a fold's step calls is
  `#[inline(always)]`), and with `hashbrown`'s probe in it since `std`'s map
  is its own type (ADR-296 D42: `entry` always inlined, a text key compared
  and hashed by words, a search per length class), and the
  runtime's `advance` at both since it is `#[inline(always)]` too (9.0 a row
  at 1 unit). One setting alone would pick the winner,
  so the table shows both. `brc.sh` sets the count per run:
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
* **`fast-unsafe.nika`** is built twice. In the build every other row has,
  which removes checks, its assumes are believed: each lowers to
  `core::hint::assert_unchecked`, the "unsafe Nikaia" row. In a build that
  removes none (`remove-bounds-checks` and `remove-overflow-checks` off, the
  rest on), each assume is checked as an `assert`; that row keeps every
  check, proved ones included, so it is not `fast.nika`'s like for like.
  Both print `naive`'s answer.
* **The human-written programs** are built by `human/build.sh` at the bench's
  settings (`opt-level = 3`, no LTO, `panic = "unwind"`, 1 and 16 units,
  `x86-64` and `x86-64-v3`), overriding their projects' profiles, and once as
  their authors build them. They run on every core; callgrind runs the threads
  one at a time and counts them all, so a count is the total work. beowolx is
  also counted at one thread (`FAST1BRC_THREADS=1`). The authors' own build
  is `target-cpu=native`, which callgrind cannot run where `native` has
  AVX-512; their profile at `x86-64-v3` is counted instead.
* **Length in tokens**, by [`scripts/tokens.nika`](../../scripts/tokens.nika), the
  same rule for both languages: identifiers, keywords, numbers, literals and
  operators count, comments and whitespace do not, `use` lines do. Each file is
  counted whole.
* **The input** is `gen`'s: 413 stations, the count the real file has, about a
  quarter of the names non-ASCII, from a fixed seed.

## The table

Nikaia 0.0.1188, rustc 1.97.0, Intel Xeon @ 2.10 GHz, 4 cores; callgrind,
1 000 000 rows, per row:

| | instructions, 1 unit | mispredictions, 1 unit | instructions, 16 units | mispredictions, 16 units | tokens |
|---|---:|---:|---:|---:|---:|
| Rust, `naive` | 840.7 | 9.23 | 859.3 | 8.64 | 556 |
| Nikaia, `naive.nika` | 557.4 | 8.07 | 544.8 | 8.31 | 375 |
| Rust, AI variant 1 (`ai1`) | 270.9 | 3.70 | 312.4 | 3.61 | 879 |
| Rust, AI variant 2 (`ai2`) | 147.4 | 0.38 | 147.4 | 0.38 | 1775 |
| **Nikaia**, `examples/1brc.nika` | 209.2 | 0.92 | 208.8 | 0.95 | 642 |
| Nikaia, overflow checks off | 210.2 | 0.92 | 208.8 | 1.04 |  |
| **Nikaia**, `fast.nika` | 106.8 | 0.14 | 107.0 | 0.14 | 1530 |
| Nikaia, `fast.nika`, overflow checks off | 102.8 | 0.14 | 102.0 | 0.14 |  |
| **Nikaia**, `fast-unsafe.nika`, assumes believed | 105.1 | 0.15 | 105.2 | 0.14 | 1864 |
| Nikaia, `fast-unsafe.nika`, assumes checked | 205.1 | 0.15 | 206.8 | 0.15 |  |
| Rust, AI variant 2 (`ai2`), `x86-64-v3` | 132.7 | 0.27 | 132.7 | 0.27 |  |
| Nikaia, `fast.nika`, `x86-64-v3` | 95.8 | 0.14 | 95.9 | 0.14 |  |
| Nikaia, `fast-unsafe.nika`, assumes believed, `x86-64-v3` | 92.1 | 0.14 | 92.2 | 0.15 |  |
| Rust, human: [beowolx `fast_1brc`](https://github.com/beowolx/fast_1brc), 4 threads | 133.7 | 0.47 | 131.3 | 0.47 | 12907 |
| Rust, human: [beowolx `fast_1brc`](https://github.com/beowolx/fast_1brc), 1 thread | 133.1 | 0.47 | 130.8 | 0.48 | |
| Rust, human: [beowolx `fast_1brc`](https://github.com/beowolx/fast_1brc), `x86-64-v3`, 4 threads | 93.4 | 0.69 | 91.5 | 0.30 | |
| Rust, human: [beowolx `fast_1brc`](https://github.com/beowolx/fast_1brc), as its authors build it (`native`, fat LTO, 1 unit, `panic = "abort"`) | not countable with callgrind (AVX-512) | | | | |
| Rust, human: [beowolx `fast_1brc`](https://github.com/beowolx/fast_1brc), its authors' profile at `x86-64-v3` | 92.2 | 0.69 | | | |
| Rust, human: [automataIA `v20_safe`](https://github.com/automataIA/1brc-rs), 4 threads | 210.5 | 1.60 | 216.1 | 1.60 | 5961 |
| Rust, human: [automataIA `v20_safe`](https://github.com/automataIA/1brc-rs), `x86-64-v3`, 4 threads | 185.2 | 1.60 | 190.9 | 1.60 | |
| Rust, human: [automataIA `v20_safe`](https://github.com/automataIA/1brc-rs), as its authors build it (`native`, fat LTO, 1 unit, `panic = "abort"`) | not countable with callgrind (AVX-512) | | | | |
| Rust, human: [automataIA `v20_safe`](https://github.com/automataIA/1brc-rs), its authors' profile at `x86-64-v3` | 187.6 | 1.60 | | | |
| Rust, human: [automataIA `v16_flat_table`](https://github.com/automataIA/1brc-rs), derived and corrected, 4 threads | 207.9 | 0.57 | 207.9 | 0.57 | 3963 |
| Rust, human: [automataIA `v16_flat_table`](https://github.com/automataIA/1brc-rs), derived and corrected, `x86-64-v3`, 4 threads | 197.7 | 0.57 | 197.7 | 0.57 | |
| Rust, human: [automataIA `v16_flat_table`](https://github.com/automataIA/1brc-rs), derived and corrected, as its authors build it (`native`, fat LTO, 1 unit, `panic = "abort"`) | not countable with callgrind (AVX-512) | | | | |
| Rust, human: [automataIA `v16_flat_table`](https://github.com/automataIA/1brc-rs), derived and corrected, its authors' profile at `x86-64-v3` | 198.6 | 0.57 | | | |

The naive programs: Nikaia's is a third shorter and runs a third fewer
instructions at both settings, from the same algorithm (below).

Against AI variant 1, Nikaia runs 23 % fewer instructions at 1 unit and 33 %
fewer at 16 units, where AI variant 1 leaves its table's entry out of line.
The mispredictions are a quarter of AI variant 1's at 1 unit and under a third at 16. Nikaia's program is a
quarter fewer tokens, with UTF-8 and every overflow checked
or proved; the same file also runs on every core. The temperature's checks are
proved (ADR-314), and so are the counter's, `count += 1` and the merge's
`count += other.count`, from the input's length (ADR-367): the counter is one
`incq`. What is left is `sum += temp`, one `jo` after its `add`; the build
without it runs one branch fewer a row at both settings and the same
instructions within one, laid out differently.

The mispredictions depend on where the code lies, the instructions do not:
callgrind's simulated predictor is indexed by address. Every build of
`1brc.nika` in the table stays at 0.92 to 1.05, a quarter to under a third of
AI variant 1's 3.6 to 3.7.

AI variant 2 runs 29 % fewer instructions than `1brc.nika` at 16 units and
30 % fewer at 1 unit, and 0.38 mispredictions against 0.92 to 1.05, in 2.8 times its
tokens, with `unsafe`, target-specific intrinsics and no UTF-8 check.

`fast.nika` runs 21 % fewer instructions than AI variant 2 at both unit
counts and 22 % fewer at `x86-64-v3`, and mispredicts well under half as
often (0.14 against 0.38 a row; 0.14 against 0.27 at `x86-64-v3`). It is 20 %
fewer tokens, with every index and overflow checked or proved, and a row
without a `.` where the temperature's shape puts one stops the program.
Where the difference lies is below.

**Instructions bought for fewer mispredictions.** A mispredicted branch
costs a current core some 15 to 20 cycles, in which it could have run about
50 instructions, so `fast.nika` trades a few instructions for a branch where
the branch's outcome varies with the input:

* Its name's two words are cut to the name with `low_bits`, without a
  branch. Cut by its length with two branches (`len <= 8`, `len < 16`) the
  program ran 110.0 instructions a row at 16 units and 101.0 at
  `x86-64-v3` at 0.0.1185, 6.0 and 1.9 fewer than then, but mispredicts 0.68 times a row instead of
  0.14: the length of the names varies. At `x86-64` `low_bits` is a shift, a
  mask and a select on the count a word; at `x86-64-v3` it is one `bzhi`,
  with the length first held to the 16 bytes the words cover, so the count
  needs no clamp (`CUT_IN_ONE`).
* It looks for each `;` in the 32 bytes from its row's start, with
  `simd::Fixed[u8, 32]` (`simd::load`, `eq`, `first()`, ADR-377), rather
  than taking the next bit of a 64-byte block's mask (`find_all`'s default),
  whose loop exit is mispredicted where a block's separators run out (0.25 a
  row at 0.0.1178). A row is about 14 bytes, so the `;` is almost always in
  the first 32, and that test is predicted. Each load is proved inside the
  input, with no `unsafe`: the row starts before `stop`, `MARGIN` bytes
  before the end, and an `assert` bounds the name to 100 bytes.
  `find_all`'s `Scan::AfterHit` searches the same way inside `std`, but its
  iterator keeps its position in a field, and Rust cannot tell LLVM that
  `at <= len` holds from one call to the next without an `unsafe` assume; it
  cost 5.0 instructions a row more at `x86-64-v3` and 9.0 at `x86-64`.

**Why `fast-unsafe.nika` exists.** It is not there to show that Nikaia is
cleverer than the people who wrote the fastest Rust programs; it uses their
tricks. It is there to show that the language reaches as far: that every
optimisation close to the hardware a tuned Rust program makes - a search a
block at a time, a name read as words, a branch-free parse, an instruction
chosen by the CPU level, claims about the input the compiler may believe -
can be written in Nikaia, and where a claim is needed, with one `assume` the
reader sees, not with `unsafe` spread through the program. `fast.nika` beside
it shows how much of that the prover already makes safe.

`fast-unsafe.nika`, its assumes believed, runs 2 % fewer instructions than
`fast.nika` at 1 and at 16 units, 4 % fewer at `x86-64-v3`, and 29 %
fewer than AI variant 2, with the same 0.14 mispredictions: its assumes drop
the checks `fast.nika` still makes (the table below), the `.` guard as
well, and the ranges they state let LLVM simplify the temperature's
arithmetic and drop `low_bits`' saturation. At `x86-64-v3` it hashes the
name's two words with two `crc32` instructions instead of a multiply.
Against beowolx at one thread it runs 21 % fewer instructions at 1 unit and
20 % fewer at 16, with under a third of its mispredictions (0.14 against
0.47), in a seventh of its tokens. At `x86-64-v3` it runs 92.1 against
beowolx's 93.4 on four threads at 1 unit, fewer, and 92.2 against 91.5 at 16
units, 1 % more (counted by hand on one thread: 91.0); it mispredicts less than
half as often there (0.15 against 0.30 at 16 units, 0.14 against 0.69 at 1 unit).
beowolx parses two rows' temperatures in one AVX2 vector; written so in
`fast-unsafe.nika` with `core::arch` intrinsics, that saved 1.6
instructions a row (ADR-377 D8, at 0.0.1166). Checked, the same file
costs 206.8 at 16 units: every assume tested and no check removed.

## Where Nikaia's instructions go

Callgrind's attribution per row for the table's Nikaia row **at 16 units**,
built again with `debug = 1` (line tables) to have one: its instructions and
its mispredictions are the table's. Each instruction goes to the innermost crate it
was inlined from that is not `core`, `alloc` or `std` (by `addr2line -i`);
what `core` inlines into the generated rule is its own row.

| | instructions | mispredictions |
|---|---:|---:|
| the byte reads, cuts, digit tests and arithmetic `core` inlines into the generated rules | 42.4 | 0.26 |
| the grammar's runtime (`winnow-grammar` 21.0, `winnow` 9.0) | 30.0 | 0.00 |
| the hash table's probe (`hashbrown`, inlined into the fold through `std`'s map) | 27.6 | 0.05 |
| finding the `;` and the line's end (`memchr`, with its SIMD intrinsics) | 27.0 | 0.00 |
| hashing a name (`nikaia-std`'s byte hash, inline up to 16 bytes) | 25.9 | 0.49 |
| the generated rules and actions, the stats' update among them | 25.6 | 0.01 |
| comparing a name with the one in the table (`std`'s map: by words, the query's read once, a search per length class; no `memcmp`) | 16.3 | 0.11 |
| the UTF-8 check (`simdutf8`) | 10.3 | 0.09 |
| the rest: start-up, the output | 3.7 | 0.03 |

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
values the checks keep alive are stored on the stack and read back. Its mispredictions, 0.14 a row
against AI variant 2's 0.38, are low because neither the name's cut nor the
search for the `;` branches on the input (above).

What each check costs, at 16 units, `x86-64`: the lowered Rust of `fast.nika`
edited by hand to drop exactly that check (an unchecked read, `wrapping_*`,
`get_unchecked`), built the same way, its output compared with `naive`'s and
counted, before #714's steps (163.6 a row then). Since, `fast.nika` states
`Table`'s rule `slots.len() == SLOTS` (ADR-369; no method can break it, so it
is checked only where `table()` builds it), `stop > 0 && stop <= buf.len() -
MARGIN` once per `run`, and `0 <= len <= 100` once per row; with
`prove-through-loops` the probe's index, the word reads at `pos` and `pos +
8`, `sep - pos`, `sep + 1`'s word read, `8 * len` and its shift and `sep + 4 +
(dot >> 3)` are proved, and the table has one multiply for its hash: 125.7 a
row. Since, `slot_of`'s postcondition proves the table's index in `run`'s
update, its precondition the long name's cut of `buf`, and the walk proves the
temperature's `- signed`; each row's index is written through the proved place
(the write already had no check left in the binary: LLVM had taken it from the
inlined postcondition). What a row still checks, and what proving it needs:

| check | what proving it needs |
|---|---|
| `sum += v`, `count += 1` in `run`, the only overflow tests left in the loop (with them off, 113.8 a row instead of 115.8 at 1 unit; 116.0 either way at 16 units, laid out differently) | ADR-361 D11's summary of a slot's field, carried across the `Par::fold` (ADR-367). Not enough as decided: a turn here is a piece, not a row, and `count` grows inside `run`'s loop; and `|sum|` at most 1023 times the rows of an input below 2^60 does not fit an `i64` without ADR-367 D4's length test before the fold |
| the long name's cut of the slot's `name`, `self.slots[at].name[0..<len]` in `slot_of` (a long name only) | a fact of a field of an indexed element (#384): `name.len() == len` for every slot, D11's summary |

## Where `naive.nika` differs from `naive.rs`

Callgrind's attribution per row at 16 units, by function. `naive.nika`'s loop
has the table's entry inlined into it, so there its instructions are split by
the crate they were inlined from, as above (built again with `debug = 1`, the
same count as the table's).

| | `naive.nika` | `naive.rs` |
|---|---:|---:|
| finding the line's end (`memchr`) | 147.7 | 147.7 |
| finding the `;` (`CharSearcher`); `naive.rs` also finds the `.` | 163.7 | 247.6 |
| `CharSearcher`'s `memcmp` of the one-byte pattern it found | 20.1 | 30.0 |
| hashing a name: Fx in Nikaia's `std`, SipHash in Rust's default | 24.1 | 185.3 |
| the hash table's entry | 28.4 | 74.8 |
| comparing a name with the one in the table: by words in Nikaia's `std`, `memcmp` in Rust | 13.4 | 19.6 |
| the loop: cutting, the digits, the stats | 137.2 | 116.3 |
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
| Nikaia, `scan.nika`, `x86-64` | 38.2 | 0.26 |
| Rust, `scan.rs`, `x86-64-v3` | 29.4 | 0.01 |
| Nikaia, `scan.nika`, `x86-64-v3` | 115.0 | 0.26 |

Nikaia 0.0.1185; the Rust rows are from an earlier count of the same
program. `scan.nika` takes `find_all`'s default, `Scan::ByBlock`. Its set is
held in a variable, and at `x86-64-v3` that costs about 400 instructions a
block: LLVM splits each half block's mask into single bits on the stack
([#780](https://github.com/Nikaia-Foundation/Nikaia/issues/780)). The same
set written at `find_all` counts 36.7 at `x86-64` and 32.2 at `x86-64-v3`.

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
| no whole-file UTF-8 check (10.3 a row): text checked only where it is new, a mapping as bytes, maybe a type for unchecked text (a grammar over `Bytes` is ADR-366, #422; a name used only as a key, checked on a miss, is its D5, not built). Today the same program over `data[0..<data.len()]` (a `Bytes`) counts 350.2 and 338.1: the per-name `from_utf8` is 115.6 a row, and the bytes loop 10 more than the text one, so D5 alone would win at most the 10.3 back | language, `winnow-grammar`, `std` | [#664](https://github.com/Nikaia-Foundation/Nikaia/issues/664) |

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
