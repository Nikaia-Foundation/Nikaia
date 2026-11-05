---
agent: builder
model: opus
max_turns: 60
budget_usd: 12
---
# Brief: a `sync` function that may `panic()` runs at build time (#1108), and j5's `Vec`/`.remove(0)` goes

Facts: Tests and the gate, Probes, Lowering and lower-std, Prover

Score: a defect j5 found in this lane's own path (#1019 step 3b'), and the way around it j5 had to write into `comptime_tree.nika` (debt, `CLAUDE.md` *Fix, do not work around*); S=A, O=~0.2M tokens, C=~0.2M tokens; two `tools/*.nika` files, three tests.

## Problem
A `sync` function that may `panic()` cannot run at build time: `NK1152`, "Nothing says what it touches outside the program". Part II 10.2 (`docs/specification/20-nikaia-advance.md` around line 130) says a build-time run that stops, `panic()` included, is refused as `NK1244` in the program's words; the same function runs fine while the program runs. Because of it, j5 (974b9ac) made the build-time sub-program's `__nikaia_comptime_turn[T]()` collect into a `Vec` and call `.remove(0)` instead of returning from the turn.

Probe (architect, target/fast/nikaia at 69ea78d; `NK1152` at line 6 today, wanted `6`):
```nika
fn pick(n: i64) -> i64 sync {
	if n > 10 { panic("too big") }
	return n * 2
}

comptime X: i64 = pick(3)

fn main() {
	print(f"{X}")
}
```

## Decision (architect, 2026-10-31: a defect, Part II 10.2 already says it; no language change)
`panic` touches nothing outside the program, as it pauses nothing (Part I 7.2). One helper says so for both walks.

Cause (traced by the architect, sure): `panic` is in no ledger, so `calls.nika` `callee_named` returns `Callee::Opaque("panic")`. `sync.nika` `pausing_call` skips it through `a_stop` (line ~470); `touch.nika` `reached_call` (line ~246) sets `reach.unknown = true` for every `Callee::Opaque`, so the contract's `touches_known` is false and `build_eval.nika` `bt_may_not_run` (~589) refuses.

## Locations
* `crates/nikaia-std/src/tools/sync.nika` `a_stop`, `pausing_call`; `crates/nikaia-std/src/tools/touch.nika` `reached_call`, `reached_by`. Move `a_stop` where both reach it (e.g. `calls.nika`, beside `callee_named`, `pub fn callee_stops`) - one rule in one place; any other walk over `Callee::Opaque` that should skip `panic` (`locks.nika` line ~212 reads `Seen::Call`: check it, say what you found in `Body:`).
* `crates/nikaia-std/src/tools/comptime_tree.nika` `comptime_turn_function` (~107) and `comptime_turn_call` (~127).
* Generated files: `crates/nikaia-std/src/tools/lowered/{sync,touch,calls,comptime_tree}.rs` by `lower-std`.
* The generated build-time `main.rs` for a run with a project sits in `<project>/target/nikaia/build-time/comptime/<hash>/`; none is written when lowering refuses (j5's debrief).
* Unfixed binary for the fails-before run: `/home/user/nikaia-suite/target/fast/nikaia`, or copy `target/fast/nikaia` to your scratchpad before your first build.

## To build
1. The helper, used by `sync.nika` and `touch.nika`: a `Callee::Opaque` named `panic` reaches nothing. Tests in `tests/compiler/tests/`:
   - `a_comptime_runs_a_function_that_may_panic.nika` + `.stdout`: the probe (`6`);
   - `a_comptime_that_panics_is_refused.nika` + `.stderr`: the same function called as `pick(30)` is `NK1244` with `too big` (take the text the compiler prints; it must name `panic`'s words and the line).
   Both fail on the unfixed binary (say so under `Tests:`).
2. `comptime_tree.nika`: `__nikaia_comptime_turn[T: Struct]() -> R sync` returns the turn's value with `return` inside the `if` at the field, and ends with `panic("...")` after the loop (a turn that names a field always finds it; the words are yours, e.g. `"no field <name>"`); `comptime_turn_call` calls it without `.remove(0)`. Update the comments above both. The `a_comptime_*` and `comptime_*` tests keep passing (`--both-settings`).
   Commit both steps in one commit, `Fixes: #1108`, `Part: #1019`.

## Done when
Both new tests pass and failed before; `a_comptime_*`, `comptime_*`, `a_sync_function_*` and every `tests/compiler/tests` program carrying `NK1152` or `NK2202` pass; `lower-std` fixpoint clean; clippy, fmt, `pre-push-check.sh` green.

## Stop and report if
the turn function with `return` + `panic` is refused (a rustc `unused_braces` warning on the unrolled turn is #1110, not a refusal) by the checker or by rustc (say the message, `Needs: architect decision`, and keep step 1 alone in the commit); a `Callee::Opaque` walk other than the two named needs a different answer for `panic`.

## Not yours
`check_boxes.nika` (#1109, bug lane), the body-`comptime` naming warning (#1110, bug lane), winnow-grammar (j8), the grammar files (`grammar_*.nika`, `emit_grammar.nika`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Your prompt holds the facts you need from `docs/builder-facts.md`. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j9-1108-panic-at-build-time.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
