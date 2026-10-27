---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 20
---
# Brief: #871, an integer literal is magnitude and sign all the way through (no `i128`)

Score: S 2 (frees a converter gap, `docs/selfhosting-gaps.md`; one refusal fixed: `comptime_run` names the refused type `i128`), O 0.5 M (Rust over `int_value` can move), C 3 M (past parser fixes), bugs/self-hosting lane.

## Issue text (#871)
Nikaia has no 128-bit integer (`NK1135`). `tools/ast.nika` holds a literal as `LitInt { value: u64, negative: bool }`. The Rust side still turns that pair into an `i128` and back; the detour carries nothing `(u64, bool)` does not, and rs2nika has no twin type for `i128`.

## Locations
* `crates/nikaia/src/ast/mod.rs:7-40`: module comment (`i128`), `int_value` :13, `int_literal` :28, `numbered_int_literal` :35.
* `crates/nikaia/src/parser/mod.rs`: `number_lit` :273, `negative_number_lit` :295, `number_at` :305, `extern rule`s :938-939 (and their callers; `grep -n "int_literal\|int_value" crates/nikaia/src/parser/mod.rs`).
* `crates/nikaia/src/check/host.rs:97`: `int_literal(i128::from(*value))`.
* `crates/nikaia/src/comptime_run.rs:1029-1034`: an open number below `i64::MIN` gets the type name `"i128"`.

## To build (one commit, `Fixes #871`)
1. `number_lit`, `negative_number_lit`, `number_at` return magnitude and sign (`(u64, bool)` or a small struct); the `extern rule`s follow; the parser builds `Expr::LitInt` directly.
2. `int_literal` / `numbered_int_literal` take `(value: u64, negative: bool)`; `int_value` and the `i128` in the module comment go. A caller needing arithmetic asks for `i64` or `u64` as its type says and gets an option where it does not fit; `NK1116` stays where it is asked today.
3. `check/host.rs:97` passes `(*value, false)`.
4. `comptime_run.rs:1033`: an open number below `-2^63` is refused with `NK1116` at the place the value is made, not typed `"i128"`. Test `tests/compiler/tests/a_comptime_number_below_i64_min_is_refused.nika` + `.stderr` (find how an existing comptime test reaches that arm: `nikaia run scripts/where.nika -- NK1116`).
5. Drop the `i128` row from the converter section of `docs/selfhosting-gaps.md` if one is still there (d2d87c1 may have done it: then nothing).
Not part of this: the `i128` in `nikaia-std`'s own runtime (`num.rs`, `cli.rs`).

## Done when
Steps 1-5 built; `it` filters `parse` (a parser change runs the whole `it` binary: `docs/builder-facts.md`), `comptime`, `literal`; `nikaia test tests/compiler --both-settings`. Changelog: one line if step 4 changes what a program sees (the diagnostic), else none.

## Stop and report if
A caller needs an `i128` for arithmetic that `i64`/`u64` cannot hold (name it under `Found`), or an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b871-int-literal-pair.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
