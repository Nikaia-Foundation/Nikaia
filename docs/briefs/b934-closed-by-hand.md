---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief #934: NK2605 although the value is closed by hand (`f.close() catch { ... }`)

Score: bugs before moving (`CLAUDE.md`, `docs/priorities.md`); a wrong refusal whose own help text recommends the refused form, and it blocks the move of `project.rs`' `trace` (plan row P5a). Checker lane only (`check_crossings.nika`, `check_compare.nika`); the first brief of this batch is in the lowering (`views.nika` / `text_tiers.nika`).

## Issue text (#934, copied; builders have no GitHub)
Found by P5a (2e13458). A value with a `Cleanup` whose cleanup can fail, closed explicitly with `close()` and a statement `catch`, is still refused with NK2605. Probe (re-run on 6ccf6c8: NK2605 at the `let`, line 24; expected output `close: closed would not flush` then `after close`):

```nika
enum FlushError {
    Refused(String),
}

impl Error for FlushError {
    fn message(ref self) -> String {
        match self {
            FlushError::Refused(name) => f"{name} would not flush"
        }
    }
}

struct Flaky {
    name: String,
}

impl Cleanup for Flaky {
    fn cleanup(ref mut self) throws {
        throw FlushError::Refused(f"{self.name}")
    }
}

fn closing() {
    let f = Flaky { name: f"closed" }
    f.close() catch { println(f"close: {error}") }
    println("after close")
}

fn main() {
    closing()
}
```

The `it` test `cleanup_points` (`FLAKY`, `crates/nikaia/tests/it/cleanup_points.rs:142`) holds this very `closing()`, but it lowers without the checker, so it never saw NK2605. The refusal's help even says *close it yourself where you want to handle the failure: `f.close() catch { … }`*.

## Locations (resolved 2026-10-29)
* The refusal is raised eagerly at the `let`: `check_crossings_a_value_with_a_cleanup_bound` (`crates/nikaia-std/src/tools/check_crossings.nika:272`) calls `check_crossings_a_value_with_a_cleanup_dying` (:259), which calls `state.a_failing_cleanup_undeclared` (`check_compare.nika:318`, the NK2605 text at :333). It does not know yet whether the value is closed or moved later in the block.
* The explicit close resolves to `cleanup::close` in `check_core_call_on.nika:192`. Where the value's cleanup is settled at the block's end: `state.facts.settle_lets` (written at `check_crossings.nika:274`); `cleanup_of` is `check_fallible.nika:218`.
* The other caller of `..._dying`: `check_function.nika:718` (a kept parameter).
* Lowered twins under `crates/nikaia-std/src/tools/lowered/` (regenerate with `target/fast/nikaia lower-std`, never edit by hand). Tests of NK2605: `crates/nikaia/tests/it/cleanup_points.rs`, `errors_lowering.rs`, `file_paths.rs` (`where.nika -- NK2605`).

## To build
1. Tests in `tests/compiler/tests/`: `a_failing_cleanup_closed_by_hand_is_handled.nika` (the probe, `.stdout` `close: closed would not flush\nafter close\n`), and `a_failing_cleanup_left_to_the_block_end_is_nk2605.nika` (the same without the `close()` line, `.stderr` with NK2605, the shape of the other `.stderr` tests there). Run the first on the unfixed compiler and see NK2605.
2. Fix in the checker: NK2605 for a failing cleanup asks only of a value whose cleanup is implicit, i.e. still held where its block ends; a value closed with `close()` (its failure handled by the `catch`, or passed on in a `throws` function) on every path is not refused. One way: keep the finding pending at the `let` and drop it when the value is closed; you choose, and say which in `Body:`. A value closed on one branch only stays refused. `lower-std` twice (fixpoint). Commit `Fixes #934`.

## Done when
Both new tests pass run through `target/fast/nikaia test --project tests/compiler` filtered to their names (the `.stdout` is compared byte for byte), `cargo test -p nikaia --test it -- cleanup errors_lowering file_paths`, the gate of `.claude/agents/builder.md` (a change to `tools/check_*.nika` runs `nikaia test --both-settings --project tests/compiler`), clippy, `cargo fmt`, `scripts/pre-push-check.sh`. Changelog: one sentence.

## Stop and report if
The rule needs a choice Part I or ADR-297 leaves open (a close on one branch, a close inside a loop): name it under `Owner:`; or the fix needs a file another agent has uncommitted (`git status` first; `crates/rs2nika/convert/nikaia.*` were another session's).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b934-closed-by-hand.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
