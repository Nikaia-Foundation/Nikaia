---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #880, a String moved into another String variable is lowered as a view

Score: bugs lane (`docs/priorities.md`, bugs before moving): a checked program that rustc refuses; it blocks rs2nika R92 (#877, commit 7881ad4 on `origin/wip/877-entry`).

## Issue text (#880, copied)
The checker accepts this program. The lowering writes the move `dflt = arg` as a view (`__keep_frame.put(..)`), and rustc refuses the generated code with "Mismatched types".

```
fn pick(rest: String, first: bool) -> String {
    let mut dflt: String = ""
    let arg: String = rest.trim().clone()
    if first {
        dflt = arg
    } else {
        dflt = (arg.strip_prefix("|") ?? "").clone()
    }
    return dflt
}

fn main() {
    print(f"{pick(" ab ", true)}|{pick("|x", false)}\n")
}
```
Expected output: `ab||x`.

## Locations
* `__keep_frame` is written by `crates/nikaia-std/src/tools/emit_keeps.nika` (:28-67, the frame keep); an argument keep by `emit_core_args.nika` (:425, `({keep}).put(`). Which plan marks `dflt = arg` as kept: start from the generated Rust of the probe and follow the keep's key back (the keep plan, ADR-283 D24). Likely cause (issue): the later `arg.strip_prefix(..)` in the other branch makes `arg` count as lent for the whole function.
* Resolve further names with `nikaia run scripts/where.nika -- NAME...`, not `grep -r`.

## To build
1. Find why the move is lowered as a view, and lower an owned `String` moved into a `String` variable as a move. Fix it in the compiler (Rust or `tools/*.nika`; a `.nika` change needs `lower-std` and its lowered file committed, `docs/builder-facts.md`). Probe also the variant without the `else` branch and with `dflt = arg` in both branches: all three must print what Nikaia means.
2. Test `tests/compiler/tests/string_move_not_view.nika` with its `.stdout` (the probe above).
3. Finish #877 step 2: `git cherry-pick 7881ad4` (R92 from `origin/wip/877-entry`) onto `develop`, build the converter (`crates/rs2nika/`), run its tests, commit `Part of #877`. Do not delete the remote branch (the owner deletes branches).

Commit of steps 1-2: `... (Fixes #880)`.

## Done when
All three steps built; `nikaia test tests/compiler --both-settings`; `it` filters `keep`, `view`, `move`, and the guard set of `.claude/agents/builder.md`; rs2nika's tests. Changelog: one line (a program rustc refused now builds).

## Stop and report if
The fix needs a language decision (what a move of a lent binding means), or an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b880-string-move-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
