---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P3b, the test runner's run loop of `project.rs` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, largest Rust files first; `project.rs` is still the largest, 4,310 lines). Plan row P3b (`docs/selfhosting-plan.md`, *Large files*). S=2 (~390 lines), C~4-6 M (comparable: P3a, b8547a6). Prerequisites met: P3a (b8547a6), `shown_path` and `fs::canonical`/`Path.strip_prefix` (594cb42, f861426).

## Facts
* Plan row P3b (copied): `project.rs`: the run loop: `run_tests`, `outcome_of`, `run_program_test`, `bless`, `bless_program`, `Blessed`, `explain_output` into `project_tests.nika`; added: `run_test_binary(binary, args, stdin, cwd) -> Outcome`, `run_program_test(test, setting) -> Outcome` (process spawns, `current_exe`, the `Sandbox` directory stay Rust); `Suite` holds the binaries as names.
* Locations at 33f9df4, `crates/nikaia/src/project.rs`: `use ... ProjectTestsOutcome as Outcome` :31, `struct Sandbox` :3413 (stays), `outcome_of` :3442 (the spawn: stays Rust), `pub struct Suite` :3472, `run_tests` :3500-3658, `run_program_test` :3659-3733, `bless_program` :3734, `explain_output` :3767, `enum Blessed` :3802, `bless` :3813. Caller: `crates/nikaia/src/main.rs` :872 (`project::run_tests(&project::Suite {..})`). Nikaia side: `crates/nikaia-std/src/tools/project_tests.nika` (`ProjectTestsOutcome` :16, `OutputTest` :31, `ProgramTest` :49, `project_tests_verdict` :221). Re-grep before cutting.
* `std` counterparts checked (Part III 17): `fs::write`, `fs::read`, `Path.join`/`display`/`strip_prefix` exist. `process::run` hands back stdout as text and takes no stdin bytes and no env, so it does not replace `outcome_of`: every process spawn (`outcome_of`, the `current_exe` runs of `run_program_test`) and `Sandbox` stay Rust and reach Nikaia as callbacks `fn(..) -> ProjectTestsOutcome sync` (or `-> ... throws`), as M2's `intern` callback does (`modules.nika`, `check_walked_kept_call` in `check_walked.nika`).
* The cut is yours: either `run_tests` moves whole with the spawns as callbacks, or the loop stays Rust and Nikaia does each test's work (the outcome, the bless decision and write, the explanation text, the printed lines). Name it in `Body:`. Every printed line stays byte for byte (the `.stdout` of `tests/compiler` and the `it` tests hold them).
* The converter's `Rs2nika:` costs are read by the converter lane: do not change `crates/rs2nika/`.

## To build
1. Run the converter on a scratch file holding exactly the moved Rust (`rs2nika --profile check --file`), report `--score` under `Rs2nika:`. Move the functions into `project_tests.nika` with their doc comments (they carry the ADR references); `project.rs` calls the lowered functions; the moved Rust is deleted. `lower-std`: only `project_tests`' lowered file and its index lines change.
2. Mark P3b built in the plan's table (one line, as P3a's row); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`); if it is more than one step, stop and report it under `Needs:`. Known open checker defect, not yours: #909 (a lent loop element moved into a tuple literal passes the checker); write `.clone()` where rustc asks for it and name it in `Body:`.

## Done when
`nikaia test tests/compiler --both-settings` (it runs the loop you moved: every test passes, the count as before, 206); `it` tests filtered on `output_test program_test bless test_runner refusal` plus the guard set of `.claude/agents/builder.md`; one `nikaia test --bless` probe on a scratch copy of a package with a changed `.stdout` (the file is rewritten, the line says `blessed`); clippy clean; lower-std fixpoint. Commit `project: the test runner's run loop into Nikaia (P3b)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the cut left above; a printed line, `.stderr` or runner message that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p3b-project-run-loop.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
