# What fallible collections cost

[ADR-327](../../docs/specification/adr/adr-327.md) D4, D5 has a task's lists,
texts and maps take their memory from a budget, and a full budget ends the task
instead of aborting the process. That needs collections whose growth can
fail: `Vec<T, A>` from `allocator-api2` and `hashbrown`'s map with the same
allocator. This counts what they cost against today's `Vec`, `String` and
`std`'s `HashMap` (#490, step 1).

Two workloads, each written twice, the same steps on both sides:

| binary | what |
|---|---|
| `brc_std`, `brc_fallible` | 1BRC's aggregation: a map from station to min/max/sum/count, then the sorted names written out. The map's table, the list of names and the output text are the collections; the map is `nikaia-std`'s own, with the trusted hash, on both sides: outside on `brc_std`, placed in the block on `brc_fallible` |
| `tasks_std`, `tasks_fallible` | `benches/taskblock`'s workload: many short tasks, each building a request and parsing it into owned parts, then writing a response (22 allocations and 7 reallocations a task) |

The fallible side is `nikaia-std`'s own (`nikaia_std::block`, #490 step 3,
reached through `src/budget.rs`): every collection is placed in the running
task's block, 16 pages of 4 KiB each counting what it holds (ADR-327 D12),
growth goes through `try_reserve`, and a refusal ends the task. Each of
`tasks_fallible`'s tasks has a block of its own, taken from the thread's
spares and handed back at its end; `brc_fallible` is one long `main` that
runs no task, so its handle places outside, with the global allocator, and
what is counted there is the handle and the fallible collections.

```sh
benches/fallible/fallible.sh                # 200 000 rows, 100 000 tasks
benches/fallible/fallible.sh 50000 20000    # quicker
```

The script builds with `--release` (`opt-level = 3`, no LTO), checks that both
sides of each workload print the same bytes, counts each with callgrind
(`--branch-sim=yes`) and subtracts a run over nothing, so what is left is per
row or per task.

## The table

On `nikaia_std::block`, `std`'s map placed by its handle (#490 step 3, its
second part):

| program | instructions | mispredicted |
|---|---:|---:|
| `brc_std` (per row) | 383.1 | 3.53 |
| `brc_fallible` (per row) | 374.5 | 3.53 |
| `tasks_std` (per task) | 17872.4 | 162.70 |
| `tasks_fallible` (per task) | 11114.0 | 91.83 |

- **A task on its block costs 37.8% fewer instructions** than on `malloc`,
  with 44% fewer mispredicted branches: what `benches/taskblock` measured for
  `pages` (-38.5%), now through `allocator-api2`'s collections and a handle.
- **Outside a task the handle costs nothing a row.** Both sides do the same
  work a row (a search that finds the station); the 2% between them is where
  the code lands, not work. The first part's +1.1% (385.3 against 381.1) was
  not the handle: `brc_fallible` asked the map for room before every row (3.0
  of its 4.2 instructions, measured by leaving the ask out). The map now asks
  only where it inserts, and only when its table is in a block.

The first part (0.0.1366), with `hashbrown`'s map and a room check a row:

| program | instructions | mispredicted |
|---|---:|---:|
| `brc_std` (per row) | 381.1 | 3.52 |
| `brc_fallible` (per row) | 385.3 | 3.66 |

Step 1's allocator counted every byte against a budget and refused past it,
with the system's memory (0.0.603):

| program | instructions | mispredicted |
|---|---:|---:|
| `brc_std` (per row) | 393.8 | 3.51 |
| `brc_fallible` (per row) | 394.4 | 3.67 |
| `tasks_std` (per task) | 17872.4 | 181.12 |
| `tasks_fallible` (per task) | 17882.3 | 193.21 |

## Reading it

- **`allocator-api2`'s `extend_from_slice` copies one element at a time** on
  stable Rust (`extend(iter().cloned())`, no specialisation). Used as it is,
  it doubled the tasks' count. `nikaia_std::block::extend` copies with one
  `copy_nonoverlapping`, as `std`'s `Vec<u8>` does.
- **The fallible text is a `List<u8>`**, handed out as a `str` without checking
  its UTF-8 again, as `String` does: every byte in it came from a `str`.
  Checked, it cost about 230 instructions a task more.
- **What is not measured here**: a program the compiler emits against these
  collections (the emitter writes `Vec` and `String` until #490's type switch),
  and a refusal and the task it ends.
