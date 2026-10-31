---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #938, a loop over an untyped list of numbers hands `&i64` to a by-value call (rustc)

Score: compiler bug (false acceptance: the checker passes, rustc refuses; every `for n in [1, 2]`-style loop whose element reaches a call), bugs-first default of `docs/priorities.md`. Filed 2026-10-29, re-probed by the architect at 4fa242a6.

## Issue #938 (copied, probes widened at 4fa242a6)
```nika
fn twice(n: i64) -> i64 {
    return n * 2
}

fn main() {
    let row = [1, 2]
    let mut total = 0
    for n in row {
        total += twice(n)
    }
    println(f"{total}")
}
```
`nikaia check` accepts it; `nikaia run` fails in rustc: *Mismatched types* at `twice(n)`, "Consider dereferencing the borrow". Lowered: `for n in row.iter() { total += twice(n); }`.

Probe matrix (architect, 4fa242a6):
| Row | Today |
| :--- | :--- |
| `let row = [1, 2]`, `for n in row { twice(n) }` | rustc mismatched types |
| `for r in [[1, 2], [3]] { for n in r { twice(n) } }` | rustc mismatched types |
| `let grid = [[1, 2], [3]]`, nested `for r in grid { for n in r ..}` | rustc mismatched types |
| `for n in grid[0] { twice(n) }` | rustc mismatched types |
| `let row: Vec[i64] = [1, 2]`, `for n in row { twice(n) }` | runs, `6` |

So the mechanism exists and works for a declared element type: the checker records `copied_loop_bindings` and the emitter writes `let n = nikaia_std::num::value(n);` at the body's opening. Only the element type of an **untyped** list literal (and lists of them) misses it.

## Locations
* Where the fact is recorded: `check_core_stmt.nika:1502-1506` (`let copied = lent && !mut_view && name != "_" && a_copy_by_value(element)`), `element` from `elements` above it.
* `a_copy_by_value`: `check_types.nika:89` (counts `Ty::Named` with a number name, `bool`, `char`, `scalar`). Unverified cause: the element of `[1, 2]` arrives as a type this does not count (an integer-literal or not-yet-defaulted type, or `Ty::Unknown`). Find which with one probe; fix where the element type is known, not by widening `a_copy_by_value` to `Unknown` (that would copy a non-number).
* The emitter's use (no change expected): `emit_stmt.nika:979-987`.
* The model test: `tests/compiler/tests/a_loop_over_copies_binds_the_value.nika`.

## To build
1. The checker records an untyped list literal's integer element (and the inner element of a nested one, and an indexed row `grid[0]`) as `copied_loop_bindings`, as it does for `Vec[i64]`. Test `tests/compiler/tests/a_loop_over_an_untyped_number_list_copies.nika` + `.stdout` (`6`, `12`, `6`, one a line), the program below; run by the architect at 4fa242a6: three rustc errors (lines 9, 16, 22). Commit `check: a loop over an untyped number list binds the value (#938)`, `Fixes #938`.

```nika
fn twice(n: i64) -> i64 {
    return n * 2
}

fn main() {
    let row = [1, 2]
    let mut total = 0
    for n in row {
        total += twice(n)
    }
    println(f"{total}")
    let grid = [[1, 2], [3]]
    let mut t = 0
    for r in grid {
        for n in r {
            t += twice(n)
        }
    }
    println(f"{t}")
    let mut u = 0
    for n in grid[0] {
        u += twice(n)
    }
    println(f"{u}")
}
```
If one row has a second cause (the fix covers the others, that row still fails), name the row and the rustc message under `Found:`, commit the test without that row and the fix, and say so in `Body:` (the architect files it).

## Done when
The test passes through `nikaia test` and fails on the unfixed compiler; `a_loop_over_copies_binds_the_value` still passes; `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the guard set of `.claude/agents/builder.md` green; pushed. Changelog: one line (a loop over a list of number literals with no declared type binds each number by value, as one over `Vec[i64]` does). Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/check_*.nika`, `emit_stmt.nika`, their lowered files (and `lowered.rs`) and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b938-loop-copy-inferred-list.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
