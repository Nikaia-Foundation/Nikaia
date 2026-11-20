# What named wide operations would buy (#876)

Nikaia has no `i128`/`u128` (ADR-315) and no named wide operation, so a
program cannot reach the single instructions that give a 128-bit result: the
full 64×64 → 128 product (`MUL`/`MULX`), a carry chain (`ADC`), 128 ÷ 64
(`DIV r64`) and the carry-less product (`PCLMULQDQ`). This measures, before
any decision, what that costs on known workloads.

```sh
nikaia run benches/wide-ops/bench.nika            # the table below
nikaia run benches/wide-ops/bench.nika -- 0.2     # the same at a fifth of the sizes
NIKAIA=$PWD/target/fast/nikaia nikaia run benches/wide-ops/bench.nika   # a compiler of your choice
```

## Method

Each workload is one program on each side, printing one checksum line, and the
script refuses to count until every program prints the same line at both sizes.

| program | what it is |
|---|---|
| `nikaia` | `wide.nika`, today's Nikaia: a 64×64 product from four 32×32 products, a 4096-bit number as 128 limbs of 32 bits with the carry in a `u64`, the carry-less product through a 4-bit table, CRC through a byte table |
| `emu` | `src/main.rs`, the same algorithm in Rust, line for line: against `nikaia` it shows the language, against `native` the missing instruction |
| `native` | tuned Rust: `u128` products, `_addcarry_u64`, `PCLMULQDQ` (`#[target_feature]` per function, as a library with a fallback would switch it on) |
| `u128rem` | `mulmod` only: `(a as u128 * b as u128 % m as u128) as u64`, which LLVM lowers to a call of `__umodti3` |
| `div` | `mulmod` only: one `DIV r64` (inline assembly), the instruction `wide_rem` would reach; `a, b < m`, so the high word is below `m` and it cannot fault |

The workloads (inputs from splitmix64, the same stream on both sides):

| workload | what it does | unit |
|---|---|---|
| `bigadd` | `a += b; b += a`, 4096 bits, modulo 2^4096 | one addition |
| `bigmul` | `a = lo(a·b) + hi(a·b)`, 4096 × 4096 bits, schoolbook | one product |
| `mulmod` | Miller-Rabin with the 12 bases that decide every 64-bit number, over odd 64-bit candidates; Montgomery multiplication (`nikaia`, `emu`, `native`) or plain `a·b mod m` (`u128rem`, `div`) | one candidate |
| `wymix` | wyhash's mix `lo ^ hi` of a 64×64 product, chained | one mix |
| `pcg64` | PCG64 XSL RR 128/64, a 128-bit LCG | one output |
| `crc64` | CRC-64/ECMA-182, not reflected, over 64 KiB; `native` is Barrett reduction, two carry-less products per 8 bytes | one byte |
| `ghash` | GHASH's product in GF(2^128) modulo x^128 + x^7 + x^2 + x + 1, chained over blocks; bit order not reflected, which changes no cost | one 16-byte block |

Counted with callgrind (`--branch-sim=yes`), never timed (docs/measuring.md).
Every program runs at N and at 2N and the table holds the difference divided by
the units of work N adds, so start-up, input generation and table building are
not in it. Mispredicted branches are conditional plus indirect. Both sides are
release builds: Rust `cargo build --release`, Nikaia a project at
`opt-level = 3` (default optimizations), 16 codegen units, no LTO, the baseline
`x86-64` CPU. Outputs, builds and callgrind's files go to
`target/wide-ops-bench`.

## Result

Measured 2026-11-20 on `develop` (5752e1554), valgrind 3.22, rustc 1.97.0:

| workload | unit | program | instr/unit | mispred/unit | vs native |
|---|---|---|--:|--:|--:|
| bigadd | 4096-bit add | nikaia | 1604.5 | 1.01 | 7.5× |
| bigadd | 4096-bit add | emu | 1608.5 | 1.00 | 7.5× |
| bigadd | 4096-bit add | native | 214.5 | 1.00 | 1.0× |
| bigmul | 4096×4096-bit mul | nikaia | 191195.6 | 129.80 | 4.8× |
| bigmul | 4096×4096-bit mul | emu | 191401.8 | 132.20 | 4.8× |
| bigmul | 4096×4096-bit mul | native | 40067.8 | 67.60 | 1.0× |
| mulmod | Miller-Rabin test | nikaia | 4986.4 | 75.83 | 2.5× |
| mulmod | Miller-Rabin test | emu | 4219.7 | 43.23 | 2.1× |
| mulmod | Miller-Rabin test | native | 1996.1 | 43.33 | 1.0× |
| mulmod | Miller-Rabin test | u128rem | 3236.6 | 43.49 | 1.6× |
| mulmod | Miller-Rabin test | div | 954.6 | 42.29 | 0.5× |
| wymix | mix | nikaia | 32.0 | 0.00 | 4.0× |
| wymix | mix | emu | 31.0 | 0.00 | 3.9× |
| wymix | mix | native | 8.0 | 0.00 | 1.0× |
| pcg64 | output | nikaia | 45.0 | 0.00 | 2.9× |
| pcg64 | output | emu | 45.0 | 0.00 | 2.9× |
| pcg64 | output | native | 15.5 | 0.00 | 1.0× |
| crc64 | byte | nikaia | 7.4 | 0.00 | 2.9× |
| crc64 | byte | emu | 7.4 | 0.00 | 2.9× |
| crc64 | byte | native | 2.5 | 0.00 | 1.0× |
| ghash | 16-byte block | nikaia | 524.0 | 0.00 | 6.5× |
| ghash | 16-byte block | emu | 528.0 | 0.00 | 6.5× |
| ghash | 16-byte block | native | 81.0 | 0.00 | 1.0× |

What it says:

* **The cost is the missing instruction, not the language.** `nikaia` and
  `emu` are within 3 % of each other on six of the seven workloads, so a named
  operation that reaches the instruction would close the whole gap to `native`:
  2.9× to 7.5× fewer instructions.
* **The full product pays everywhere** (`wymix` 4.0×, `pcg64` 2.9×, `bigmul`
  4.8×, `mulmod` 2.5×): `widening_mul` alone, with `u128` underneath, reaches
  `MUL`.
* **The carry chain pays most** (`bigadd` 7.5×): 32-bit limbs do half the work
  per step and pay for the split; `_addcarry_u64` is one `ADC` a limb.
* **The carry-less product pays** (`crc64` 2.9× against a byte table, `ghash`
  6.5× against a 4-bit table) and is reachable from no integer type at all.
* **128 ÷ 64**: `u128 %` is a library call (`u128rem`, 1.6× the Montgomery
  `native`), which is the issue's point that `u128` does not reach `DIV`. The
  `div` row is half of `native`'s instructions, but callgrind counts `DIV r64` as
  one instruction whatever its latency (tens of cycles on many x86 cores, from
  memory, not checked): this row is no speed claim, and the choice between
  `wide_rem` and Montgomery needs a cycle count on real hardware.
* **Not measured**: `ADCX`/`ADOX` (two carry chains). Stable Rust's
  `_addcarryx_u64` does not promise them and LLVM seldom emits them, so
  `native` here is a single `ADC` chain; a double chain needs assembly.
* **`mulmod`'s `nikaia` row is 18 % above `emu`**, with 33 more mispredicted
  branches a test: the one workload where the language costs as well, in the
  Miller-Rabin loop around the product (`wymix`, the product alone, is even).
  Not chased here.
