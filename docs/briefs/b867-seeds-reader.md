---
agent: builder
model: opus
max_turns: 110
budget_usd: 20
---
# Brief: #867 last step, the reader of `emit/seeds.rs` moves to `emit_seeds.nika`

Score: self-hosting lane (`docs/priorities.md`), one of two builders' lanes; bugs lane has #655 in the same batch. Last step of #867: its commit says `Fixes #867`.

## Issue text (#867, copied)
`crates/nikaia/src/emit/seeds.rs` writes a map position's hidden seed parameters (`__S0`, ADR-010 D9) into the text of a written type, and reads which `pub` items hold maps. Writer half done (38697bf), the `Seeds` tables and type queries done (6cd8fea, `EmitSeedsTables`). Left: the reader walks.

## Locations (`crates/nikaia/src/emit/seeds.rs`, 549 lines today)
* `root_name` 50, `read_with` 66 (to 184), `count_types` 185, `hands` 217, `self_handers` 248, `read_barriers` 286 (to ~477), `count` 494, `distinct` 498, `key` 507.
* Nikaia side: `crates/nikaia-std/src/tools/emit_seeds.nika` (539 lines, prefix `emit_seeds_`), lowered to `crates/nikaia-std/src/tools/lowered/emit_seeds.rs`.
* Caller of the reader: `crate::contracts::maps::analyse_with(units, &seeds)` inside `read_with`'s fixpoint loop.

## To build
1. The walks over `ast` (`Parsed`, `Item`, `ast::Expr`, `ast::Block`) stay Rust as thin collectors: each turns the AST into plain facts (names, lists of `(String, Vec<String>)`, field type keys) and hands them to a Nikaia function that does the decision: the counting fixpoint of `count_types`, the barrier closure of `read_barriers`, the "hands a held map to a barrier" test of `hands`/`self_handers`. Leaves first (`count`, `distinct`, `key` if not yet moved). What can only be said on the AST stays Rust, named in the commit message.
2. Output of `lower-std` unchanged outside the slice's files; row in `scripts/self_hosting.py` updated.
3. Tests: `cargo test --profile fast -p nikaia --test it -- seeds maps sysroot`; `target/fast/nikaia test --both-settings --project tests/compiler`. A defect met is a `tests/compiler/` test, own commit, first. Commit: `emit/seeds: the reader's decisions move to emit_seeds.nika` with `Fixes #867`.

If a decision cannot move because a fact type is missing in Nikaia (a syntax gap, a lowering defect), stop and report it under `Found:`, do not route around it.

Read `docs/builder-facts.md` first (self-hosting rules: slices, syntax gaps, lowering).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b867-seeds-reader.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
