# What a `par_iter` chain costs against Rayon

[ADR-async-5](../../docs/specification/adr/adr-async-5.md) D28 accepts the
lowering of a `par_iter` chain only with measured rows:
`par_iter().map(…).collect()` over 10^6 elements counted against Rayon, once
with a lambda that can throw and once with one that allocates (#988 item 11,
#2502).

Each row is a `.nika` program and its Rayon twin, the same steps on both
sides: the list `0..<n` made first, the chain, the result summed (its
values, or its texts' lengths) and printed. In the row that can throw no
element fails, so both sides do the same work; Rayon's `collect` into a
`Result` keeps one failure and Nikaia's walk keeps every one (D26), which is
what that row costs.

| row | Nikaia | Rayon |
|---|---|---|
| allocates | `allocates.nika`: `xs.par_iter().map(fn(x) { f"{x}" }).collect()` | `src/bin/par_map_allocates.rs`: `xs.par_iter().map(\|x\| format!("{}", x)).collect()` |
| can throw | `throws.nika`: `xs.par_iter().map(fn(x) { work(x) }).collect()`, `work` throwing `Bad` for a negative element | `src/bin/par_map_throws.rs`: `xs.par_iter().map(\|x\| work(*x)).collect::<Result<_, _>>()` |

```sh
nikaia run benches/par-map/par_map.nika                  # 10^6 elements, 4 threads
nikaia run benches/par-map/par_map.nika -- 10000 2       # quicker
NIKAIA=$PWD/target/fast/nikaia nikaia run benches/par-map/par_map.nika
```

`par_map.nika` builds both sides at opt-level 3, 16 codegen units, **no LTO**,
no debug info: the Rayon side with the workspace's `release` profile, the
Nikaia side through its `[build.x86_64-linux]`, at `user-parallelism = "yes"`
(at `no` the chain is the list's `iter`; the script stops unless the emitted
Rust walks through `nikaia_std::par::`). The Nikaia package keeps its overflow
checks, as every Nikaia build does. Both run on Rayon's global pool, pinned by
`RAYON_NUM_THREADS`; the Nikaia side with `io-method = "blocking"`
(`docs/measuring.md`). It checks that both print the same bytes, then counts
each with callgrind (`--branch-sim=yes`): total instructions, and mispredicted
branches, conditional and indirect.

## The table

2026-11-22, commit `e18b66a6c`, valgrind 3.22.0, cargo 1.97.0, 4 threads,
10^6 elements, both sides printing the same bytes (`1000000 1499999500000`
and `1000000 5888890`):

| lambda | Nikaia: instructions | mispredicted | Rayon: instructions | mispredicted |
|---|--:|--:|--:|--:|
| can throw | 66,842,070 | 12,381 | 45,559,268 | 10,668 |
| allocates | 786,713,034 | 4,019,861 | 832,520,860 | 1,861,196 |
| (0 elements: start and end, `ebbbc1dd9`) | 624,358 | 11,679 | 458,506 | 8,388 |

**The chain whose lambda can throw costs 1.47 times Rayon's.** Nikaia
collects each element's `Result` into a list on the pool (19.0 M), then
`nikaia_std::par::kept_failures` walks that list on one thread, pushing each
value into a second list grown from empty and collecting that into the
result (33.0 M, half the program); Rayon's `collect::<Result<_, _>>` writes
the values straight into the result (26.0 M on the pool, 10.1 M of
`memcpy`, 5.0 M in `from_par_iter`). The rest is the sum after the chain:
14.0 M against 3.8 M, the overflow check every Nikaia build keeps, not the
chain.

**The chain that allocates costs no more than Rayon's**: Nikaia counts 5.5 % fewer
instructions. Both lower onto the same `rayon::slice::Iter`
(`nikaia_std::par::iter`), and the lambda body is the same `format!`; what
is left is the allocation the two `format!`s make. Nikaia mispredicts twice as
many branches: the extra is in `malloc`/`_int_malloc` (about 1 M conditional)
and indirect jumps (about 2 M), and was not traced further.

Rayon's twin with `x.to_string()` instead of `format!` counts 548,489,831
instructions: the integer's own `to_string` skips `core::fmt::write` and
`pad_integral`, which `f"{x}"` reaches through `format!`. That is the
f-string's lowering, not the chain's, so the row compares the same body.
