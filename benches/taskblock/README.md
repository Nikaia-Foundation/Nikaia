# What a memory block per task costs

[ADR-327](../../docs/specification/adr/adr-327.md) D4-D10 has a program allocate
from a block per task, with a full block ending the task, and memory a task
frees can be allocated again (D10). This counts what that costs against the
system allocator, on many short tasks that each read a request and write a
response, as a server's do, and on one long task that does the same in a loop
(a connection loop, `main`) and frees each request's memory before the next
(#490, step 1).

Six binaries, one file each, the same workload (`src/workload.rs`); each
differs from the others only in its `#[global_allocator]`:

| binary | allocator |
|---|---|
| `global` | the system allocator (`malloc`) |
| `budget` | the system allocator, with every task's bytes counted against a budget of 64 KiB (D5's check without a block) |
| `block` | a block of 64 KiB per task: an allocation moves a pointer, nothing is given back one by one, the block is empty again when the task ends; a full block ends the task |
| `reuse` | the block cut into size classes (powers of two) with a free list each inside the block (`src/classes.rs`): a free list first, then the untouched rest, then a larger free piece split |
| `bumpreuse` | the same classes, the untouched rest first and the free lists only once the block is full |
| `pages` | the block as 16 pages of 4 KiB (`src/pages.rs`): a pointer moves through the current page and the page counts what it holds; a page whose count drops to nothing starts over (the current one) or is free again; more than a page takes a run of free pages |

`block` cannot run the long task: it never reuses freed memory, so its block
is full within 100 requests.

```sh
benches/taskblock/taskblock.sh          # 100 000 tasks
benches/taskblock/taskblock.sh 20000    # quicker
```

The script builds with `--release` (`opt-level = 3`, no LTO), checks that all
binaries print the same bytes in both workloads, counts each with callgrind
(`--branch-sim=yes`), and subtracts a run of no task, so what is left is
the tasks' own.

## The tables

100 000 requests; a request makes 22 allocations and 7 reallocations.
Percentages against `global`.

Many short tasks:

| allocator | instructions / task | mispredicted / task |
|---|---:|---:|
| `global` | 17410 | 167.9 |
| `budget` | 17809 (+2.3%) | 174.7 |
| `block` | 10188 (-41.5%) | 88.8 (-47%) |
| `reuse` | 11179 (-35.8%) | 79.0 (-53%) |
| `bumpreuse` | 11056 (-36.5%) | 79.1 (-53%) |
| `pages` | 10713 (-38.5%) | 82.6 (-51%) |

One long task:

| allocator | instructions / request | mispredicted / request |
|---|---:|---:|
| `global` | 17438 | 175.7 |
| `budget` | 17835 (+2.3%) | 182.0 |
| `reuse` | 10978 (-37.0%) | 91.0 (-48%) |
| `bumpreuse` | 11438 (-34.4%) | 85.7 (-51%) |
| `pages` | 10696 (-38.7%) | 80.6 (-54%) |

- **A block is cheaper than `malloc`, not dearer**: 41% fewer instructions and
  half the mispredicted branches. What it saves is `malloc`'s and `free`'s own
  work (about 7 200 instructions a task, `callgrind_annotate`); taking from the
  block and emptying it costs a few instructions per allocation.
- **Reusing freed memory costs 3 to 6 points of that**, in every form
  measured: a free does work where the bump block's does none (a count taken
  off a page, or a piece put on a list), and an allocation rounds to a class or
  counts. `pages` is the cheapest form on both workloads, 525 instructions a
  short task above `block`, and has fewer mispredicted branches than `block`.
- **Starting as a bump block does not pay**: `bumpreuse` is 123 instructions
  a short task under `reuse` and 460 a request over it in the long task. The
  frees have to be kept from the start either way, or there is nothing to
  reuse once the block is full; a real switch at the first full block would
  find nothing to switch to.
- **Size classes waste and fragment**: a piece is rounded up to a power of two
  and pieces are not merged again, so a class whose list is empty splits a
  larger piece. `pages` keeps a page whose count is not yet zero: one value
  held for long keeps 4 KiB.
- **Counting a budget over `malloc` costs 2.3%** (400 instructions a task):
  the thread-local count on every allocation, reallocation and free.

## Reading it

- **`global` declares the system allocator as the global one**, as the others
  declare theirs. Without a declaration every allocation goes through
  `std`'s default shims (`__rdl_alloc`, `__rdl_realloc`), which are not inlined
  and cost about 420 instructions a task more - a difference of the
  declaration, not of a strategy.
- **`malloc`'s count depends on where the heap starts**: the same binary called
  by another path (`./target/release/global` against its absolute path) moves
  `_int_malloc` and `_int_free` by about 177 instructions a task, because the
  argument vector shifts the heap. The script always calls the same way.
- **Which binary is faster is not only the allocator's**: LLVM inlines the
  workload differently into each binary (the string search is a function of
  its own in `block` and inlined into `run` in `reuse`), so a comparison of single
  functions across binaries misleads; the totals are what is compared.
- **What is not measured here**: a block that is given back to the system when
  a task ends (ADR-327 §6), a task that holds more than one block, values that
  cross from a task into a shared value, and fallible growth of the
  collections themselves (`benches/fallible`).
