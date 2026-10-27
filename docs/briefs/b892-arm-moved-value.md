---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: a value moved in an arm, bound by a `let` (Fixes #892)

Score: bugs lane (`docs/priorities.md`, bugs-first default): a soundness gap, the checker passes a use after move that rustc refuses; found by the #888 builder.

## Issue text (#892)

Found by the #888 builder (report `docs/briefs/b888-let-bound-proved-name.md.report.md`), reproduced by the architect on 3b77d7c.

## Probe
```nika
struct P { name: String }
fn main() {
    let t = P { name: "a" }
    let c = true
    let s: P = if c { t } else { P { name: "b" } }   // also: match c { true => t, false => ... }
    println(s.name)
    println(t.name)
}
```
`nikaia check` passes; `nikaia run` fails in rustc: *Borrow of moved value: `t`*. The control `let s: P = t; println(t.name)` is NK2105 (*`t` was bound to `s` by a `let`*). The proved form (`let s: P = if p != null { p } else { ... }`, then a read of `p`) does the same.

## Cause
`hands_over` (`crates/nikaia-std/src/tools/check_arm_leaves.nika:389`) returns at once unless the value is a name or a field path, so the tails of an `if`/`match`/block value are never handed over. `narrowed_reads_given_back` (same file, :356) already walks those tails (if with else, block tail, each match arm body).

## To build
1. `hands_over` walks the tails of `Expr::If` (with an `else`), `Expr::Block` and `Expr::Match` the way `narrowed_reads_given_back` does, handing over each tail that is a name or field path (one arm moving is enough for NK2105 after the `if`). Test: `tests/compiler/tests/a_value_moved_in_an_arm_is_handed_over.nika` + `.stderr` (if form, match form, proved form: each NK2105), and a passing control where the moved name is not read again, with `.stdout`.
2. Check every caller of `hands_over` (10 in 7 files, `nikaia run scripts/where.nika -- hands_over`): function arguments and struct fields given an `if` value get the same rule; `lower-std` re-lowers the touched files, and the guard set of `tools/*.nika` still checks (a tools file that now hits NK2105 is a finding in that file: fix it with `.clone()` only where the value is really read again, and say so in `Body:`).

Rules: ADR-094 D4, ADR-293 D29 (a rename moves). A text (`String`) moved in an arm and read later already runs (the checker copies it); keep that behaviour.

## Known facts
Read `docs/builder-facts.md`. The `let` site that calls `hands_over`: `crates/nikaia-std/src/tools/check_core_stmt.nika:519`. The commit says `Fixes #892`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b892-arm-moved-value.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
