---
agent: builder
model: opus
max_turns: 80
budget_usd: 20
---
# Brief: a call with type arguments in brackets is named in the effect walk (#1095; unblocks #1087 step 2, #1019 step 3b')

Facts: Traps already met

Score: blocks j5's step 2 (`comptime c = wire(f)` through a generic value function in the build-time sub-program), so #1019 step 3c; S=A, O=~0.6M tokens, C=~0.5M tokens; one walk in `foreign.nika`. A defect (kind 3): `sync` and `comptime` are specified over any call the checker resolves.

## Problem (probe both with `target/fast/nikaia run` at your start, in your scratchpad)
```nika
fn id[T](x: T) -> T sync {
	return x
}

fn v() -> i64 sync {
	return id[i64](3)
}

fn main() {
	println(f"{v()}")
}
```
Today (architect's probe on 417abcf): `error[NK2202]: v is sync, but it calls something this compiler cannot resolve, which can pause.` at 6:2 (an NK1233 warning comes first, #514, not yours). Wanted: `3`.

```nika
struct Contact {
	name: String,
	age: i64 = 7,
}

fn turn[T: Struct]() -> String sync {
	for f in T::fields {
		if f.name == "age" { return f.name.clone() }
	}
	return ""
}

fn v() -> String sync {
	return turn[Contact]()
}

comptime W: String = turn[Contact]()

fn main() {
	println(v())
	println(W)
}
```
Today: `NK1152` "You can't call `W` while the program is built." and the same `NK2202`. Wanted: `age` twice.

## Decision (no owner decision needed)
A call `f[A](...)` is a call of `f` (Part I 4.7, ADR-288: the callee is named and resolved before its arguments). The effect walk names it by its base and takes the generic function's promise, as for `f(...)`. A bracket call of a function that can pause stays refused in a `sync` caller.

## Locations
* `crates/nikaia-std/src/tools/foreign.nika` `a_named_call` (~428): names the callee only for `Expr::Variable` and `Expr::Path`; `id[i64]` is `Expr::Index { base, index, .. }`, so it pushes `Seen::Opaque` (438). Suspected cause, fairly sure. The checker's own resolution of the form: `check_core_call.nika` ~824 (`check_core_call_filled`).
* Consumers (read, change only if the fix needs it): `crates/nikaia-std/src/tools/sync.nika` `violations_in` (427), `pausing_call` (445), and the `sync_call` loop (~271). Look with `where.nika -- a_named_call` for other callers of the same naming (a second walk that repeats the `Variable`/`Path` match for a call names its file in `Body:` and is fixed the same way).
* Lowered files (`crates/nikaia-std/src/tools/lowered/foreign.rs`, and `sync.rs` if touched) and the `lower-std` fixpoint.

## To build
1. The effect walk names a bracket call by its base. Test `tests/compiler/tests/a_sync_function_calls_a_generic_by_brackets.nika` + `.stdout` (both probes in one program); test `tests/compiler/tests/a_sync_function_calls_a_pausing_generic_by_brackets.nika` + `.stderr`: a generic function without `sync` that calls `sleep` (or another pausing `std` call; probe which one the checker counts) called as `g[i64](1)` from a `sync` function stays NK2202, now naming `g`. The commit says `Fixes #1095`, `Part: #1019`.

## Done when
The new tests pass through `nikaia test` (one program alone: `docs/builder-facts.md`, *Traps already met*) with `--both-settings`, the first failed once on the compiler without your change (say so under `Tests:`); every `sync_*` and `*sync*` test and every `comptime_*` test still pass; `lower-std` fixpoint clean; the cheap checks of Rules. Changelog: one line.

## Stop and report if
an edit outside `foreign.nika`, `sync.nika`, their lowered files and the new tests; an error in another agent's file. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j7-1095-sync-bracket-call.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
