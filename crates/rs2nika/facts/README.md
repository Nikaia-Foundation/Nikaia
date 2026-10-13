# facts

Rust's types for the Rust-to-Nikaia converter (`../convert`, #579), read from
rust-analyzer on stable (ADR-001 D5), not from a nightly:

```sh
rustup component add rust-analyzer
rust-analyzer lsif path/to/crate > crate.lsif
cd crates/rs2nika/facts && nikaia run -- [--stats] [--src DIR] crate.lsif
```

One line a fact, tab-separated:

```
<file>  <line>:<col>-<line>:<col>  <kind>  <value>
src/lib.rs  3:15-3:18  method  core::slice pub const fn get<I>(&self, index: I) -> Option<&I::Output>
src/lib.rs  3:9-3:10   local   Option<&u32>
src/lib.rs  5:21-5:22  op      usize
```

The file is relative to the LSIF project root (absolute outside it); lines and
columns count from 1, columns in UTF-16 units as LSIF gives them.

| Kind | Where | Value |
| :--- | :--- | :--- |
| `method` | a call (not a definition) of a method or associated function | the resolved path and the signature line of rust-analyzer's hover |
| `local` | the definition of a `let` binding, a parameter, a closure parameter | its type |
| `op` | an operator resolved to a `core::ops`/`core::cmp` trait method | the result type (`&T` for a deref: generic, not instantiated) |

How it is read: a `range` reaches a `resultSet` by `next`, the result set its
`hoverResult` by `textDocument/hover`; `contains` gives the range's document,
an `item` edge out of a `definitionResult` marks the ranges that define.
rust-analyzer writes an operator's range a column wide on each side as well,
so one `+` gives three `op` lines.

Nikaia's std has no JSON reader; `src/main.nika` reads exactly the line form
rust-analyzer writes.

`--stats` writes to stderr the facts per kind and the method-call shapes
`.name(` / `.name::` in the project's own `.rs` files that no `method` fact
starts at (`--src DIR` reads the sources from DIR instead of the paths in the
dump).

## Measured on `crates/nikaia`

`rust-analyzer lsif crates/nikaia` (rust-analyzer 1.97.0, stable), measured
once on a shared machine:

| | |
| :--- | :--- |
| `rust-analyzer lsif` | about three minutes (it loads the whole workspace) |
| LSIF size | 92 MB, 885,541 lines, 621 documents (the workspace's crates too) |
| `facts` over it | well under a minute |
| facts in `crates/nikaia` | 54,288: 30,815 `method`, 19,115 `local`, 4,358 `op` |
| facts in all documents | 69,114 |
| method calls without a resolved method | 2,414 of 25,970 (9 %); in `src/` 49 of 11,155 (0.4 %), the rest in `tests/`, `build.rs`, `examples/` |

## Test

`nikaia test` runs `tests/probe`: the crate in `tests/probe.in` (`src/lib.rs`)
with its dump `out.lsif` (project root rewritten to `file:///raprobe`) and the
facts it must give: `v.get` is the slice's `get`, `m.get` `HashMap::get`,
`a: Option<&u32>`, `n: usize`, `+` gives `usize`.
