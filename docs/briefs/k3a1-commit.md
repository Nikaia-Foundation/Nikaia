---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: verify and commit K3a part 1, left uncommitted in the shared checkout (Part #125)

Score: self-hosting lane, bootstrap week lane 1. K3a part 2 (`k3a2-comptime-compute.md`, next in the batch) builds on this host; nothing else moves until it is on `develop`. S=1, cost small.

## Facts
* The k3a1 builder (2026-10-29) wrote `docs/briefs/k3a1-comptime-lower.md.report.md` with `Result: done` but **never committed or pushed**: no `Pushed:` line, placeholders `TC` and `DISK`. Its work is in the working tree of the main checkout, uncommitted:
  `crates/nikaia-std/src/tools/comptime_tree.nika` (+115: trait `ComptimeLowerHost`, `comptime_lowered_units`, `comptime_without_foreign`, `comptime_dependency_modules`, `comptime_by_package`),
  `crates/nikaia-std/src/tools/lowered/comptime_tree.rs` (its lowering),
  `crates/nikaia-std/src/tools/lowered.rs` (one hunk: the `pub mod comptime_tree` index line gains `ComptimeLowerHost, comptime_lowered_units, comptime_without_foreign, comptime_dependency_modules, comptime_by_package`),
  `crates/nikaia/src/comptime_run.rs` (`LowerHost` and its `tree::ComptimeLowerHost` impl, :120-263; the four Rust functions are adapters with unchanged signatures),
  `crates/rs2nika/convert/rs2nika.names` (+11: `[forms comptime_]`, `[hosts comptime_]`).
* `docs/selfhosting-plan.md` row K3a already says part 1 is built (committed earlier); do not touch it.
* `develop` moved since (0.0.1307, lowering changes #933, #942, #943, #949 among others), so the lowered file must be made again from the current compiler before it is committed. 9538c0b0 once dropped these index names from `lowered.rs` because the lowered file was missing; this commit puts both in together.
* **Not yours, never stage**: `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}` and every `docs/briefs/*.report.md` that `git status` shows modified (another session's). If `git diff crates/nikaia-std/src/tools/lowered.rs` shows a hunk besides the `comptime_tree` one, stage only that hunk (`git diff ... > p; edit p; git apply --cached p`, as p5e did).

## To build
1. `git diff --stat` on the five files: confirm they hold the work above (no other session's edit mixed in; `comptime_run.rs` last changed on develop in 7e3f8ebd, so the whole diff is k3a1's).
2. `cargo build --profile fast -p nikaia`; `lower-std` (`docs/builder-facts.md`); the fixpoint may change only `lowered/comptime_tree.rs` and its index line. A checker or lowering error in `comptime_tree.nika` from the moved compiler: fix it in `comptime_tree.nika` if it is a change of a few lines that keeps texts byte for byte (say so in `Body:`); a compiler defect: stop, `Result: blocked`, `Found:` with a probe.
3. `cargo fmt`, `cargo clippy --all-targets -- -D warnings` (a lint only in another session's uncommitted file goes under `Needs:`, as p5e reported), `it` filters `comptime grammar_at_build_time` plus the guard set of `.claude/agents/builder.md`, once, beside clippy. `scripts/pre-push-check.sh`.
4. Commit the five files (`git add` by name) with subject `comptime: a build-time program of several files assembled in Nikaia (K3a part 1)` and as body the `Body:` paragraph of `docs/briefs/k3a1-comptime-lower.md.report.md` (drop its sentence on `lowered.rs`'s text_tiers line if no such hunk exists now), `Part: #125`. Push.

## Done when
The commit is on `origin/develop`; clippy and the tests above pass. Changelog: none (a move). `Rs2nika: none` (this task converts nothing; k3a1's figure is in its own report).

## Stop and report if
A text the lowering writes would change; an edit needed outside the five files; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/k3a1-commit.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
