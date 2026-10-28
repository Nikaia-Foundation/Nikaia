---
agent: builder
model: opus
max_turns: 70
budget_usd: 15
---
# Brief: a slice plan for lane 1's Thursday to Saturday files (research, docs only, Part #125)

Score: bootstrap week lane 1 (`docs/bootstrap-week.md` section 2: Thu 10-29 P5 `project.rs` build drivers; Fri 10-30 M3 `modules.rs` collect, `manifest.rs`, S1; Sat 10-31 K3 `comptime_run.rs` rest, `build_time.rs`, `grammar_run.rs`). P5, M3, K3 and the three small files have no plan row yet, so no brief can carry `Cut:` lines for them (lever L6: two cut briefs ready per lane). S=2 (about 3,500 Rust lines unplanned), C~2 M (reading and converter runs only).

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read `crates/nikaia-std/src/tools` as `crates/nikaia-compiler/src`.

## Facts
* Line counts at a29d3b8: `project.rs` 3,565, `comptime_run.rs` 1,699 (K1+K2 run just before this brief, 35-1093 in part), `modules.rs` 1,088, `manifest.rs` 599, `grammar_run.rs` 522, `build_time.rs` 387. Re-count at your start.
* What the earlier plans called *Stays Rust* in `project.rs` (`lower`, `lower_reading`, `project_for_file`, `Project::open`/`drive`/`drive_to`, `check`'s calls, `explain`) and in `modules.rs` (`collect*`, `package_at`, `read_unit`, `check_imports`, `emit_reading`) and `comptime_run.rs` (286-745, `Bundle`..) is **this week's work**: the deadline's done is "the compiler's Rust is the adapters, <= 2,950 lines" (`docs/bootstrap-week.md` section 1). So the question is no longer *whether* a driver moves but *how*: which part is Nikaia over host callbacks (file system through `std::fs`, `Parsed` through items + `InternerContext` + `intern`, as M2 and K1 do), and which part is the adapter left (Cargo and `rustc` processes until #920, `toml::Value`, `thread_local`s, `OnceLock`, `env!`). Name the adapter lines each row leaves.
* The model of a slice table is `## Large files: project.rs and parser/mod.rs` in `docs/selfhosting-plan.md` (columns Id, Rust file:lines, Target, Host, Prerequisite, Rust lines, Draft). Append `## Large files 3: lane 1's drivers` after `## Large files 2`; do not change the earlier tables except to mark nothing.
* Every row: 400-600 Rust lines (bootstrap-week L3), its ready `Cut: path:A-B` lines (ranges at your commit) and its `Convert:` line with `--prefix X_` of the target file (`docs/architect.md`, *Briefs and reports*); the converter's draft share over exactly those cuts (`crates/rs2nika/convert/target/nikaia/bin/rs2nika --profile check-free --prefix X_ --file <scratch>.rs`; the share of code lines in items without a TODO); every Rust `std` call it moves checked against Part III 17 for a Nikaia `std` counterpart (a missing one is named as the row's prerequisite: a `std` brief, or #920/#922); every struct the row reshapes with the files that read its fields and a count (P1 lost six `cargo check` rounds on 42 field reads); the callers of what moves, including `crates/nikaia/tests/it/` (`target/fast/nikaia run scripts/where.nika -- NAME...`, not `grep -r`).
* Rows in build order: P5 (`project.rs`), M3 (`modules.rs`), MF (`manifest.rs`), K3 (`comptime_run.rs` 286-745), BT (`build_time.rs`), GR (`grammar_run.rs`); split a file into several rows where it is over 600 lines. Files of lane 2 (`contracts/*`, `check/*`, `emit/*`, ...; *File ownership* of `bootstrap-week.md`) are not edited by these rows: a caller there keeps a one-line Rust forward.
* Do not touch `crates/rs2nika/` (the converter lane's uncommitted `convert/src/main.nika` is in the checkout).

## To build (one commit, `docs/selfhosting-plan.md` only)
1. Read the six files; resolve callers with `where.nika`.
2. Run the converter once per candidate row (scratch `.rs` under your scratch dir, not committed); note the draft share.
3. Append the table with the columns above plus a `Cut/Convert` column (or a list under the table, one block per row, ready to paste into a brief), a *Stays Rust* list of the adapter lines with one line why each, and the order: the first two rows to brief (from different files) and the one known risk of each.
Nothing is built; `Tests: none, docs only`. Commit `docs: selfhosting-plan, lane 1's drivers (P5, M3, MF, K3, BT, GR) (#125)`; no `Fixes`. `Rs2nika: none` (no move).

## Stop and report if
A row cannot be cut under 600 lines without splitting a function (say so in the table instead of stopping); an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p5-lane1-plan.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
