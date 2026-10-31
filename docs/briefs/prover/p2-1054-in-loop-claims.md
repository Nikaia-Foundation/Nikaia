---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief p2: #1054, a claim inside a loop body is asked through its own loop (ADR-361 D2, D3, D10)

Facts: Where things live, Traps already met, Commands

Score: S=3 (a decided rule that does not work: ADR-361 D2 says "after the loop or inside it", and only *after* is built. It gates most checks in `tools/*.nika`, which sit inside loop bodies: 281 of the unproved operations are `x += 1` counters), O=3 M (every later counter and index extension in the lane rests on it), C=10 M (the prover's loop path; like the turn-indexed intervals, ADR-361 D12), area prover.

## Problem (issue #1054, probed on d206915)

```nika
fn f(n: i64) -> i64 sync {
    let mut i = 0
    let mut j = 0
    while i < n {
        assert(j <= i)      // today: refused NK1300, "nothing before it shows it"
        j += 1              // under prove-through-loops:on: still checked
        i += 1
    }
    assert(j <= i)          // today: proved through the loop, invariant
    return j                //   i >= 0 && j >= 0 && i == j && (i < n + 1 || i == 0)
}

fn h(n: i64) -> i64 sync {
    let mut i = 0
    while i < n {
        assert(i >= 0)      // today: refused
        i += 1
    }
    return i
}
```

Run: `target/fast/nikaia check --asserts FILE` and `target/fast/nikaia check --bounds --optimization prove-through-loops:on FILE`. ADR-361 D2 (`docs/specification/adr/adr-361.md`): a claim that reads a name a loop changes, *after the loop or inside it*, is asked once more through the loop's problem. D3: an `assert` in a loop is proved through the loop where it stands in the flow, and once proved it is a fact. D10: under `prove-through-loops:on` the compiler's own index and overflow checks are asked the same way.

## Locations (suspected, medium confidence)

* `crates/nikaia-std/src/tools/prover_state.nika`: `fn through_loops` (about line 210) and its caller (about line 437, *Asked once more through the loops before it*). It walks `scope.loops`, the loops recorded **before** the claim. The loop that encloses the claim is recorded only when the walk leaves it, so an in-body claim never sees it. At the claim, the facts are: the loop's invariant (at the head), its condition, and the path from the head to the claim.
* `crates/nikaia-std/src/tools/prover_loops.nika`: `loop_found`, `loop_candidates` (line 448), `loop_recorded`, the `LoopRecord` (line about 20).
* `crates/nikaia-std/src/tools/bounds_walk.nika`: `through_loops` (lines 75, 472, 2186) and `walk.collecting`, the path for the checks under D10.
* Resolve more with `nikaia run scripts/where.nika -- NAME` and `--outline FILE`. A changed `.nika` is re-lowered with `lower-std` (builder-facts) and the diff in `crates/nikaia-std/src/tools/lowered/` is committed.

## Steps

1. **An `assert` inside a loop body**, reading a name the loop changes, is asked through the enclosing loop (and through every loop it is nested in) where the walk leaves it open. Test `tests/compiler/tests/prove_an_assert_inside_a_loop_is_asked_through_it.nika` with `.stdout`: both functions above, called from `main`, printing their results. Today it is refused, so the test fails against the unfixed compiler. Say so under `Tests:`. Also check that an in-body claim that is false at some turn is still refused or refuted, not proved: for example `assert(i < 3)` in `h`, called with `n = 5`, has to stay refused. Add that row to an existing refused-claims test of the same kind, or make it a second test with a `.stderr`.
2. **Under `prove-through-loops:on`, the checks inside a body** (D10): `j += 1` in `f` is proved. `tests/compiler` is one project with one `nikaia.toml`, so no `.nika` test can set the optimization per test. Show it with `check --bounds --optimization prove-through-loops:on` on the probe under `Tests:`. That is no deviation. Keep `crates/nikaia/tests/it/prove_through_loops.rs` passing, and add no Rust test.
3. Run `nikaia check --bounds --optimization prove-through-loops:on` on `crates/nikaia-std/src/tools/logic_search.nika` before and after, and give both header lines under `Tests:`. Also give the lane's figure if `nikaia lower-std --bounds` exists by then (brief p1).

Stop about 15 turns before `max_turns`. Commit what is finished and report `partial`, with what you ruled out.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `docs/briefs/prover/p2-1054-in-loop-claims.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
