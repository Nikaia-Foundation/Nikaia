---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief p5: #1069, a `.bounds` expectation beside a test program, so a proof is guarded by a test

Facts: Where things live, Traps already met, Commands

Score: S=3 (every prover gain of the lane, p2-p4 and the counters next, is guarded today by no test: a later change can lose a proof silently, and claim (b)'s figure moves only when someone measures), O=3 M (every later brief of the lane tests with it; CLAUDE.md wants tests in Nikaia, and `tests/compiler` cannot show `--bounds`), C=6 M (one optional file pair in `project_tests.nika`, the comparison in the program-test runner, bless). Area: prover, toolchain.

## Problem (probed 2026-10-31 on ecf435a's binary)

`tests/compiler` is one project: a test cannot set `--optimization` and cannot show the `--bounds` report, so p2-p4 verified their gains with probes outside the suite (`docs/briefs/prover/state.md`, *Lane facts*). `nikaia.toml` has no optimization key. `check --bounds FILE` prints a first line `bounds in FILE: N indexes, P proved; M operations, Q proved - written without their check: indexes off, operations off`, then one line per site (`  LINE  TEXT  proved|checked: REASON`).

## Decision (architect, 2026-10-31, under *Language decisions*; issue #1069)

A program test `tests/NAME.nika` (ADR-269 D22) may have, beside its `.stdout`/`.stderr`:

* `NAME.bounds`: the `--bounds` report of `NAME.nika` at `prove-through-loops:off`;
* `NAME.loops.bounds`: the same at `prove-through-loops:on`.

`nikaia test` compares each one that exists, byte for byte, after two normalisations: the path in the first line is shown relative to the project (`tests/NAME.nika`), and the build-dependent tail (` - written without their check: ...`) is cut, so the file depends neither on `--both-settings` nor on the machine. A difference fails that test and prints the difference as a `.stdout` difference does; `--bless` writes them as it writes `.stdout`. A `.bounds` or `.loops.bounds` with no `NAME.nika` is an error, as a stray `.stderr` is. The report is computed as `check --bounds` computes it for the program (the file checked as `nikaia test` checks it), independent of the build's `remove-*` settings (ADR-306 D14: what is proved does not depend on them).

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/project_tests.nika`: `ProgramTest` (50-57), `program_tests` (172-181, reads `.stdout`/`.stderr` with `project_tests_optional`), `project_tests_bless_program` (355-380), `project_tests_difference` (206), `project_tests_explain_output` (384). Re-lower it (`lower-std`) and commit `lowered/project_tests.rs`.
* `crates/nikaia/src/project.rs`: `run_tests` (about 2931) and the loop over `suite.programs_of_their_own` (about 3057) - the Rust side that builds and runs each program test. Edit only there: lines 2112-2586 (`drive`, `drive_to`) are cut by the main lane's P5c move; do not touch them.
* `crates/nikaia/src/bounds_report.rs`: `report` (47, checks the file alone), `report_checked` (75); `through_loops: bool` is the setting.
* Part III, the `nikaia test` section of `docs/specification/30-nikaia-tooling.md` (search `.stderr` near *program test* / ADR-269 D22): one paragraph with the decision.
* Test pattern for a temp project that runs `nikaia test`: `tests/compiler/tests/a_library_trait_with_no_impl_runs.nika`.

## Steps

1. **Read and compare.** `ProgramTest` gets `bounds: Bytes?` and `loops_bounds: Bytes?`; `program_tests` reads them; the runner computes both reports where a file exists and fails the test on a difference, with the normalisations above; bless writes them; the stray-file error. Part III paragraph. Commit.
2. **The test of the form**: `tests/compiler/tests/bounds_expectation_guards_a_proof.nika` + `.stdout`. In a temp project: a test program with a `while` counter (`fn k() -> i64 sync { let mut i = 0; let mut s = 0; while i < 10 { s += 1; i += 1 }; return s }`, written over lines) and `.bounds`/`.loops.bounds` that agree passes (`1 passed`); with `proved` changed to `checked` in one line of `.loops.bounds` it fails and the output names `.loops.bounds`; a stray `x.bounds` is the error. Probe first with `check --bounds` (with and without `--optimization prove-through-loops:on`) what the report really says, and write the files from that. Commit.
3. **Two guards for proofs the lane has**: `tests/compiler/tests/prove_loop_counter_bounds.nika` (p4's probe functions `k`, `g`, `f` and `c` from `docs/briefs/prover/p4-1054-loop-checks.md`, *Problem*) and `tests/compiler/tests/prove_in_loop_assert_bounds.nika` (an index inside a loop under a p2 in-loop `assert`, e.g. `while i < xs.len() { assert(i >= 0); s += xs[i]; i += 1 }`), each with `.stdout`, `.bounds` and `.loops.bounds` written with `--bless` and read through: each `proved` there must be one the lane claims (p4's report: `k`, `g`, `f` proved under `on`, `c`'s `n += 1` checked). Run both by name with `nikaia test --project tests/compiler --both-settings`. Commit with `Fixes: #1069`.

Changelog: this changes what `nikaia test` does: write the `Changelog:` line (one sentence), not `none`.

## Done when

The three tests pass by name through `nikaia test --project tests/compiler --both-settings`, with the program tests whose names start `prove_` and the tests that contain `bless` or `stderr` (by name); `lower-std` fixpoint clean; build, fmt, clippy (1.97 and 1.99.1 where installed), `pre-push-check.sh`; committed locally. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if

An edit is needed outside `project_tests.nika` and its lowered twin, the program-test part of `project.rs`, `bounds_report.rs`, the Part III paragraph and the new tests; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (`crates/nikaia-std/src/tools/lowered/project_members.rs` is uncommitted residue at the time of writing; do not commit it), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p5-1069-bounds-expectation.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
