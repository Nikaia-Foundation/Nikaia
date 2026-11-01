---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
effort: low
---
# Brief: #1129, a lambda handed round a call cycle through two files of a package is not `&dyn Fn`

Blocks: none (latent: found by converter cv2; rs2nika is a package lowered file by file, so the next code-parameter cycle across its files meets it)

Score: compiler bug, checker accepts and rustc refuses (an accepted program must build). Location traced, sure; the fix mirrors 72e7789c (effort low).

Facts: Where things live, Traps already met, Commands

## Issue #1129

**A `sync` code parameter handed round a call cycle through two files of a package is not `&dyn Fn`** (ADR-277 D11, #1023): rustc stops with "Reached the recursion limit while instantiating `ping::<{closure}>`".

Found by cv2 (converter lane, commit 72e7789c, which fixed the same one-file blind spot for the pausing graph). Probed by the architect, `target/fast/nikaia` at 8b125868, scratch package `cyc`:

`src/main.nika`
```nika
pub fn alt(name: String) -> i64? sync { return name.len() }

pub fn ping(n: i64, known: fn(ref String) -> i64? sync) -> i64 sync {
    if n == 0 { return known("a") ?? 0 }
    let more = fn(name) { known(name) ?? alt(name) }
    return pong(n - 1, more)
}

fn main() { println(f"{ping(2, fn(_name) { null })}") }
```
`src/beside.nika`
```nika
pub fn pong(n: i64, known: fn(ref String) -> i64? sync) -> i64 sync { return ping(n, known) }
```
Expected `1` (as `tests/compiler/tests/a_lambda_handed_round_a_cycle_runs.nika`, the one-file form, prints). Got rustc's recursion limit at `beside.nika:1:71`.

## Locations

Traced, sure: `code_on_a_cycle` (`crates/nikaia-std/src/tools/emit_reach.nika:173`) builds its graph from `items`, the lowered file's own items; its caller `emit_function.nika:551` passes `world.items`. The pausing graph had the same fault; 72e7789c split it into `pausing_edges` (one file) + `pausing_reach` (closure), merged over the file and its `beside` units in `pausing_reach` (`crates/nikaia/src/emit/mod.rs:1269`) and handed to the emitter as `pausing_reach: reach` (mod.rs:1000).

## To build

1. Split `code_on_a_cycle` the same way: the per-file edges of the code-running functions (`code_cycle_edges`) and the cycle test over a merged graph, both in `emit_reach.nika`; the emitter gets the merged graph (or the set of functions on a cycle) computed once per program beside `pausing_reach`, built over the file and its `beside` units. The Rust adapter only loops over the units, as `pausing_reach` does; all logic stays in Nikaia.
2. Test: the probe as two files of `tests/compiler` (`src/main.nika` gains `ping`/`alt`, renamed so they do not clash; `src/` gets the `pong` half), the program `tests/compiler/tests/a_lambda_round_a_cycle_across_files.nika` + `.stdout`, as `pausing_cycle_across_files` did. Fails on the unfixed binary.
3. `lower-std` and its fixpoint.

Commit `emit: a lambda handed round a cycle through two files is &dyn Fn (Fixes #1129)`. `emit_core_leaves.nika` and its lowered file hold another agent's uncommitted edit: do not stage them.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b31-1129-code-cycle-across-files.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
