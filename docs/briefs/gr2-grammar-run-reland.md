---
agent: builder
model: opus
max_turns: 70
budget_usd: 14
---
# Brief: GR re-land, a grammar run's build and run of its parser move into Nikaia again (Part #125)

Score: self-hosting lane, row GR of `docs/selfhosting-plan.md`. GR was built (b070adb9, report `docs/briefs/gr-grammar-run.md.report.md`) and reverted (0fbe3d6b) because `lowered/project_members.rs` drifted (TrustedMap to HashMap). b15 (#1060, 1d88667d) showed the drift came from a stale binary, not from GR: with `grammar_run.nika` applied, `lower-std` on today's HEAD leaves `project_members.rs` as committed; and the gate now turns such drift red (`gate_lowered_drift`). So the move lands again as built. Small: S=1, C~1 M.

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

Shape: faithful (the reviewed code of b070adb9; no `Convert:` line: nothing new is converted)

## Pre-flight (stop if one fails)
1. `git log --oneline -1 0fbe3d6b` is the revert and `crates/nikaia-std/src/tools/grammar_run.nika` does not exist.
2. `cargo build --profile fast -p nikaia` first, so `lower-std` runs on a binary of HEAD (the b15 lesson: a stale `target/fast/nikaia` rewrites lowered files).

## To build
1. `git revert --no-commit 0fbe3d6b`. Checked on origin/develop c5873d3f: `grammar_run.nika`, `lowered/grammar_run.rs` and `grammar_run.rs` apply; `lowered.rs`, `docs/selfhosting-plan.md` and `scripts/self_hosting.py` conflict. Resolve them: keep today's side and add GR's lines (the plan row GR as built, its `self_hosting.py` entry); `lowered.rs` comes from `lower-std`, not by hand.
2. `lower-std`, then `git status --short crates/nikaia-std/src/tools/lowered/`: only `grammar_run.rs` and `lowered.rs` may differ from HEAD. Any other lowered file differing (above all `project_members.rs`) is `Found:` with the diff's first lines, and stops the brief (#1060).
3. `cargo check --profile fast -p nikaia-std --features compiler --message-format short`; a rustc error in `lowered/grammar_run.rs` is a lowering defect: stop, `Found:` with the lowered line.
4. Tests: the `it` filters `grammar_at_build_time dsl_grammar build_time comptime` (b070adb9 ran 115), and `nikaia test --project tests/compiler --both-settings` on the tests whose names contain `grammar`, `comptime`, `sysroot`, `lowered` (by name).
Commit `grammar run: the parser's build and run move into Nikaia (GR, again after 0fbe3d6b)`, `Part: #125`; `Body:` b070adb9's body, plus one sentence that b15 cleared the drift the revert named.

## Known and kept (from b070adb9, say so in `Body:`)
The unchanged-file check reuses `sysroot_holds` (`sysroot_cache.nika`), since `String?` == `String` is `NK1102`; `process::run` throws on output that is not UTF-8 where Rust read it lossily; `{error}` is an `io::IoError`'s text.

## Stop and report if
A conflict outside the three files above; any lowered file other than GR's drifts; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (today `lowered/project_members.rs` and the rs2nika ledgers among them), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/gr2-grammar-run-reland.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
