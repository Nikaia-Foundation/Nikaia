---
agent: builder
model: opus
max_turns: 70
budget_usd: 15
---
# Brief: C-7 rest, a slice plan for `check/mod.rs` and the rest of `emit/` (research, docs only)

Score: self-hosting lane (`docs/priorities.md`): S=2 (the next slices of #125 cannot be briefed without it; 4 400 Rust lines are left in `check/` and `emit/`), O~5 M (a planned slice costs about 10 k tokens per line against 15+ unplanned), C~2 M (reading only), area self-hosting. #867 (seeds) finished in 0e24e46.

## Facts
Left in Rust (wc, 252bcf2): `check/mod.rs` 1309, `check/host.rs` 482, `check/core.rs` 321, `check/attributes.rs` 87, `emit/mod.rs` 2350, `emit/seeds.rs` 507, `emit/host.rs` 472, `emit/rust_targets.rs` 327. What the earlier moves left and why is at the end of `docs/selfhosting-plan.md` (E11-4c + E11-5: `template_hole_tables`, `fold_callees`, `read_view_slots`, `tethered_types` read Rust-side `Parsed` caches and the ledger; seeds: `root_name`, `shapes`). Hosts (`check/host.rs`, `emit/host.rs`) shrink as their callers move.

## To build (one commit, `docs/selfhosting-plan.md` only)
1. Read `check/mod.rs` and `emit/mod.rs` (and skim `seeds.rs`, `rust_targets.rs`); resolve callers with `target/fast/nikaia run scripts/where.nika -- NAME...`, not `grep -r`.
2. Append a section `## C-7 rest and E12: what is left` to `docs/selfhosting-plan.md`: a table with one row per slice of 300-1200 Rust lines: id (`C7-r1`..., `E12-1`...), the Rust functions (file:line ranges), the target `tools/*.nika` file (new or existing), what it needs from the host (callbacks added / removed), the prerequisite (another slice, an open issue such as #791 or #804, or none), and a cost estimate in Rust lines. Order the rows so each can be built by one builder in one run. Mark what must stay Rust (the driver, `Parsed`/ledger ownership, process entry) and say why in one line each.
3. A line naming the first slice to brief and the one known risk of each slice (a syntax gap of `tools/*.nika` or a lowering defect likely to be met, from `docs/builder-facts.md`).
Nothing is built; no test runs are needed (`Tests: none, docs only`). Commit `docs: selfhosting-plan, C-7 rest slice table (#125)`; no `Fixes`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/c7-rest-plan.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
