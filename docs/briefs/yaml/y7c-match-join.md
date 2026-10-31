---
agent: builder
model: opus
max_turns: 80
budget_usd: 20
---
# Brief: #1021, the prover joins a `match` as it joins an `if` (ADR-361 D9) (`Fixes #1021`)

Score: a prover defect y7b found (bugs before moving); small, one file family (`prover_walk.nika`), and it runs after y4c so the two never touch the same files at once. Not needed by json or yaml today.

**Read first:** `docs/builder-facts.md`, ADR-361 D9 (`docs/specification/adr/adr-361.md`: "after a branch that changes it on one side or both, it is one side's value or the other's"), then in `crates/nikaia-std/src/tools/prover_walk.nika`: the statement walk's `Expr::If` arm (about line 1271) to `prover_if`, `prover_join` (about 1372) and `prover_side` (about 1455, renames a branch's own names apart, y7b 65d86e57). `Expr::Match` falls through to `prover_expr` and is never joined. Facts y7b paid for: `prover_follow_mutables` decides whether a `let mut` is followed; `scope.rebind` clears; flatten nested `if` in `tools/*.nika` up front (clippy `collapsible_if` fires on the lowered code; grep clippy's output with `-A3` to see the `-->` line).

**Before your first build:** `cp target/fast/nikaia $S/nikaia.before` (a scratch dir), so "fails on the unfixed compiler" is one command per test.

## Issue #1021 (ready)

Probe (refused NK1300 "not a whole number the prover reads"; the `if` form of the same body is proved):

```nika
fn f(first: i64) -> i64 {
    assert(first >= 0 && first <= 10)
    let mut cp = first
    match cp > 5 {
        true => { cp = cp + 1 },
        false => {},
    }
    assert(cp >= 0 && cp <= 11)
    return cp
}
```

## To build (each step a commit)

1. **A `match` on a `bool`** (arms `true`, `false`, `else`, in either order, one arm missing allowed when `else` covers it) is walked as the `if` it is: the scrutinee is the condition, each arm a side, joined by `prover_join`. Test `tests/compiler/tests/prove_a_match_on_a_bool_joins_like_an_if.nika` + `.stdout` (the probe, and the `false` arm first).
2. **Any other `match`** joins what its arms assign: n sides instead of two (generalise `prover_join`/`prover_side`, or fold pairwise, your choice: say which in `Body:`); an arm that leaves (`return`, `throw`, `break`, `continue`) drops out as a leaving side does in `prover_join`; an arm's pattern gives a fact only where it is a whole-number literal or range and the prover reads the matched value, else none. Tests `prove_a_match_arm_assignment_reaches_the_join.nika` + `.stdout` (an `enum` match whose arms assign different bounded values), and `prove_a_match_arm_that_breaks_a_bound_is_refused.nika` + `.stderr` (NK1300). The last commit says `Fixes #1021`.

If n-way joining needs something the ADR does not say (for instance what a guard `if` on an arm gives), build without it and name it under `Found:`.

## Gate and facts

* Gate of `.claude/agents/builder.md` (#852): a `tools/prover_*.nika` change, so `lower-std`, `target/fast/nikaia test --both-settings --project tests/compiler`, the `it` filters `prove unproved_claims bounds` plus the guard set, and the package gate below. The `sequences` guard: your change must not raise it.
* Declare changed prover functions `sync` (#1014).
* `Changelog:` one sentence (a `crates/` change); the version raise is the supervisor's.

Files this brief may touch (beyond its report): `crates/nikaia-std/src/tools/prover_*.nika`, `bounds_body.nika`, their lowered files and `lowered.rs`, the files `lower-std` rewrites, the new tests.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).
* **Package gate** (this lane): `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project packages/yaml` green, and `cargo test --profile fast -p nikaia --test it -- fmt::every_nika_file` (it formats every `.nika` under `packages/`). For a change only under `packages/yaml/` this is the gate the line above asks for; a change under `crates/` runs the gate of `.claude/agents/builder.md` (#852) as well. `target/fast/nikaia` is built by the session hook; if it is missing, `cargo build --profile fast -p nikaia` once.
* A package change raises no toolchain version (CLAUDE.md table; ADR-yaml-1 D1): `Changelog: none`, unless the brief changes `crates/`.
* Files not to touch: anything outside `packages/yaml/` and the brief's own report, unless the brief names it; never `docs/briefs/batch.txt` or a brief of another lane.

* **Stop about 15 turns before `max_turns`**: commit and push every finished step, then report `Result: partial` with what is left under `Needs:`. Uncommitted work at the end is lost work (y4 ran out at 110).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y7c-match-join.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
