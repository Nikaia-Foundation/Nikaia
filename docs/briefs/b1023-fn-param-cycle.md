---
agent: builder
model: opus
max_turns: 100
budget_usd: 25
---
# Brief: #1023, three lowering defects of `fn` parameters handed on (blocks K3a part 2)

Score: bug before move; it blocks the self-hosting row K3a (k3a2 stopped on it, run 4). The whole issue, copied:


Found by k3a2 (K3a part 2, run 4; `docs/briefs/k3a2-comptime-compute.md.report.md`). Blocks K3a part 2 (`comptime_computed` / `comptime_worked_out_ahead`, draft `docs/briefs/k3a2-comptime-compute.run4.patch`).

## Problem and probes (run by the architect at 32080dd6)
1. **`Send + Sync` resolves to `tools`' own `enum Sync`.** A kept callback in a crate that overlaps lowers `Kept<dyn Fn(..) -> R + Send + Sync>` (`emit_core_types.nika:183` and `:191`); inside `tools`, `ty.nika:693`'s `pub enum Sync` shadows the trait: E0404 at `lowered/comptime_tree.rs:473`. The emitter writes `std::marker::Send + std::marker::Sync`.
2. **A let lambda that shadows the `fn` parameter it reads is handed on unlent.**
   ```
   fn alt(name: String) -> i64? sync { return name.len() }
   fn use_it(known: fn(ref String) -> i64? sync) -> i64 sync { return known("ab") ?? 0 }
   fn outer(known: fn(ref String) -> i64? sync) -> i64 sync {
       let known = fn(name) { known(name) ?? alt(name) }
       return use_it(known)
   }
   fn main() { println(f"{outer(fn(name) { null })}") }
   ```
   rustc E0308 "expected `&_`, found closure"; should print `2`. Without the shadowing (`let more = ...; use_it(more)`) it writes `&more`. The emitter takes the name for the parameter, not for the local that shadows it.
3. **A function in a call cycle that hands a new lambda to its own `fn` parameter instantiates itself without end.** Direct (`deep(n - 1, more)` from `deep`) and mutual:
   ```
   fn alt(name: String) -> i64? sync { return name.len() }
   fn ping(n: i64, known: fn(ref String) -> i64? sync) -> i64 sync {
       if n == 0 { return known("a") ?? 0 }
       let more = fn(name) { known(name) ?? alt(name) }
       return pong(n - 1, more)
   }
   fn pong(n: i64, known: fn(ref String) -> i64? sync) -> i64 sync { return ping(n, known) }
   fn main() { println(f"{ping(2, fn(_name) { null })}") }
   ```
   "Reached the recursion limit while instantiating `ping::<{closure}>`"; should print `1`. ADR-277 D11 writes a run `sync` parameter `impl Fn(A) -> R`; each new closure is a new type, so in a cycle the instantiation never ends. Handing the same parameter on (`deep(n - 1, known)`) already works (prints `3`) and must keep working.

## Decision (architect, a lowering defect: the programs are valid, D11 did not foresee the cycle)
ADR-277 D11 gains one sentence, committed with the build: *A run `sync` parameter of a function that lies on a cycle of the call graph is written `&dyn Fn(A) -> R`: `impl Fn` would make each new lambda a new instantiation, without end.* The cost is one indirect call per call of the parameter, only on cycles. A pausing parameter on a cycle is not decided here.

## Locations
* `crates/nikaia-std/src/tools/emit_core_types.nika:150-231` (the code-type shapes; `crossing` at :181-192).
* `crates/nikaia-std/src/tools/emit_function.nika:555-590` (a parameter's shape, `walk.code_parameter_runs`), `emit_item.nika:570` (the same for trait methods).
* `crates/nikaia-std/src/tools/emit_core_args.nika:145` (`emit_arg_calls_itself`), `:419` (`emit_arg_code_handed_on`): where an argument naming a code parameter is handed on vs lent with `&`.
* The cycle: `crates/nikaia-std/src/tools/emit_reach.nika` (`pausing_reach`, the call graph restricted to pausing keys, kept in `self.pausing_reach`; `emit_misc.nika:156` `closes_a_pausing_cycle`). Build the same reach over all functions with a code parameter (or all functions), keyed the same way, and ask it where the parameter's shape is written.

## To build
1. Defect 1: both `crossing` strings fully qualified. Check: after `lower-std`, `grep -n "+ Send + Sync" crates/nikaia-std/src/tools/lowered/*.rs` is empty. Commit.
2. Defect 2: an argument naming a local that shadows a code parameter is lent as a local. Test `tests/compiler/tests/a_shadowing_lambda_is_lent.nika` (+ `.stdout` `2`): probe 2. Commit.
3. Defect 3 and the ADR-277 D11 sentence. Tests `a_lambda_handed_round_a_cycle_runs.nika` (probe 3, `.stdout` `1`) and `a_recursive_function_hands_a_new_lambda_to_itself.nika` (the `deep` form: `fn deep(n: i64, known: fn(ref String) -> i64? sync) -> i64 sync { if n == 0 { return known("a") ?? 0 }; let more = fn(name) { known(name) ?? alt(name) }; return deep(n - 1, more) }`, main `deep(2, fn(_name) { null })`, `.stdout` `1`). `lower-std` fixpoint clean; `cargo check --profile fast -p nikaia-std --features compiler` clean (without the feature `tools` is not checked). Commit `lowering: a fn parameter handed round a call cycle is a &dyn Fn (Fixes #1023)`.

## Done when
The three tests pass through `nikaia test`, `tests/compiler/tests/a_trait_method_*` and `a_method_hands_its_callback_to_a_function.nika` still pass, the guard set of `.claude/agents/builder.md` is green; pushed. Changelog: one line (a `fn` parameter handed round a call cycle is borrowed as `&dyn Fn`; a shadowing lambda is lent).

## Stop and report if
An edit outside `crates/nikaia-std/src/tools/emit_*.nika` and their lowering, the checker fact files if the cycle needs one (`check_facts.nika`), ADR-277 and the new tests; a pausing parameter on a cycle (no probe here: report it under `Found:`, do not build it); an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (among them `crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`), and every `docs/briefs/*` file except your report. If `lowered.rs` shows another hunk, stage only yours.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1023-fn-param-cycle.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, nor a step another commit on develop already built: name the commit in `Body:`, nor a test's expected output taken from what the program really prints: say so in `Body:`); how you verified goes under `Tests:`. A `noop` says why under `Tests:`, not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
