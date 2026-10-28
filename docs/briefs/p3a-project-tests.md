---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P3a, the test runner's discovery and verdicts of `project.rs` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, largest Rust files first; `project.rs` is the largest, 4,533 lines). Plan row P3 (`docs/selfhosting-plan.md`, *Large files*) is ~720 lines, too large for one run (M1's first run was cut by the driver's 30-minute limit), so it is split: P3a now (discovery, comparison, verdict texts, ~330 lines), P3b later (the run loop: `run_tests`, `outcome_of`, `run_program_test`, `bless`, `bless_program`, `Blessed`, `explain_output`). S=2, C~4-6 M (comparable: S2, 9826a9e, 221 lines in one run). Prerequisite of P3 (the `std::fs` bytes probe) is built: 070b9c7.

## Facts
* Locations at 67dfd5a, `crates/nikaia/src/project.rs`: `struct Outcome` :3404, `pub struct OutputTest` :3420, `files_under` :3438, `output_tests` :3465-3535, `copy_into` :3562, `pub struct ProgramTest` :3781, `program_tests` :3791-3825, `is_refusal` :3829, `does_not_compile` :3835, `named_by` :3951, `difference` :3991, `verdict` :4057, `shown_path` :4070-4090. Re-grep before cutting.
* Moved here: `files_under`, `output_tests`, `copy_into`, `program_tests`, `is_refusal`, `does_not_compile`, `named_by`, `difference`, `verdict`, `shown_path`, into new `crates/nikaia-std/src/tools/project_tests.nika` (prefix `project_tests_` where `where.nika` finds a clash). `std::fs` reads, writes and `read_dir` as `sources.nika` and `sysroot_cache.nika` do (async in std: the Rust caller goes through `rt::exec::block_on`, as `sysroot.rs` does since S2).
* `OutputTest`, `ProgramTest` become Nikaia structs (paths as `fs::Path` or `String`, as `sysroot_cache.nika` chose) re-exported in `project.rs`, if the Rust callers (`Suite`, `run_tests`, `crates/nikaia/src/main.rs`) stay readable with it; otherwise they stay Rust structs and the Nikaia functions return their parts. That choice is yours: name it in `Body:`. `Outcome` stays Rust in P3a unless `verdict` needs it as Nikaia (then it moves with it). `Sandbox`, `Suite`, process spawns, `current_exe` stay Rust.
* The converter's `Rs2nika:` costs are read by the converter lane: do not change `crates/rs2nika/`.

## To build
1. Run the converter on a scratch file holding exactly the moved Rust (`rs2nika --profile check --file`), report `--score` under `Rs2nika:`. Move the listed functions into `project_tests.nika` with their doc comments (they carry the ADR references); `project.rs` calls the lowered functions; the moved Rust is deleted. `lower-std`: only the new lowered file and its index lines change; add the file to `scripts/self_hosting.py` as S2 did.
2. In `docs/selfhosting-plan.md` split row P3 into P3a (built, `project_tests.nika`) and P3b (the run loop, as named above, prerequisite P3a); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`); if it is more than one step, stop and report it under `Needs:`. Known open checker defect, not yours: #909 (a lent loop element moved into a tuple literal passes the checker); write `.clone()` where rustc asks for it and name it in `Body:`.

## Done when
`nikaia test tests/compiler --both-settings` (it runs the runner you moved: every test passes, the count as before); `it` tests filtered on `output_test program_test bless test_runner refusal` plus the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. Commit `project: the test runner's discovery and verdicts into Nikaia (P3a)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the one left above; a `.stderr` or a runner message that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p3a-project-tests.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
