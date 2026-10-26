---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 20
---
# Brief: #665, a lent `let` over a place handed back (`let k = p.items; return k` fails in rustc)

Score: S=3 (a program the checker accepts fails in rustc), O=0.3 M, C=5 M (#848/#864-#866 fixes), bugs lane 1 (`docs/priorities.md`).

## Issue text (#665)
The #453 step 3 builder hit this. Probe (reproduces at f691147, `target/fast/nikaia run a.nika`: rustc *expected `Vec<_>`, found `&Vec<_>`* at `return k`):

    struct P { items: Vec[i64] }
    fn back(p: P) -> Vec[i64] {
        let k = p.items
        return k
    }
    fn main() {
        let v = back(P { items: [1, 2, 3] })
        print(v.len())
    }

It lowers to `let k = &p.items; k`. The `let` over a place is lent (ADR-094 D4), then handed back; nothing refuses the return (NK2106), nothing makes it a move.

To build:
1. Where a lent `let` over a place escapes (a return, a push, a field, an argument the callee keeps), lower it as a move out of the place when the place's owner is owned here (not `ref`) and is not read again. Otherwise refuse with the existing NK2106, saying what to write (`.clone()`). Decide from what `hands_over` and `check_core_returns` already record.
2. Tests (`tests/compiler/tests/`, `.nika` + `.stdout`/`.stderr`): the probe runs and prints 3; the same through a push into a `Vec[Vec[i64]]`; a variant that reads `p.items` again after the `let` is refused (NK2106 `.stderr`); the same over a `ref P` parameter is refused.

## Locations (re-grep with `nikaia run scripts/where.nika -- NAME`)
* Lent lets recorded: `crates/nikaia-std/src/tools/check_core_stmt.nika:307` (`state.facts.lent_lets.insert`), `:314`; `check_binds.nika:68, 206`; field in `check_facts.nika:327`.
* Lowered `&`: `crates/nikaia-std/src/tools/emit_stmt.nika:465` and `:519` (`facts.lent_lets`).
* Escape checks: `hands_over` `tools/check_arm_leaves.nika:305`; `check_core_returns` `tools/check_core_leaves.nika:458`.
* NK2106 tests: `crates/nikaia/tests/it/handed_over.rs`, `tests/compiler/tests/let_lent_field_kept.*`; spec Part III `docs/specification/30-nikaia-tooling.md:1713`.

## Steps
1. Read `docs/builder-facts.md`. Reproduce the probe, write the tests first.
2. Fix in the `.nika` files above; `target/fast/nikaia lower-std`, `cargo build --profile fast -p nikaia`, commit the `lowered/*.rs` diff.
3. Tests: the new ones, `nikaia test --both-settings --project tests/compiler`, `cargo test --profile fast -p nikaia --test it -- handed_over:: sysroot`. Commit `... (Fixes #665)`.
Stop and report (`Owner:`) if the move-vs-refuse choice needs a spec change beyond Part III's NK2106 text.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b665-lent-let-returned.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
