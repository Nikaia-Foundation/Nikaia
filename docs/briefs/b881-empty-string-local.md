---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #881, a String local that starts as "" and is assigned later is lowered into __keep_frame

Score: bugs lane (`docs/priorities.md`, bugs before moving): a checked program rustc refuses; found by rs2nika (#877), same family as #880 (`tools/buffers.nika` origins).

## Issue text (#881, copied)
Probe (run on c7b90a1, before fef8e78):

```
fn tail(head: ref String) -> String {
    let mut st: String = (head.trim().strip_prefix("impl") ?? "").clone()
    if st.starts_with("<") {
        let mut after: String = ""
        for ch in st.chars() {
            after.push(ch)
        }
        st = after
    }
    let f = st.rfind(" for ") ?? 0
    if f > 0 {
        st = st[f + " for ".len()..].clone()
    }
    let mut name: String = ""
    for ch in st.trim().chars() {
        if ch == ' ' {
            return name
        }
        name.push(ch)
    }
    return name
}

fn main() {
    print(tail("impl<T> Foo<T> {"))
}
```

Seen: the local `after` is lowered as `__keep_frame.put(..)`, and `st = after` fails in rustc with "Mismatched types". Expected: `after` is an owned `String`, `st = after` is a move; the program prints what Nikaia means (the issue says `Foo<T>`; check by reading the code, as b880 found its expected output was a slip).

## Locations
* `crates/nikaia-std/src/tools/buffers.nika`: `copies_text` :529, `origins_of` :1080, `origins_of_local` :1175, `method_origins` :1206, `flow_into` :716, `a_local` :190. #880's fixes: fef8e78 and ae26238c (read both diffs first: the same mechanism).
* `__keep_frame` is written by `tools/emit_keeps.nika`. Resolve other names with `nikaia run scripts/where.nika -- NAME...`, not `grep -r`.

## To build
1. Run the probe on today's `develop`. If it builds and prints the right line, the result is still `done`: do step 3 (the test pins it), and say under `Tests:` that it did not reproduce; commit `Fixes #881`.
2. Otherwise find why `after` gets origins (a `""` start, `push` of a `char` from `st.chars()`, the move `st = after`) and fix it in `buffers.nika` (then `lower-std`, commit the lowered file; `docs/builder-facts.md`). Probe also: without the loop (`after = "x"` then `st = after`), and with `after.push_str(st.trim())`.
3. Test `tests/compiler/tests/string_empty_then_moved.nika` with its `.stdout` (the probe). Commit `... (Fixes #881)`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b881-empty-string-local.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
