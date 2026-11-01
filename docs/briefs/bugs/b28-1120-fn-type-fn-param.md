---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief: #1120, a function type with a function parameter lowers to nested `impl Trait` (rustc E0666)

Blocks: c2-rs2nika-thread-local.md step 2 (a scoped accessor such as `proofs::in_function` becomes a parameter of this shape; `fold_bound.rs` `proven_counters` waits)

Score: compiler bug, a program `nikaia check` accepts fails `nikaia run`; blocks a self-hosting move. Cause traced to the function, not run: small.

Facts: Where things live, Traps already met, Commands

## Issue #1120

```nika
fn twice(scoped: fn(i64, fn() -> i64 sync) -> i64 sync) -> i64 sync { return scoped(1, fn() { return 2 }) + 1 }
fn main() { println(f"{twice(fn(at, run) { return at + run() })}") }
```

Expected `4`. Got from rustc: ``Nested `impl Trait` is not allowed`` and ``` `impl Trait` is not allowed in the parameters of `Fn` trait bounds```.

## To build

1. Reproduce (`Tests:`).
2. Fix in `emit_core_ty_named`, `crates/nikaia-std/src/tools/emit_core_types.nika` (the "A parameter that is code" branch, about :150-270). The code's own parameter types (`ty.generics`, emitted into `params`, `called` and `elided`) and its result are emitted with `walk.in_parameter` still set, so an inner code type becomes `impl Fn` too. Emit them with `walk.in_parameter = false` (restored after), as the optional-code branch just above does, so an inner code type becomes `nikaia_std::func::Kept<dyn Fn() -> i64>`; the outer parameter keeps `impl Fn`. The same in the `for<'call>` branch. Owner, 2026-11-01: the simple `Kept` (an `Arc`) form now; a form without the allocation is a later issue, filed by the supervisor.
3. The argument side: the lambda handed to `scoped` must become `Kept(Arc::new(..))` (the existing wrapping, `emit_expr.nika` :512, `emit_core_flow.nika` :378). If it is not, fix it where an argument meets a `Kept` parameter (`emit_core_args.nika`).
4. `lower-std` and its fixpoint.
5. Test `tests/compiler/tests/a_function_type_takes_a_function.nika` + `.stdout`: the probe; an inner function that captures a local; an inner function with a parameter and a `throws` result if the language allows it there. Fails on the unfixed binary.

Commit `emit: a function type's function parameter is kept code, not a nested impl Fn (Fixes #1120)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`; the tests named `fn_|code|lambda|closure|kept` pass; `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside the emitter files (`emit_*.nika`), their lowered twins, `lowered.rs`, `check_*.nika` where the checker's type of the inner lambda is wrong, and the new test. Or you hit an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b28-1120-fn-type-fn-param.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
