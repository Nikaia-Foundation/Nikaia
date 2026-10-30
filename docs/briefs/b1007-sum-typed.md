---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #1007, `sum()` of a typed number sequence lowers without a type (run 2)

Score: bug lane, a false acceptance (the checker passes, rustc refuses; S=2) in a one-line form any move that adds numbers writes. K3a part 2 waits on the owner (#1012), so this runs first. Builders have no GitHub: the issue text is copied below. Run 1 (`b1007-sum-typed.md.run1.report.md`) stopped because no checker fact carries the call's result type to the emitter; this run adds that fact (the brief now allows the checker files).

## Problem

`sum()` over a sequence of a typed number list lowers to a bare `.sum()`; rustc cannot infer the result type when nothing annotates it (found by b1006). #931 (5162370e) wrote the type on the `let` only for an *open* number literal list.

## Probe (run by the architect on a5663bf6, `target/fast/nikaia run`; all rows but the annotated ones fail)

```nika
fn total(v: Vec[i64]) -> i64 {
    let t: i64 = v.iter().sum()
    return t
}

fn main() {
    let v: Vec[i64] = [3, 4]
    println(v.iter().sum())             // 7
    let s = v.iter().sum()
    println(s)                          // 7
    println(total(v) + v.iter().sum())  // 14
    let w: Vec[f64] = [1.5, 2.0]
    let t: f64 = w.iter().sum()
    println(t)                          // what println prints for 3.5
    println(w.iter().sum())             // the same
}
```
Today: rustc "type annotations needed" (`Sum<i64>`, `i64: Add<_>`).

## How the type reaches the emitter (resolved for you)

The emitter reads checker facts by node key (`node_key(e)` = `id_of(e) as i64`); there is **no fact with a call's result type**: `std_receivers` (receiver's ledger type), `open_items` (open literals only, #931) and `collected_into` (a set) do not give `i64` here. Add one:

* `crates/nikaia-std/src/tools/check_facts.nika`: a new field beside `collected_into` (line 218), e.g. `pub summed: collections::BTreeMap[i64, String]` (call node -> Rust number type, `"i64"`, `"f64"`, ...), with its doc comment, and its empty value in the constructor beside line 502.
* `crates/nikaia-std/src/tools/check_core_method.nika:568`, `check_core_method_called`: `value` is the checked result of `check_core_call_on`. When `name == "sum"` (and only for the core sequence method, not a program's own `sum`) and `value` is a plain number type (`Ty::Named` with no args, a number name), insert `id_of(expr) as i64 -> name` into the new fact.
* `crates/nikaia-std/src/tools/emit_core_calls.nika` ~1176-1186, `emit_core_method_call_body`: beside `collect`'s `::<Vec<_>>`, push `::<T>` after `sum` when `self.facts.summed[on_node]` holds a type. Check which node `on_node` is at this point (the call's or the receiver's: `emit_core_calls.nika:1480` passes `node_key(receiver)`, so the call node may need to be handed in, or the fact keyed by the receiver; decide by reading, say which in `Body:`).
* Their lowered twins: `crates/nikaia-std/src/tools/lowered/{check_facts,check_core_method,emit_core_calls}.rs` (written by `lower-std`, never by hand).
* Test beside: `tests/compiler/tests/a_number_list_literal_summed_is_typed.nika` (must still pass).

## To build
1. The fact, the checker write and the emitter read above; `lower-std` fixpoint (lower-std, build, lower-std: the second run changes nothing; only files that call `sum` or the three edited files change). New test `tests/compiler/tests/a_typed_list_summed_needs_no_annotation.nika` + `.stdout` holding the probe rows (take the `.stdout` from the real run; say so in `Body:` for the `f64` lines). Any turbofish rustc accepts is fine (`.sum::<i64>()`); a `ref` element stays `Sum<&i64>` on the Rust side, the turbofish names the result. `Fixes #1007`.

## Done when
The new test passes through `nikaia test` (copy it to the scratchpad to `nikaia run` it alone first), `a_number_list_literal_summed_is_typed` still passes, the `lower-std` fixpoint is clean, the gate of `.claude/agents/builder.md` is green; pushed. Commit subject: `check, emit: sum() names its result type (#1007)`, `Fixes #1007`. Changelog: one line (a program that rustc refused now runs).

## Stop and report if
An edit outside the three `.nika` files above, their lowered files, `lowered.rs`'s index lines, other lowered files `lower-std` rewrites because they call `sum`, and the test; an error in another agent's file.

**Not yours, never stage**: `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}` and every `docs/briefs/*` file that `git status` shows modified or untracked except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1007-sum-typed.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
