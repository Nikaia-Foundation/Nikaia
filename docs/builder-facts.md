# Builder facts

Fixed facts a builder otherwise looks up itself (counted over 48 builder runs,
2026-10-25: median 2, up to 19 `grep -r` calls per run to find where a name is
defined; four `pre-push-check` failures on the same badge; seven runs puzzled by
the same dirty lock file). Read once; the brief adds only what is specific.

## Where things live

| What | Where |
| :--- | :--- |
| Checker, moved parts | `crates/nikaia-std/src/tools/check_*.nika` (87 files); Rust adapter in `crates/nikaia/src/check/` |
| Emitter | `crates/nikaia/src/emit/mod.rs`, `emit/host.rs`; moved parts `tools/emit_*.nika` |
| `std` methods by receiver | `std_receivers` in `tools/check_core_method.nika` |
| Generated Rust of a tools file | `tools/lowered/<file>.rs`, index `tools/lowered.rs` (never edit, `lower-std` writes them) |
| Program tests (new tests) | `tests/compiler/tests/NAME.nika` + `.stdout` / `.stderr` |
| Rust tests by NK code | `grep -rln NK1234 crates/nikaia/tests/it/` |
| Specification | `docs/specification/10-nikaia-light.md`, `20-nikaia-advance.md`, `30-nikaia-tooling.md` (code table) |
| A name, Rust or Nikaia | `nikaia run scripts/where.nika -- NAME...`: definitions (`fn`, `struct`, `enum`, `type`) with file:line, the files that call it; for an `NKnnnn` the tests and the spec row (a field is not found: grep it) |

## Traps already met

* `tests/compiler/nikaia.lock` is git-ignored, like the other test
  packages' locks (it changes with every compiler build). Never commit it.
* `scripts/pre-push-check.sh` fails with *out of date: self-hosted.svg* when a
  `.nika` changed the self-hosting share: run `python3 scripts/badges.py` and
  add `assets/badges/self-hosted.svg` to `Files:` (the only badge you touch).
* `cargo test ... --test it -- a:: b::`: the `--` before the filters is needed.
* A `nikaia run` script cannot `use` another script and cannot find its own
  binary; a test program cannot set `RUSTFLAGS` or see git state. Such a
  change is checked by hand runs; say so under `Deviation`.
* Valgrind cannot run a binary built for an AVX-512 CPU: name `x86-64-v3`.
* Builders have no GitHub: the brief holds the issue text. If it does not,
  say so under `Deviation` and build from the brief's title only.
* Syntax gaps of `tools/*.nika`: the list in `CLAUDE.md`, *Moving Rust into
  Nikaia*; `crates/rs2nika/convert --lint F.nika` flags the known ones.

* Guard tests (`range_spelling`, `shipped_ledger`, the std ledger in
  `contracts`) scan the whole repository: a script's `a..b` or a new
  `unsafe fn` fails them. They are in the gate (builder.md) for that reason.

## Commands, exactly

The cycle, clippy beside the tests, the report and `commit.nika`:
`.claude/agents/builder.md`. Disk at the end: the same file, last step.
