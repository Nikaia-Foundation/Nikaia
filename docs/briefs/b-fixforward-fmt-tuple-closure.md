---
agent: builder
model: sonnet
max_turns: 80
budget_usd: 15
---
# Brief: fix-forward, the formatter breaks a nested tuple-pattern closure (CI red since 0f9a525, P5d)

Score: a red `develop` is fixed forward first (`CLAUDE.md`); `main` stands still until then (last green c53ddd4). A compiler bug (the formatter is self-hosted), before any move.

## Facts (checked by the architect, 2026-10-29)
* CI run 40838071412 (0f9a525, P5d): `bootstrap_compiler` and `arm64` fail one test only, `fmt::every_nika_file_formats_to_the_same_tree_and_a_fixed_point` (`crates/nikaia/tests/it/fmt.rs:190`; it formats every parsing `.nika` under `examples tests benches crates/nikaia-std/src crates/nikaia/tests packages` and fails on a guard refusal or a second pass that changes the text). The push before (c53ddd4) was green; 0f9a525 added `crates/nikaia-std/src/tools/project_check.nika`. P5b's `project_lower.nika` formats fine.
* `target/fast/nikaia fmt` on a copy of `project_check.nika`: `not formatted: the formatted file does not parse: Expected `_` or identifier here, but found `)`.` The first failing item is `project_check_keeps_that_moved` (lines 434-449), the line 440 chain.
* Minimal probe (refused the same way, line 5 column 2), the file `p.nika`:
  ```nika
  fn f(v: ref Vec[(String, bool)]) {
      let m = v.iter().filter(fn((param, _)) { param != "self" && then.params.iter().any(fn((p, _)) { p == param }) }).filter_map(fn((param, ty)) {
          return 1
      }).collect()
  }
  ```
  Formats fine: the same without the nested `any(fn((p, _)) ...)`, or with `fn(param)` instead of `fn((param, _))`, or the short one-liner `v.iter().filter(fn((a, _)) { w.iter().any(fn((q, _)) { q == a }) }).count()`. So the long line is cut, and the cut lands inside a closure's tuple pattern.
* The formatter: `crates/nikaia-std/src/tools/fmt.nika` (951 lines; `crates/nikaia/src/fmt.rs:21` calls `fmt_source`). Likely place: `fmt_sealed` (:851) seals a pattern only when `=>`, `|`, `=` or `in` follows it, not a closure parameter pattern `fn((a, b))`, so `fmt_breakable` (:865) lets it go one per line with a trailing comma (`(param, _,)`). Confirm before fixing; `fmt_bar_opens` (:480) is the `|...|` analogue.

## To build
1. Find the cut (print the formatter's output for the probe: e.g. a temporary `println` in a scratch build, or `nikaia::fmt::format` from a scratch `it` filter; nothing of that committed) and fix it in `tools/fmt.nika`: a closure's parameter list and the tuple patterns in it stay sealed (no cut, no trailing comma). Run `target/fast/nikaia lower-std` (only `lowered/fmt.rs` may change), rebuild.
2. Test: `tests/compiler/tests/fmt_keeps_a_nested_tuple_closure_whole.nika` with the probe's shape made runnable (a `fn main` that prints the count, its `.stdout` beside it; a line over 100 columns with the nested tuple-pattern closure followed by a multi-line closure). The `it` scan then covers it. Check: `target/fast/nikaia fmt` on a scratch copy of it and of `project_check.nika` formats (no refusal), and formatting the result again changes nothing.

## Done when
`cargo test --profile fast -p nikaia --test it -- fmt::` passes (it fails before the fix on `project_check.nika`; say so under `Tests:`); `nikaia test --project tests/compiler` filtered to the new test passes; the gate of builder.md; clippy, `cargo fmt`, `scripts/pre-push-check.sh`. Commit: `fmt: a closure's tuple pattern is not cut (fix-forward, CI red since 0f9a525)`, `Fixes` none. Changelog: one sentence (the formatter is the toolchain's).

## Stop and report if
The cut is not in `fmt.nika` (then name where), or the fix changes the formatting of another `.nika` in the repository (run the `fmt::` filter; a changed file elsewhere is a `Deviation`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-fmt-tuple-closure.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
