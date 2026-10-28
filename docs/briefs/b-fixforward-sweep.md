---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: fix-forward, arm64's *Accepted-programs sweep* red since c7e5b66

Score: a red `develop` is fixed forward first (`CLAUDE.md`); `main` stands still until then (last green 590cd7c, 14:36). Lane 1, before P5a.

## Facts
* Every CI run on `develop` from c7e5b66 (`rs2nika: --engine tree`) on fails in job `arm64`, step *Accepted-programs sweep (accepted-sweep)* (`.github/workflows/nikaia.yml` :375, `cargo test --release -p nikaia --test it accepted_sweep:: -- --ignored`, after the `Test` step ran the suite with `NIKAIA_DUMP_ACCEPTED`). Runs 6e9901e..7e3f8eb fail the same step whenever their `Test` step passed (5c14b5f..f42afdb had other reds, since fixed). The step before c7e5b66 (590cd7c) was green.
* c7e5b66 changed only `crates/rs2nika/convert/src/{main,tr_core,tr_expr,tr_items,tr_stmt,tr_types,tree}.nika` (3,345 lines added). So the likely cause: a test that checks converter sources (or the converter's corpus outputs) one file at a time now dumps an accepted one-file program that `rustc` refuses, or the dump changed a hash on the allow-list. Find which test runs `nikaia` over `crates/rs2nika/convert/` (`grep -rn "rs2nika" crates/nikaia/tests crates/rs2nika --include=*.rs`, `crates/rs2nika/convert/tests/`).
* The job log is not readable here (the proxy blocks GitHub's log blob); annotations are (`gh api repos/nikaia-foundation/nikaia/check-runs/<job>/annotations`), but the sweep step writes none. So this red was invisible.
* The sweep: `crates/nikaia/tests/it/accepted_sweep.rs` (the header says how it runs; `KNOWN` is its allow-list by source hash). It fails on a refusal not on the list, on a list entry that now compiles, and on an entry no test produces any more.

## To build
1. **Reproduce locally, narrowly** (never the full suite): run only the tests you found above with `NIKAIA_DUMP_ACCEPTED=<scratch dir>`, then the sweep over that directory (`NIKAIA_DUMP_ACCEPTED=<dir> cargo test --release -p nikaia --test it accepted_sweep:: -- --ignored`, or `--profile fast` if it builds the same). Name the failing program(s) and `rustc`'s first error in `Body:`. If the first guess dumps nothing that fails, widen to the `rs2nika` crate's tests, then the `it` tests whose names hold `rs2nika` or `convert`.
2. **Fix it at the source.** A lowering defect: fix in the compiler with a `tests/compiler` program (*Fix, do not work around*), as its own commit. If it is more than one step, stop: report it under `Found:` with the program and the error, and do not add a `KNOWN` entry yourself. Do not edit `crates/rs2nika/convert/src/*.nika` (the converter lane's; uncommitted `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}` are in the checkout, not yours): if the program is a converter file that should not be swept, the fix is in the test that dumps it, and you say so.
3. **Make the next sweep red readable**: in `.github/workflows/nikaia.yml`, both sweep steps (:166, :375) `tee` their output to `$RUNNER_TEMP/sweep.log` under `set -o pipefail`, and a following `if: failure()` step turns each failing program line and the panic message into `::error::` lines (the pattern of *Name the failing tests*, :160). CI only, no version. Commit separately: `ci: the accepted sweep names its failures as annotations`.

## Done when
The sweep passes locally over the dump of the tests you ran, the fix's own test passes, clippy clean, `scripts/pre-push-check.sh` passes. Commit the fix as `<what was wrong> (fix-forward, sweep red since c7e5b66)`; Changelog: one sentence if the compiler changed, else none.

## Stop and report if
The cause is in another lane's uncommitted file; the fix is more than one step; it is not reproducible from the tests named (then build step 3 alone, push it, and report `partial` with what you ran under `Tests:`).

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-sweep.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
