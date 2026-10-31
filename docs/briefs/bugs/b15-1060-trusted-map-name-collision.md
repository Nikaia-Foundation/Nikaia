---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1060, a tools file's untrusted host method demotes another file's `fs::Path` maps from `TrustedMap` to `HashMap` (and the gate that missed it)

Blocks: grammar_run (the GR move, b070adb9, reverted in 0fbe3d6b; it re-enters the main batch once this is on develop)

Score: compiler bug, a silent regression of the lowering (ADR-010 D5/D9: the maps lose the fast hasher) that keeps a self-hosting move out, and a gate that let a red CI through as green. Every new tools file with a host trait may set it off again (K3b, P5c carry a guard for it).

Facts: Where things live, Traps already met, Commands

## Issue #1060 (summary; the issue holds the full text)
Adding `crates/nikaia-std/src/tools/grammar_run.nika` (b070adb9) makes `lower-std` write `lowered/project_members.rs` with `collections::HashMap` where it wrote `collections::TrustedMap` (`seen`, `here`, `renames_in`'s parameter, `at_of`; `project_members.nika` :118, :124, :184, :238, :282), although `project_members.nika` did not change. Not a cache effect (fresh `NIKAIA_LOWER_CACHE_SALT`). The suite was green because `gate.nika --all` runs `lower-std` first and rewrites the drifted file; CI was red.

Probe: `git show b070adb9 -- crates/nikaia-std/src/tools/grammar_run.nika crates/nikaia-std/src/tools/lowered.rs | git apply` (or just add `grammar_run.nika`), then `NIKAIA_LOWER_CACHE_SALT=f$RANDOM NIKAIA_SYSROOT=$PWD/crates target/fast/nikaia lower-std` and `git diff --stat crates/nikaia-std/src/tools/lowered/project_members.rs`. Restore only the files you applied, by name.

## Suspected location (architect's trace, not run)
`crates/nikaia-std/src/tools/trust.nika` `trusted_maps` :282-290 puts the **last `::` segment** of every library function with `Provenance::Untrusted` into `env.sources`. `GrammarRunHost::manifest` (a body-less host method, so untrusted) puts the bare word `manifest` there; in `project_members_packages_of` the parameter `manifest` (`manifest.paths.iter()`) then reads as an untrusted source and poisons the class of `seen`/`here`. Check where `env.sources` is matched (`tm_deps_of`, `tm_call_deps`): a source must be a **call** of that function (its full path, or a method call on a receiver of that type), never a variable or parameter that happens to share the name. If the trace is wrong, find the rule that demotes the maps; if the demotion is right by D9, stop and say why (`Needs: architect decision`).

## To build
1. Reproduce with the probe on the unfixed binary; keep the result under `Tests:`.
2. The fix in `trust.nika` (and its lowered twin). Test `tests/compiler/tests/trusted_map_name_not_a_source.nika` + `.stdout` (or a package test if one module cannot show it): a body-less `trait` method `fn manifest(ref self) -> String` with untrusted provenance and, in another function, a parameter or `let` named `manifest` feeding a `collections::HashMap[fs::Path, String]` whose keys are trusted; the lowering must keep `TrustedMap` (assert it the way the existing `trusted_map*` / `hasher*` tests in `tests/compiler/` do: `grep -rln TrustedMap tests/compiler/tests | head`). Commit `trust: an untrusted function's name is no source where a variable carries it (Part #1060)`.
3. The gate (#1060 step 5): in `scripts/gate.nika` (`gate_lower_std` :108, the fixpoint :121-127) check first that the committed lowered files are what this compiler lowers: after the first `lower-std`, `git diff --exit-code --stat crates/nikaia-std/src/tools/lowered crates/nikaia-std/src/lowered*` against the tested commit, and fail the step (`red: lowered drift: <files>`) on a difference **when the commit under test did not change that `.nika`** (`git diff --name-only HEAD^ HEAD`); a builder's own uncommitted re-lowering in `--cheap` mode stays allowed. Update the gate's dry-run test if one covers the step. Commit `gate: a lowered file that drifts from what this compiler lowers is red (Part #1060)`.

`Part: #1060`, not `Fixes`: step 4 (the GR re-land) goes into the main lane after this.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings` (by name), with the tests whose names contain `trusted`, `hasher` or `map` (run by name); with `grammar_run.nika` applied, `lower-std` leaves `project_members.rs` as committed (say so under `Tests:`, then remove the applied files by name); `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `trust.nika`, its lowered twin, `scripts/gate.nika`, the gate's test and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (`lowered/project_members.rs` is uncommitted residue at the time of writing: if your fix makes `lower-std` write it back to `HEAD`, say so; do not commit it), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b15-1060-trusted-map-name-collision.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
