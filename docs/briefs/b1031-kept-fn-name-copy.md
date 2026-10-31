---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1031, a kept fn copied from a name, a field or a tuple part is moved, not cloned (rustc)

Score: compiler bug (false acceptance: the checker passes, rustc refuses; every second use of a copied function value), bugs-first default of `docs/priorities.md`. Found by b1018; the same family as #1018 and #1008.

## Issue #1031 (copied)
Found by b1018 (report of d5995dbd), widened by the architect's probes at d7e56e49.

### Problem

A function value is a copy that shares its closure (Part I, *A function type stands wherever a type does*). Read from a list by index it is cloned since #1018; read from a **name**, a **field** or a **tuple part** in a value position it lowers to a move of the `Kept`, so a second read passes the checker and rustc refuses.

```nika
fn main() {
    let f: fn(i64) -> i64 = fn(x) { x + 5 }
    let g = f
    println(f(1))
    println(g(2))
}
```
Today: rustc `Borrow of moved value: f`. Expected `6`, `7`.

Probe matrix (all at d7e56e49, `nikaia run`):
| Row | Today |
| :--- | :--- |
| `let g = f` then `f(1)`, `g(2)` | rustc `Borrow of moved value: f` |
| `struct H { f: fn(i64) -> i64 }`: `let a = h.f; let b = h.f` | rustc `Use of moved value: h.f` |
| `H { f: f }`, then `H { f: h.f }`, then `let a = h.f` | rustc `Use of moved value: h.f` |
| `fs.push(f)` twice, then `f(2)` | rustc `Use of moved value: f` |
| `let t = (f, 1); let u = (f, 2)` | rustc `Use of moved value: f` |
| `keep(f)` to an owning `fn` parameter, then `f(1)` | 6, 7 (fine: `hands_on_kept`, emit_core_args.nika:369, #1008) |
| `let a = fs[0]` | fine since #1018 |

### Locations
* #1018's fact and its use (the model): `kept_fn_reads` in `check_facts.nika:112`, recorded in `check_core_value_of`, `check_core_expr.nika:279` (`Expr::Index` arm), written in `emit_core_index_read`, `emit_core_reads.nika:396` (`.clone()`).
* Name and field reads: `check_core_value_of`, `check_core_expr.nika:242` (`Expr::Path` -> `check_core_expr_path`), `:252` (`Expr::Field` -> `check_core_field`); emitted in `emit_expr.nika:38`/`:313` (`Expr::Path`) and `:381` (`emit_core_field`).
* The argument clone that already works: `hands_on_kept`, `emit_core_args.nika:366-370`; a named function used as a value is `facts.named_fn_values` (not a `Kept` place, no clone).

### To build
1. The checker records every name or field read whose type is `Ty::Fn` and that names a place (a local, a parameter, a field; not a named function, `named_fn_values`), keyed by the read's `node_key`, in a fact beside `kept_fn_reads`; the emitter writes `.clone()` after such a read where it stands as a value (a `let` initialiser, an argument, a struct literal field, a tuple part, a list element, a `return`), and not where it is the callee of a call (`f(1)`, `h.f(2)`) or already duplicated by `hands_on_kept`. Test `tests/compiler/tests/a_kept_fn_copied_from_a_name_or_field.nika` + `.stdout`: rows 1-5 in one program (text in the brief), printing `6`, `7`, `21`, `13`, `21`; it fails today at its line 6 (rustc `Borrow of moved value: f`). Commit `emit: a kept fn copied from a name or a field shares the closure (#1031)`, `Fixes #1031`.

### The test program (run by the architect at d7e56e49: rustc `Borrow of moved value: f` at line 6)
```nika
struct H { f: fn(i64) -> i64 }

fn main() {
    let f: fn(i64) -> i64 = fn(x) { x + 5 }
    let g = f
    println(f(1))
    println(g(2))
    let h = H { f: f }
    let k = H { f: h.f }
    let a = h.f
    let b = h.f
    let c = k.f
    println(a(1) + b(2) + c(3))
    let mut fs: Vec[fn(i64) -> i64] = []
    fs.push(f)
    fs.push(f)
    let d = fs[1]
    println(d(1) + f(2))
    let t = (f, 1)
    let u = (f, 2)
    let (p, _) = t
    let (q, _) = u
    println(f(1) + p(2) + q(3))
}
```
Expected `.stdout`: `6`, `7`, `21`, `13`, `21`, one a line. Rows 1, 2, 4 and 5 were each traced to the move separately (one program per row, the rustc message names the copied place); a tuple part read back by a `let (p, _) = t` pattern is not traced separately: if it needs more than the `.clone()` at `(f, 1)`, say so in `Body:`.

Not this brief: `println(f(1) + h.f(2) + k.f(3))` in one expression gave `NK2605` (*can fail, because `H.f` can fail*), unprobed further; `h.f(2)` alone runs. Keep field calls out of the test; if you meet it, it is a `Found:`.

## Done when
The test passes through `nikaia test` and fails on the unfixed compiler; `lower-std` fixpoint clean; `a_kept_fn_read_by_index` still passes; `cargo build --profile fast -p nikaia`; the guard set of `.claude/agents/builder.md` green; pushed. Changelog: one line (a function value copied from a name, a field or a tuple part shares its closure, as Part I says, instead of a lowering rustc refused).

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/emit_*.nika`, `check_*.nika`, `check_facts.nika`, their lowered files (and `lowered.rs`) and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1031-kept-fn-name-copy.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
