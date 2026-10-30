---
agent: builder
model: opus
max_turns: 100
budget_usd: 25
---
# Brief: #1012 step 3, the trait column covers data parameters; the Rust host impls take what the column says (ADR-277 D17)

Score: self-hosting lane; the last step of the *To build* list of #1012 that changes the compiler. Precondition: `b1012a-trait-column-fn.md` and `b1012b-trait-column-refusals.md` are on `origin/develop`; if one is not, stop with `Result: blocked`, `Needs:` the missing one.

## Problem

D17 covers every by-value parameter, code or data (ADR-094 D2 does not tell them apart): by value no longer means kept. Step 1 kept today's by-value seed for data parameters of a trait method, because the lowered host traits in `crates/nikaia-std/src/tools/*.nika` are implemented only in Rust, and the column moving from kept to run moves their signatures (`String` to `&str`, an owned value to a borrow). Known by-value, non-`Copy` data parameters of host traits (re-grep): `hole(text: String)`, `crate_root(name: String)`, `parse_unit(package: String?)`, `emit_unit(provenance: Provenance)`, `absorb(into: Ledger, other: Ledger)`, two `Symbol` parameters.

## Locations
* `crates/nikaia/src/contracts/keeps.rs`: drop the data seed of the trait node; data parameters get the same edge pairs as `fn` parameters (step 1).
* The hand-written Rust impls of lowered tools traits: `check/host.rs`, `emit/host.rs`, `project.rs`, `modules.rs`, `comptime_run.rs`, `build_time.rs`, `fold_bound.rs`, `emit/seeds.rs`, `check/core.rs` (grep `impl .* for` against the trait names of `tools/*.nika`); each takes the signature its lowered trait now has, and its callers in Rust follow. A Rust impl that really keeps a parameter the column now runs (stores the `String`): stop, `Found:` with the trait method and the impl. The column is not widened from Rust (D17: impls in Rust are not evidence).

## To build
1. The seed removed; the lowered host traits and their Rust impls follow; `lower-std` fixpoint clean. Test in `tests/compiler/tests/` (+ `.stdout`): a trait method taking `text: String` that its only impl reads, called through a bound by a caller that uses its string after the call and with a borrowed one, prints both; a sibling row where a local impl stores the string keeps the column (the caller's string moves). Commit `contracts: a trait method's data parameter is a view unless its package's impls keep it (#1012)`; `Fixes #1012` if `k3a2-comptime-compute.md` (step 4) is on `origin/develop`, else `Part of #1012` and `Needs:` says step 4 is the last open one.

## Done when
The test passes through `nikaia test`, the #1012 tests of steps 1-2 still pass, `cargo build --profile fast -p nikaia`, the `it` filter `contracts` and the guard set of `.claude/agents/builder.md` are green; pushed. Changelog: one line (a trait method's by-value data parameter is borrowed when no impl in its package keeps it).

## Stop and report if
An edit outside `crates/nikaia/src/contracts/keeps.rs`, the Rust host impls named above and their callers, `crates/nikaia-std/src/tools/*.nika` and their lowering, and the new test; a Rust impl keeps a parameter the column runs; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1012c-trait-column-data.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
