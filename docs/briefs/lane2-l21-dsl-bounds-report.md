---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: L2-1, the crossing `dsl.rs` goes, and `bounds_report.rs`'s walk into Nikaia (lane 2, Part #125)

Score: the bootstrap week (`docs/bootstrap-week.md`, owner's deadline 2026-11-04): lane 2's first slice, file-disjoint from P4a/P4b (`project.rs`) running in the main checkout. S=2, C~3-5 M. Target: about -300 Rust lines.

**Owns:** `crates/nikaia/src/dsl.rs` (deleted), `crates/nikaia/src/bounds_report.rs`, `crates/nikaia/src/check/mod.rs` and `crates/nikaia/src/emit/mod.rs` (only the two `crate::dsl::` call sites), `crates/nikaia/src/lib.rs` (only `mod dsl`), new `crates/nikaia-std/src/tools/bounds_report.nika`, `crates/nikaia-std/src/tools/dsl.nika`, the generated `tools/lowered/*`. **Not yours:** `project.rs` (its call `crate::bounds_report::report(` at :2107 keeps its signature), `bounds.rs`, `prove.rs`, `check.rs`'s other code, `contracts/sync.rs` (call `visit_stmt` from Nikaia only if `tools/sync.nika` already has it; else keep the Rust walk and say so).

## Facts
* **`dsl.rs` is a crossing** (108 lines, 51 code): `parameters`, `is_deferred`, `type_name`, `shadow_types`, `drivers`, `check` all exist in `tools/dsl.nika` (:26, :94, :102, :111, :147, :179; the last three take `ref winnow_grammar::InternerContext` and `ref Vec[Spanned[Item]]`). Callers: `emit/mod.rs:988` and `check/mod.rs:381` (`dsl::drivers(parsed)`), `check/mod.rs:418` (`dsl::check(parsed)`); comments in `types.rs:13`, `traits.rs:9` name it (update the words). The callers call `nikaia_std::tools::dsl::drivers(&parsed.interner, &parsed.items)` (check how `Parsed` holds them) and `dsl.rs` is deleted with its `#[cfg(test)]` module: each of its three tests (`a_hole_is_a_colon_and_a_name`, `a_path_and_a_time_are_not_holes`, `a_repeated_hole_is_one_parameter`) becomes a `tests/compiler` case only if no test there covers it yet (`grep -rl` the hole forms in `tests/compiler`).
* **`bounds_report.rs`** (292 lines, 244 code): `report` (:47) runs `crate::check::check`, `crate::prove::with_published`, `crate::bounds::proven` (Rust drivers: they stay in `report`, which keeps its signature) and then the `Walk` (:154-276, over `Block`/`Stmt`/`Expr` with `contracts::sync::visit_stmt`), `symbol` (:277), `blocked_index` (:286), `blocked_operation` (:290), `Site` (:32). Move `Walk`, `Site` and the three helpers, and the text `report` builds after the analyses, into new `tools/bounds_report.nika` (prefix `bounds_report_` where `where.nika` finds a clash); `report` passes what the analyses returned. `Walk` holds `self` and the source text: `rs2nika --file` without `--profile check-free`.
* `std` counterparts (Part III 17): `BTreeMap`/`BTreeSet` are `std::collections`; `HashSet` becomes a `BTreeSet` unless order shows in the output (it must not: same bytes first). `self.source[..at]` line counting: the `text` slice by bytes as in `project_tests.nika`; if a needed `std` call is missing, stop and report it under `Needs:`.
* Converter: run on a scratch file holding exactly the moved Rust; report `--score` under `Rs2nika:`. Do not change `crates/rs2nika/`. Under 60% kept is not your stop (the main session reads it), but say it.

## To build
1. `dsl.rs` deleted, callers call `tools::dsl` directly; commit `dsl: the crossing goes, the checker and the emitter call Nikaia (L2-1)`, `Part: #125`.
2. `bounds_report.rs`'s walk and text into `bounds_report.nika`; `report` is the analyses plus one call; commit `bounds_report: the walk into Nikaia (L2-1)`, `Part: #125`. Report the self-hosting % (`python3 scripts/self_hosting.py | tail -3`, after adding your new file to `COMPILER_NIKA` locally and **not** committing that edit: name the row under `Body:`).

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*); if it is more than one step, stop and report it under `Needs:`. Known open checker defect, not yours: #909 (write `.clone()` where rustc asks, name it in `Body:`).

## Done when
`nikaia test tests/compiler --both-settings` (every count as before); `it` tests filtered on `bounds report dsl shadow driver` plus the guard set of `.claude/agents/builder.md`; `nikaia --lib bounds` passes; clippy clean; lower-std fixpoint in the worktree after the rebase. Changelog: none (a move).

## Stop and report if
An open design choice beyond the ones above; a `.stderr`, `.stdout` or report text that would change by one byte; a missing `std` call; an error in another lane's file (`docs/bootstrap-week.md`, *File ownership*).

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; worktree lane, `docs/bootstrap-week.md` section 3)
* Read `docs/builder-facts.md` first. **You work in the worktree `/home/user/nikaia-lane2`, not in `/home/user/Nikaia`** (the driver's builder works there; never edit, build or stage anything in it except your report file). Set up once: `cd /home/user/Nikaia && git fetch origin develop && (test -d /home/user/nikaia-lane2 || git worktree add --detach /home/user/nikaia-lane2 origin/develop)`; then `cd /home/user/nikaia-lane2 && git status` (another agent of your checkout may have left files: not yours, report them). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/nikaia-lane2/target/fast/nikaia`, never another checkout's binary.
* `git add <files>` only; commit after each step. Push: `git fetch origin develop && git rebase origin/develop`, rebuild, `lower-std` once more, commit any lowered file that changed (`lowered: regenerate after rebase`), then `git push origin HEAD:develop`; rejected: again, at most three times, then `Pushed: rejected`. A conflict in `tools/lowered/*.rs` or `tools/lowered.rs` is never merged by hand: `git checkout --theirs -- <file>`, `git rebase --continue`, rebuild, `lower-std`, commit.
* Commit every lowered file `lower-std` changed, yours and others' (a compiler change re-lowers other files; that is correct in your tree). Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap-week.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most three checkouts with `target/` this week (C1, C2, C3); if `df -h /home/user` shows under 4 GB free, delete `/home/user/nikaia-lane2/target` and report `blocked`. At the end delete `/home/user/nikaia-lane2/target/debug` and `/home/user/nikaia-lane2/target/release/nikaia-scratch` (keep `/home/user/nikaia-lane2/target/fast` for the lane's next brief) and report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/lane2-l21-dsl-bounds-report.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
