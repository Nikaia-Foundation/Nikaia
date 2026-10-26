---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 20
---
# Brief: #655 step 2b, `if c { null } else { given[1] }` under `ref E?`; then `check_arguments_at` goes

Score: bugs lane (`docs/priorities.md`): a false refusal (NK1103) that keeps a detour in our own code (`check_arguments_at` in `check_arguments.nika`, kept in 2e1ba5d for this reason); small, one area. Runs after step 2a (`b655-element-view-slots.md`).

## Issue text (#655, item 1, step 2)
- `if c { null } else { given[1] }` is NK1103: the branch is typed `E?`.
To build 2: a view of an element as ... an `if` branch under `ref E?`: it lowers to `Some(&..)` and is accepted. One test per probe. After step 1: `check_arguments` drops `check_arguments_at`.
Decided by ADR-354 D1 (an arm `given[0]` under `ref Expr?` is a view of the element); `null` stays `None`.

## Probes (`struct E { n: i64 }`; architect ran them on 2e1ba5d's binary)
* (a) `fn f(given: ref Vec[E], c: bool) -> i64 { let y: ref E? = if c { null } else { given[1] }; return (y ?? return 0).n }` (one statement a line), main calls `f(g, false)`: NK1103 *This value is `E?`, but the `let` declares `ref E?`*. Expected `2`; with `true`, `0`.
* (b) the same as a `match` with a `null` arm, and as a function result `-> ref E?` whose last value is that `if`.
* (c) control, already fine: `if c { t } else { given[1] }` with `t: ref E`; and `let mut y: ref E? = null` / `y = t`.

## Then: the detour
`crates/nikaia-std/src/tools/check_arguments.nika`: `check_arguments_at` 19-ish (comment says it exists because a `ref Expr?` is not built from an `if`) is replaced at its one call (~line 219, `given: check_arguments_at(given, at)`) by the `if` it stood for, and deleted. If it still fails for another reason, that is a `Found:` with the probe.

## Locations
* `crates/nikaia-std/src/tools/check_arm_leaves.nika`: the arm walk ~595-645 (`check_core_stmt_element_view` at ~616 under `what == "let"`; `ends_in_null`, `note_mixed` 104); where an `if`'s type is joined from `null` and an element arm before the `let`'s type is asked: find with `nikaia run scripts/where.nika -- NAME` from there.
* `crates/nikaia-std/src/tools/check_core_stmt.nika` `check_core_stmt_element_view` 330.
* Generated files: `crates/nikaia-std/src/tools/lowered/*.rs` (`docs/builder-facts.md`).
* Tests: new `tests/compiler/tests/a_null_arm_beside_an_element_view_is_an_optional_view.nika` + `.stdout` (a and b).

## Done when
Probes (a)-(b) print as stated, (c) unchanged, `check_arguments_at` is gone. `nikaia test tests/compiler --both-settings` passes, plus `it` filters `element_views`, `views_kept`, `nullable`, `match_arms`. Commit `A null arm beside a view of an element is an optional view (Part of #655)`; no `Fixes` (step 5 remains). Changelog line: yes.

## Stop and report if
An open design choice: `Result: blocked`, the question under `Owner:`.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-null-arm-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
