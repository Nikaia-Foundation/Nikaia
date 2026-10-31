---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief: #1107, a `ref T?` map view unwrapped by `?? return null` or a `match` binding loses `ref`

Blocks: c1c-ledger-lookup.md (C1c: `LedgerOps::lookup` moves to `(String, ref FnContract)?`; C1b, the `stored` fold, runs meanwhile)

Score: compiler bug, a wrong refusal of what Part I allows (an element read is a view, ADR-282 D7, ADR-293 D28), blocks a self-hosting move. Cause suspected, not traced to the line: medium.

Facts: Where things live, Traps already met, Commands

## Issue #1107

A `ref T?` view (a map index into a `ref` parameter) unwrapped by `?? return null` or by a `match` binding loses its `ref`: the value is typed as owned `T`, so it cannot be bound to a `ref T` local nor handed back as the `ref T` part of a tuple. Part I (the views section, `let first: ref Expr = given[0]`) and ADR-282 D7 make an element read a view; the unwrapped value is the same element. Found tracing C1b (`LedgerOps::lookup`, `contracts/mod.rs:548`: `self.functions.get(name).map(|c| (name.to_string(), c))`), which it blocks.

## Probe (checked by the architect on `target/fast/nikaia` at 7ad2ea08)

```nika
use std::collections
struct C { n: i64 }
fn lookup(m: ref collections::BTreeMap[String, C], k: String) -> (String, ref C)? {
    return match m[k] {
        null => null,
        c => (k.clone(), c),
    }
}
fn main() {
    let mut m = collections::BTreeMap()
    m.insert(f"a{1}", C { n: 5 })
    let hit = lookup(m, f"a{1}")
    println(f"{hit?.0 ?? "-"} {(hit?.1)?.n ?? 0} {lookup(m, f"b{1}")?.0 ?? "-"}")
}
```

Expected `a1 5 -`. Got NK1104 ``You're returning `(String, C)?`, but the function is declared to return `(String, ref C)?` ``.

Rows (same `C`, `m: ref collections::BTreeMap[String, C]`, `k: String`, `xs: ref Vec[C]`):

| body | result |
| :--- | :--- |
| the probe's `match` into `(String, ref C)?` | NK1104 `(String, C)?` |
| `let c: ref C = m[k] ?? return null` | NK1103 "This value is `C`, but the `let` declares `ref C`" |
| `let c: ref C = m.get(k) ?? return null` | NK1103, the same |
| `let c = m[k] ?? return null` / `return (1, c)` into `(i64, ref C)?` | NK1104 `(?, C)` |
| `let c = m[k] ?? return null` / `return c` into `ref C?` | NK1104 `C` |
| `return m.get(k)` into `ref C?` | NK1104 `C?` (`m[k]` passes) |
| `return m[k]` into `ref C?` | passes (must stay) |
| `let c: ref C = xs[0]` / `return (1, c)` into `(i64, ref C)?` | passes (must stay) |

## To build

1. Reproduce every row (`Tests:`).
2. Trace where the unwrapped value's type drops the view (the `??` with a jump, the `match` arm binding over a `T?` scrutinee, and `BTreeMap.get` over a `ref` map against `m[k]`), and keep it a view of the same read. Lowering: `Option<&C>` / `(String, &C)`; any lowering rustc accepts.
3. `lower-std` and its fixpoint.
4. Test `tests/compiler/tests/an_unwrapped_map_view_stays_a_view.nika` + `.stdout`: the probe and rows 2-6 as functions. Fails on the unfixed binary.

Commit `check: a view unwrapped by ?? or a match stays a view (Fixes #1107)`.

## Suspected location (medium confidence)
* `??`: `check_core_ops_coalesce`, `crates/nikaia-std/src/tools/check_core_ops.nika:755`. It records `map_read_views` for a map read past a jump (:784) but returns `check_core_ops_inside(left)`, which probably drops the `Ty::Ref` the index read carried; compare with how `return m[k]` keeps `ref C?`. A `let` with a `ref` annotation then compares in `check_core_stmt.nika` (the `Expr::Coalesce` arms at :123-185, `check_core_stmt_element_read` :551).
* `match` over a `T?` view: the arm binding's type, where the `null` arm narrows the scrutinee (`check_narrow.nika`, `check_narrow_a_maybe_read` :355). Unsure; trace first.
* `m.get(k)` against `m[k]`: the method's result type over a `ref` receiver (`check_core_method.nika`).
Fix each at its cause; if two functions repeat the same rule, keep them agreeing and name them in `Body:`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`; the tests named `coalesce|view|map_read|element_read` pass; `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/check_*.nika`, their lowered twins, `lowered.rs`, the emitter files for the lowering (`emit_*.nika`) and the new test. Or you hit an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b26-1107-unwrapped-map-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
