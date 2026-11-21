# The Benchmarks Game: Nikaia against the fastest Rust

Programs of the [Computer Language Benchmarks Game](https://benchmarksgame-team.pages.debian.net/benchmarksgame/),
each counted against the **fastest Rust entry published there** on
2026-11-16, not a Rust program written here, plus the `json` package against
`serde_json`. The Rust entries are copied unchanged, with their header, the
link to the entry and its revised BSD licence.

```sh
nikaia run benches/game/game.nika             # every problem, the table below
nikaia run benches/game/game.nika -- n-body   # one
nikaia run benches/game/game.nika -- json     # json-small, -large, -deep
```

## The programs

| problem | what it exercises | Nikaia, as first written | Nikaia, written for speed | Rust entry |
|---|---|---|---|---|
| n-body | floats in a tight loop | [`examples/n-body`](../../examples/n-body/src/main.nika) | [`speed.nika`](n-body/speed.nika): `simd::Fixed[f64, 4]` | [Rust #9](n-body/rust-9.rs) |
| spectral-norm | floats, nested loops, calls | [`main.nika`](spectral-norm/main.nika) | [`speed.nika`](spectral-norm/speed.nika): `simd::Fixed[f64, 2]` | [Rust #6](spectral-norm/rust-6.rs), threads |
| fannkuch-redux | small arrays, permutations | [`main.nika`](fannkuch-redux/main.nika): lists, a clone per permutation | [`speed.nika`](fannkuch-redux/speed.nika): arrays of 16, no allocation | [Rust #6](fannkuch-redux/rust-6.rs), threads |
| binary-trees | many short-lived small objects | [`main.nika`](binary-trees/main.nika): a recursive enum | none: the faster form is an arena, which Nikaia does not offer | [Rust #5](binary-trees/rust-5.rs), bumpalo arena, threads |
| fasta | generated text, a random generator, output | [`main.nika`](fasta/main.nika): a `String` a line | [`speed.nika`](fasta/speed.nika): a letter table, `Bytes` written a block at a time | [Rust #7](fasta/rust-7.rs), held to one core |
| k-nucleotide | hash maps keyed by short text | [`main.nika`](k-nucleotide/main.nika) | [`speed.nika`](k-nucleotide/speed.nika): two-bit codes as `u64` keys | [Rust #7](k-nucleotide/rust-7.rs), threads |
| reverse-complement | large byte buffers, table lookup | [`main.nika`](reverse-complement/main.nika): text split and rebuilt | [`speed.nika`](reverse-complement/speed.nika): in place, an `Array[u8, 256]` table | [Rust #1](reverse-complement/rust-1.rs), threads |
| json | reading and writing JSON | [`json/tree.nika`](json/tree.nika) | none | [`serde_json`](json/rust-tree.rs), [its own README](json/README.md) |

Not here yet: mandelbrot, whose binary image output waits for its program
(#1616; `print` of `Bytes`, ADR-019 D7, is built since); regex-redux, which
needs a regex package; pidigits, which needs integers of any size.

## How it is measured

By [`docs/measuring.md`](../../docs/measuring.md):

* **Counts, not a clock.** Instructions and mispredicted branches
  (`valgrind --tool=callgrind --branch-sim=yes`, conditional and indirect)
  over the whole process, divided by the problem's unit (a step, a node, a
  letter). A count does not move from one run to the next.
* **Builds.** Each program is a binary of its own. The Rust entry is built as
  the game builds it: `opt-level = 3`, one codegen unit, its `target-cpu`.
  The Nikaia program with the same `opt-level` and unit count, once at
  `x86-64` and once at `x86-64-v3`, the least level that holds the game's
  `ivybridge`. Each is counted with overflow checks on and off; the
  comparison column takes checks off on both sides.
* **The same output.** Every program of a problem prints the same bytes at
  both settings, or the run stops. Long outputs are compared and shown by
  their md5.
* **Sizes.** Chosen so that one callgrind run stays under about a minute;
  each is in the table's head. fasta's output at 2500000 is the input of
  reverse-complement, at 250000 the input of k-nucleotide.
* **Threads.** callgrind runs one thread at a time and counts them all. An
  entry on threads is counted as it is, with a second row held to one thread
  where it allows it. fasta's entry waits by spinning, which callgrind counts
  instead of the work, so it runs on one core.
* **Standard input.** Nikaia programs that read it run with
  `io-method = "blocking"`, since valgrind 3.22 hangs on the `io_uring` wait.
* **Tokens.** Each program's length by `scripts/tokens.nika`, the same rule
  for both languages.

## The table

Counted on 2026-11-18 with Nikaia 0.0.1431 and rustc 1.97.0 on an Intel Xeon
at 2.10 GHz. Each row gives instructions and mispredicted branches per unit.
The column *off / Rust* divides the program's instructions with checks off by
those of the Rust entry. Tokens are given once per program.

| problem | run, unit | output, the same for every program |
|---|---|---|
| n-body | `n-body 1000000`, a step | `-0.169075164` `-0.169086185` |
| spectral-norm | `spectral-norm 1000`, a matrix entry | `1.274224148` |
| fannkuch-redux | `fannkuch-redux 10`, a permutation | `73196` `Pfannkuchen(10) = 38` |
| binary-trees | `binary-trees 16`, a tree node | the game's lines for depth 16 |
| fasta | `fasta 2500000`, a letter | md5 `daf1153fded2bb87f2aa03d03990937f` |
| k-nucleotide | fasta 250000's output on standard input, a letter of section THREE | the game's frequencies and counts |
| reverse-complement | fasta 2500000's output on standard input, a letter | md5 `ce5a95c777fd3fe04a62294e69b231e9` |
| json-small | `input.nika small` on standard input (1,046 bytes), a byte | md5 `b4abe7cc43bddbec2274cdb58c59c4c1` |
| json-large | `input.nika large` (10,000,080 bytes), a byte | md5 `4bfb653cbee186e71e901b53dad9a32f` |
| json-deep | `input.nika deep` (819,202 bytes), a byte | md5 `fcbb04950b38a0639d7dd636003ecf9e` |

| problem | program | instructions, checks on | mispredicted, on | instructions, checks off | mispredicted, off | off / Rust | tokens |
|---|---|---:|---:|---:|---:|---:|---:|
| n-body | Rust #9, `target-cpu=ivybridge` | 196.4 | 0.008 | 196.4 | 0.008 | 1.00 | 1768 |
| n-body | Nikaia, `--cpu=x86-64` | 410.5 | 0.008 | 410.5 | 0.008 | 2.09 | 1057 |
| n-body | Nikaia, `--cpu=x86-64-v3` | 326.5 | 0.008 | 326.5 | 0.008 | 1.66 |  |
| n-body | Nikaia for speed, `--cpu=x86-64` | 510.5 | 0.008 | 510.5 | 0.008 | 2.60 | 1164 |
| n-body | Nikaia for speed, `--cpu=x86-64-v3` | 249.5 | 0.008 | 249.5 | 0.008 | 1.27 |  |
| spectral-norm | Rust #6, `target-cpu=ivybridge` | 20.3 | 0.001 | 10.0 | 0.001 | 1.00 | 1012 |
| spectral-norm | Rust #6, one thread, `target-cpu=ivybridge` | 20.3 | 0.001 | 10.0 | 0.001 | 1.00 |  |
| spectral-norm | Nikaia, `--cpu=x86-64` | 21.5 | 0.001 | 19.0 | 0.001 | 1.90 | 415 |
| spectral-norm | Nikaia, `--cpu=x86-64-v3` | 19.5 | 0.001 | 14.3 | 0.001 | 1.42 |  |
| spectral-norm | Nikaia for speed, `--cpu=x86-64` | 17.4 | 0.250 | 9.5 | 0.001 | 0.95 | 792 |
| spectral-norm | Nikaia for speed, `--cpu=x86-64-v3` | 15.9 | 0.001 | 8.0 | 0.001 | 0.80 |  |
| fannkuch-redux | Rust #6, `target-cpu=ivybridge` | 87.3 | 1.945 | 81.5 | 1.794 | 1.00 | 954 |
| fannkuch-redux | Rust #6, one thread, `target-cpu=ivybridge` | 87.2 | 1.945 | 81.5 | 1.794 | 1.00 |  |
| fannkuch-redux | Nikaia, `--cpu=x86-64` | 513.1 | 8.842 | 510.1 | 11.056 | 6.26 | 378 |
| fannkuch-redux | Nikaia, `--cpu=x86-64-v3` | 514.3 | 9.130 | 511.3 | 9.304 | 6.27 |  |
| fannkuch-redux | Nikaia for speed, `--cpu=x86-64` | 225.7 | 7.518 | 216.1 | 7.444 | 2.65 | 460 |
| fannkuch-redux | Nikaia for speed, `--cpu=x86-64-v3` | 225.2 | 7.522 | 215.6 | 7.448 | 2.64 |  |
| binary-trees | Rust #5, `target-cpu=ivybridge` | 49.3 | 0.577 | 43.3 | 0.471 | 1.00 | 502 |
| binary-trees | Rust #5, one thread, `target-cpu=ivybridge` | 49.3 | 0.577 | 43.3 | 0.471 | 1.00 |  |
| binary-trees | Nikaia, `--cpu=x86-64` | 183.0 | 1.209 | 145.8 | 1.108 | 3.37 | 237 |
| binary-trees | Nikaia, `--cpu=x86-64-v3` | 182.7 | 1.175 | 145.4 | 1.108 | 3.36 |  |
| fasta | Rust entry 7, `taskset -c 0`, `target-cpu=ivybridge` | 33.3 | 0.014 | 25.7 | 0.014 | 1.00 | 1895 |
| fasta | Nikaia, `--cpu=x86-64` | 81.4 | 1.039 | 79.3 | 1.058 | 3.08 | 633 |
| fasta | Nikaia, `--cpu=x86-64-v3` | 80.6 | 1.063 | 77.7 | 1.029 | 3.02 |  |
| fasta | Nikaia for speed, `--cpu=x86-64` | 27.5 | 0.015 | 20.2 | 0.016 | 0.78 | 773 |
| fasta | Nikaia for speed, `--cpu=x86-64-v3` | 27.5 | 0.015 | 20.2 | 0.015 | 0.78 |  |
| k-nucleotide | Rust entry 7, `target-cpu=ivybridge` | 359.5 | 0.215 | 313.9 | 0.209 | 1.00 | 1543 |
| k-nucleotide | Nikaia, `--cpu=x86-64` | 2066.6 | 0.159 | 1999.6 | 0.156 | 6.37 | 506 |
| k-nucleotide | Nikaia, `--cpu=x86-64-v3` | 1984.7 | 0.166 | 1921.7 | 0.177 | 6.12 |  |
| k-nucleotide | Nikaia for speed, `--cpu=x86-64` | 999.5 | 0.218 | 986.1 | 0.202 | 3.14 | 627 |
| k-nucleotide | Nikaia for speed, `--cpu=x86-64-v3` | 956.9 | 0.199 | 956.3 | 0.185 | 3.05 |  |
| reverse-complement | Rust entry 1, `target-cpu=ivybridge` | 1.9 | 0.000 | 1.9 | 0.000 | 1.00 | 1966 |
| reverse-complement | Rust entry 1, one thread, `target-cpu=ivybridge` | 1.9 | 0.000 | 1.9 | 0.000 | 1.00 |  |
| reverse-complement | Nikaia, `--cpu=x86-64` | 45.5 | 0.069 | 40.4 | 0.069 | 20.86 | 385 |
| reverse-complement | Nikaia, `--cpu=x86-64-v3` | 45.5 | 0.069 | 40.4 | 0.069 | 20.87 |  |
| reverse-complement | Nikaia for speed, `--cpu=x86-64` | 10.8 | 0.020 | 10.6 | 0.020 | 5.48 | 571 |
| reverse-complement | Nikaia for speed, `--cpu=x86-64-v3` | 10.2 | 0.020 | 10.1 | 0.020 | 5.21 |  |
| json-small | serde_json 1 (`preserve_order`, `arbitrary_precision`), `target-cpu=x86-64` | 491.9 | 8.622 | 489.1 | 8.662 | 1.00 | 148 |
| json-small | Nikaia, `--cpu=x86-64` | 657.1 | 11.499 | 651.1 | 11.728 | 1.33 | 55 |
| json-small | Nikaia, `--cpu=x86-64-v3` | 658.1 | 11.555 | 650.2 | 11.593 | 1.33 |  |
| json-large | serde_json 1 (`preserve_order`, `arbitrary_precision`), `target-cpu=x86-64` | 137.5 | 1.063 | 136.6 | 1.040 | 1.00 | 148 |
| json-large | Nikaia, `--cpu=x86-64` | 230.4 | 2.159 | 225.2 | 2.082 | 1.65 | 55 |
| json-large | Nikaia, `--cpu=x86-64-v3` | 229.4 | 1.995 | 224.1 | 2.334 | 1.64 |  |
| json-deep | serde_json 1 (`preserve_order`, `arbitrary_precision`), `target-cpu=x86-64` | 387.3 | 0.899 | 382.4 | 0.900 | 1.00 | 148 |
| json-deep | Nikaia, `--cpu=x86-64` | 748.1 | 5.381 | 739.4 | 5.595 | 1.93 | 55 |
| json-deep | Nikaia, `--cpu=x86-64-v3` | 745.4 | 5.137 | 737.2 | 5.357 | 1.93 |  |

## Where Nikaia is behind

Each loss is quoted with a control where the entry's algorithm differs: the
Nikaia program's own algorithm written in plain Rust and counted beside it,
which separates the entry's algorithm from what Nikaia's code costs.

The causes below come from `callgrind_annotate` and `objdump` in the run
that added each problem. Its commit message holds the attribution. Where a
figure has moved since, the paragraph says so.

* **n-body**, 1.66x as first written, 1.27x for speed, at `x86-64-v3`. At
  `x86-64` the speed row, `simd::Fixed[f64, 4]` on a level without AVX,
  counts more than the plain one (2.60x against 2.09x). Neither gap has
  been attributed yet.
* **spectral-norm**, 1.42x as first written, 0.80x for speed. The plain loop
  is scalar, one `divsd` an entry, where Rust #6 divides two lanes by hand.
  `i64`'s signed `/ 2` also costs a shift, an add and a shift, 3 of an entry's
  13 instructions, where Rust's `usize` costs one shift.
* **fannkuch-redux**, 6.26x and 2.65x. The same two algorithms in plain Rust
  count 499 and 266 a permutation, so Nikaia's code costs what Rust's does.
  The gap is Rust #6's algorithm. It holds the permutation in one `u128` and
  flips a prefix with one `pshufb` whose mask it picks at run time. `std::simd`
  has only constant shuffles; a run-time shuffle is undecided (ADR-377,
  section 4).
* **binary-trees**, 3.37x (re-counted on 2026-11-21 with Nikaia 0.0.1517;
  5.75x before). Each node's box is built in the task's block (ADR-327 D16,
  D17), `main`'s here, so `malloc` and `free` are no longer counted (71 % of
  the instructions before). The block's own cut and free are now about 80 %:
  37 % building a box (`Boxed::at`), 25 % dropping one, and 18 % freeing a
  box in a segment of the chain beyond the current one (`give_chained`),
  which the long-lived tree and each tree larger than a segment make common.
  Rust #5 bumps a pointer in a `bumpalo` arena and frees each tree whole,
  without a count per page. The same boxed tree in plain Rust counted 197.1
  a node when binary-trees was added, so Nikaia is now below it.
* **fasta**, 3.08x as first written, 0.78x for speed. The plain program's
  draw walks the cumulative probabilities and mispredicts about once a
  letter, where the entry counts with SSE2. It also grows one `String` a line
  and writes it line-buffered. This is the program's algorithm: when fasta
  was added the same algorithm in plain Rust counted 84.1 against Nikaia's
  97.3, all of the extra in `Text::push`. Since then `Text::push` is inlined
  at the call site (#1864), and the plain row fell from 3.66x to 3.08x.
* **k-nucleotide**, 6.37x and 3.14x. Both programs read standard input,
  which is untrusted, so their maps hash with the keyed SipHash-1-3.
  Trusted, the speed program counts 1.13x Rust. When it was added, the plain
  program also spent 545 instructions a letter in UTF-8 checks of each cut of
  the input. The row has since fallen from 8.08x to 6.37x, about that much,
  since a cut of a text's own bytes reads back as text without the check.
* **reverse-complement**, 20.9x and 5.5x. The same two algorithms in plain
  Rust count 39.2 and 15.3 a letter. The entry's algorithm makes the gap: it
  uses SSSE3 `pshufb` as a run-time complement table on 16 bytes at a time,
  stores them back with vector stores and finds headers with `memchr`.
  `std::simd` has neither a run-time shuffle nor a store into a buffer
  (ADR-377, section 4, undecided). The speed program also looks for `>` a
  byte at a time, since a `Vec[u8]` has no `find`.
* **json**, 1.33x to 1.93x serde_json. The package builds the path text of
  every value while it reads, text only an error shows. It also copies text
  a character at a time. Both are in [json/README.md](json/README.md).
