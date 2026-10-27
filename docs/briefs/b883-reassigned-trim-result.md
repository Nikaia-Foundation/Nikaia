---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #883, a reassigned String local whose trimmed view becomes the String result goes through the keep frame

Score: bugs lane (`docs/priorities.md`, bugs before moving): a checked program rustc refuses; found by b881 (0fa94bb), same family as #880/#881 (`tools/buffers.nika` origins).

## Issue text (#883, copied)
Probe:

```
fn tail(head: String) -> String {
    let mut st: String = head.clone()
    let mut after: String = ""
    st = after
    let n: String = st.trim()
    return n
}

fn main() {
    print(tail("  x  "))
}
```

Seen: checks, then rustc refuses `st = __keep.put_viewed(after)` (Mismatched types). Expected: `after` is owned, `st = after` is a move, `n` is a copy of the trimmed text. Prints `x`.

## Locations
* `crates/nikaia-std/src/tools/buffers.nika`: `a_local` :190, `copies_text` :529, `flow_into` :719, `origins_of` :1083, `origins_of_local` :1178, `method_origins` :1209. Read the diffs of fef8e78, ae26238c (#880) and 0fa94bb (#881) first: the same mechanism.
* `put_viewed` is chosen in `tools/emit_stmt.nika` `emit_stmt_put_call` :77 (`KeepAt::Param` here: `st` is taken as reaching the `String` parameter's keep). Other names: `nikaia run scripts/where.nika -- NAME...`, not `grep -r`.

## To build
1. Reproduce on `develop`. Probe the variants: without `st = after`; `let n = st.trim()` untyped; `return st.trim()`; `mut head: String` used as `st` directly. Note which fail.
2. Fix where the origin is computed (a `let n: String = <view>` copies text, so `st` and `after` do not reach the result's keep) in `buffers.nika`; `lower-std`; commit the lowered file with it.
3. Test `tests/compiler/tests/string_reassigned_trim_result.nika` + `.stdout` (the probe, plus the variants that failed in step 1). Commit `... (Fixes #883)`.

If step 1 does not reproduce: still `done`, do step 3 to pin it, say so under `Tests:`.

## Done when
All steps; `nikaia test tests/compiler --both-settings`; `it` filters `keep`, `view`, `move`, and the guard set of `.claude/agents/builder.md`. Changelog: one line if the compiler changed, else `none`.

## Stop and report if
The fix needs a language decision (what a lent binding's move means), or an error in another agent's file. #882 (a `String` parameter read through `??` counted as kept) is another lane's: do not touch it.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b883-reassigned-trim-result.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
