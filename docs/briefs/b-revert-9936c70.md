---
agent: builder
model: sonnet
max_turns: 40
budget_usd: 6
---
# Brief: fix-forward, revert 9936c70 (a statement begins a line)

Score: red `develop` comes first (CLAUDE.md, *A red develop is fixed forward*).

## Facts
CI on 7280a0b is red in `arm64` and `bootstrap_compiler`: the `it` binary fails in at least `contracts::a_constructor_makes_the_same_promise_or_does_not`, `comptime::a_comptime_binding_over_text_folds`, `comptime::a_comptime_binding_may_call_a_method_of_std`, `catch_handler::each_handler_binds_what_it_reads`, `build_time::the_escapes_are_the_ones_the_source_wrote`, `build_time::a_forbidden_callee_is_also_said_once`, `accepted_compiles::every_accepted_program_compiles_and_prints_what_it_should`, `block_comments::division_is_untouched`, `block_comments::a_block_comment_opener_inside_a_string_is_text`, `anonymous_constructors::new_is_refused_at_a_call` (CI lists only the first ten). Cause: 9936c70 makes two statements on one line a parse error, and many Rust tests hold one-line programs such as `fn main() { let n = 10 / 2 println(f"{n}") }`. The rule stays decided (#655); it comes back in `b655-one-statement-a-line-redo.md` with those tests rewritten. Now the commit goes.

## To build
1. `git revert --no-edit 9936c70` (it touches only its own 11 files; if it does not apply cleanly, report `blocked`). Amend nothing else.
2. Run: `cargo test --profile fast -p nikaia --test it -- contracts:: comptime:: catch_handler:: build_time:: accepted_compiles:: block_comments:: anonymous_constructors::` - all pass. The commit message: `Revert "A statement begins a line" (fix-forward 9936c70, CI red in it tests; Part of #655)`.

The main checkout holds uncommitted files of an interrupted run of `b655-operator-types.md` (`crates/nikaia-std/src/tools/check_*.nika`, their `lowered/*.rs`, `lowered.rs`, `docs/specification/30-nikaia-tooling.md`, four `tests/compiler/tests/a_*` / `an_*` files). They are not yours: do not stage, stash or change them. If a test above fails because of them (not of the revert), report it under `Found:`.
Changelog: none (the reverted change was never released).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-revert-9936c70.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
