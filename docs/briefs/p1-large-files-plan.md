---
agent: builder
model: opus
max_turns: 70
budget_usd: 15
---
# Brief: a slice plan for `project.rs` and `parser/mod.rs`, the two largest Rust files (research, docs only, Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: every batch carries at least two moves, the largest Rust files first). After E12-4b no planned move is left: the E12 table in `docs/selfhosting-plan.md` is the last one. S=2 (the next ~10 moves cannot be briefed without it; 9 202 Rust lines in the two files), O~5 M (a planned slice cost about 10 k tokens a line against 15+ unplanned, `c7-rest-plan`), C~2 M (reading and converter runs only).

## Facts
* Line counts at d6318de: `crates/nikaia/src/project.rs` 4851, `crates/nikaia/src/parser/mod.rs` 4351. Next by size: `comptime_run.rs` 1699, `sysroot.rs` 1575, `emit/mod.rs` 1545, `modules.rs` 1398, `contracts/mod.rs` 1326, `check/mod.rs` 1039.
* An older, partly stale plan names R-4 "the non-grammar leaves of `parser/mod.rs` (Trivia, `number_at`, the error helpers, ~600 lines, prefix `Pr`)" (`docs/selfhosting-plan.md` around line 473, from `docs/briefs/r1-next-rust-slices.md`): check what of it is still in Rust.
* The model of a slice table is `## C-7 rest and E12: what is left` in `docs/selfhosting-plan.md` (columns, *Stays Rust* list with one-line reasons, known risk per slice).
* Each move starts from an `rs2nika` draft (`docs/builder-facts.md`, *The converter*).

## To build (one commit, `docs/selfhosting-plan.md` only)
1. Read both files; resolve callers with `target/fast/nikaia run scripts/where.nika -- NAME...`, not `grep -r`.
2. Run the converter (`rs2nika --profile check --file`) over each candidate slice once (a scratch `.rs` per slice under your scratch dir, not committed) and note the share of lines it drafts without a TODO.
3. Append `## Large files: project.rs and parser/mod.rs` to `docs/selfhosting-plan.md`: one row per slice of 300-1200 Rust lines, in build order: id (`P1`.. for `project.rs`, `R4`.. for the parser), the Rust functions (file:line ranges), the target `tools/*.nika` file (new or existing), host callbacks added/removed, prerequisite (another slice, an open issue, or none), Rust lines, and the converter's draft share. Mark what must stay Rust (process entry, file system, Cargo invocation, `Parsed` ownership...) with one line why each. Name the first two slices to brief, from different files, that can run one after the other without touching the same file, and the one known risk of each.
Nothing is built; `Tests: none, docs only`. Commit `docs: selfhosting-plan, slices of project.rs and parser/mod.rs (#125)`; no `Fixes`. `Rs2nika: none` (no move).

## Stop and report if
A file turns out to be mostly what must stay Rust (then say so in the table and plan the next file by size instead, `comptime_run.rs`); an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p1-large-files-plan.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
