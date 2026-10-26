---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 15
---
# Brief: #655 step 3, `?? return` on a field of a `mut` parameter is lent

Score: bugs lane (`docs/priorities.md`): the checker passes a program rustc refuses (E0507), a wrong-acceptance bug; one step, about 10 lines in one function.

## Issue text (#655, item 2 and step 3)
`state.flow.unrolling ?? return`, where `state` is a `mut` parameter whose type does not copy, lowers by value, and rustc refuses it (E0507). A `ref` parameter gets `.as_ref()` and runs. The place to fix is `lent_coalesces` in `check_core_ops.nika`. To build 3: a field of a `mut` parameter read through `?? return` is lent. Test.

## Probe (reproduced by the architect on fbc9bfd, `target/fast/nikaia run n.nika`)
```
struct Unroll { names: Vec[String] }
struct Flow { unrolling: Unroll? }
struct State { flow: Flow }
fn first(mut state: State) -> i64 {
    let u = state.flow.unrolling ?? return 0
    return u.names.len()
}
fn main() {
    let mut s = State { flow: Flow { unrolling: Unroll { names: ["a"] } } }
    println(first(s).to_string())
}
```
Today: rustc *Cannot move out of `state.flow.unrolling` ... behind a mutable reference*. Wanted: prints `1`. With `unrolling: String?` it already runs (prints the length). The decision is settled: a `mut` parameter is a loan, so ADR-279 D10 (*a loan on the left and a jump on the right is a view*) applies; no design choice is open.

## Locations
* `crates/nikaia-std/src/tools/check_core_ops.nika`: `check_core_ops_coalesce` (724), the D10 arm (~774: `place && state.jumps(..) && ... && (check_core_ops_lent_root(..) || check_core_ops_behind_a_view(..))`); `check_core_ops_lent_root` (563) answers `state.lent_here(root) || got_past_a_jump`; find why a `mut` parameter's root is not `lent_here` (grep `fn lent_here` in `tools/check_*.nika`) and fix it there or in `lent_root`, whichever is narrower.
* Lowering reads `lent_coalesces` in `tools/emit_core_flow.nika:588`; no change expected there.

## To build
1. The fix above, then `lower-std` with the rebuilt binary; commit the changed `lowered/*.rs` with it.
2. Tests in `tests/compiler/tests/`: `a_mut_parameters_field_past_a_jump_is_lent.nika` + `.stdout` (the probe), and a second where the view is then used after a write to another field of `state` (`state.flow` untouched, e.g. a new `count: i64` field incremented) - if rustc refuses that, the checker must refuse it with the existing borrow code, not let it through; report which.
Run: `cargo test --profile fast -p nikaia --test it -- coalesce lent view`, `target/fast/nikaia test --both-settings --project tests/compiler`. Commit `Part of #655` (steps 1, 2, 4, 5 stay open).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-mut-param-coalesce.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
