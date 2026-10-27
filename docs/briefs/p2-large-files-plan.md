---
agent: builder
model: opus
max_turns: 70
budget_usd: 15
---
# Brief: a slice plan for `comptime_run.rs`, `sysroot.rs` and the rest of `emit/mod.rs` (research, docs only, Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, the largest Rust files first; every batch carries at least two moves). The brief before this one (`p1-large-files-plan.md`) plans `project.rs` and `parser/mod.rs`; this one plans the next three by size, so the batches after it have moves from four files to choose from without touching the same file twice in a row. S=2 (4 685 Rust lines without a plan), O~5 M (a planned slice costs about 10 k tokens a line, against 15+ unplanned, `c7-rest-plan`), C~2 M (reading and converter runs only).

## Facts
* Line counts at d8264d0: `crates/nikaia/src/comptime_run.rs` 1699, `crates/nikaia/src/sysroot.rs` 1575, `crates/nikaia/src/emit/mod.rs` 1411 (after E12-1..4b). Next by size: `modules.rs` 1398, `contracts/mod.rs` 1326, `check/mod.rs` 1039.
* `emit/mod.rs` keeps `Build`, `host_target()`, `default_target()` in Rust on purpose (E12-4 row: the host triple from `env!`, the orphan rule): name them under *Stays Rust*, do not plan them.
* `sysroot.rs` holds the lowered-file cache (#667: per-file keys, `LoweredBefore`, `in_parallel`) and Cargo calls: separate the pure parts (key computation, table walks) from the file system and process parts.
* The model of a slice table is `## C-7 rest and E12: what is left` in `docs/selfhosting-plan.md` (columns, *Stays Rust* list with one-line reasons, known risk per slice). `p1-large-files-plan.md` may already have appended `## Large files: project.rs and parser/mod.rs`: append after it, do not change it.
* Each move starts from an `rs2nika` draft (`docs/builder-facts.md`, *The converter*).

## To build (one commit, `docs/selfhosting-plan.md` only)
1. Read the three files; resolve callers with `target/fast/nikaia run scripts/where.nika -- NAME...`, not `grep -r`.
2. Run the converter (`rs2nika --profile check --file`) over each candidate slice once (a scratch `.rs` per slice under your scratch dir, not committed) and note the share of lines it drafts without a TODO.
3. Append `## Large files 2: comptime_run.rs, sysroot.rs, emit/mod.rs` to `docs/selfhosting-plan.md`: one row per slice of 300-1200 Rust lines, in build order: id (`CR1`.. for `comptime_run.rs`, `S1`.. for `sysroot.rs`, `E13-1`.. for `emit/mod.rs`), the Rust functions (file:line ranges), the target `tools/*.nika` file (new or existing), host callbacks added/removed, prerequisite (another slice, an open issue, or none), Rust lines, and the converter's draft share. Mark what must stay Rust with one line why each. Name the first two slices to brief, from different files, and the one known risk of each.
Nothing is built; `Tests: none, docs only`. Commit `docs: selfhosting-plan, slices of comptime_run.rs, sysroot.rs, emit/mod.rs (#125)`; no `Fixes`. `Rs2nika: none` (no move).

## Stop and report if
A file turns out to be mostly what must stay Rust (then say so in the table and plan `modules.rs` instead); an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p2-large-files-plan.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
