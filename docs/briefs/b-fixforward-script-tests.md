---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: fix-forward, the script tests run the toolchain that tests them (develop red)

Score: red `develop` first (`CLAUDE.md`): `main` stands still until this lands. Test-only change: no version raise, no Changelog.

## Facts (checked by the architect on CI run 40359579581, de7c3d3d)
* Jobs `bootstrap_compiler` and `arm64` fail in `nikaia_test::the_compiler_test_package_passes_at_both_settings`: `104 passed; 4 failed` (earlier `98 passed; 2 failed`, before `batch_lines` and `drive_actions` were added). Locally, with `target/fast/nikaia` present, all 110 pass: that is why the earlier fix-forward (`b-fixforward-compiler-tests.md`) did not reproduce it.
* The four tests take the binary from a fixed path that CI does not build (CI builds `target/release`):
  `tests/compiler/tests/agent_script.nika:19`, `batch_frontmatter.nika:27`, `batch_lines.nika:22`, `drive_actions.nika:24`:
  `let nikaia = process::run("realpath", [f"{repo}/target/fast/nikaia"]).stdout.trim().clone()`
* Their `root()` already finds the checkout from the process that runs the test: `readlink /proc/$(cut -d' ' -f4 /proc/$PPID/stat)/cwd` (the parent of the test program is the `nikaia` running `nikaia test`).

## To build
1. In each of the four files, take the binary of that same process: `readlink /proc/$(cut -d' ' -f4 /proc/$PPID/stat)/exe`, as a function `toolchain()` beside `root()`. If its file name is not `nikaia` (the test was started otherwise), fall back to the first that exists of `{repo}/target/release/nikaia`, `{repo}/target/fast/nikaia`. No other change to the tests' output.
2. Probe, CI's situation, before and after the change (`S` = a scratch dir of yours):
   `cp target/fast/nikaia $S/nk && mv target/fast/nikaia target/fast/nikaia.off`, then `NIKAIA_CACHE_LIMIT=2G $S/nk test --both-settings --project tests/compiler` (before: the 4 fail; after: all pass), then `mv target/fast/nikaia.off target/fast/nikaia` (always, also on failure: other sessions use it).
3. Also `cargo test --profile fast -p nikaia --test it -- nikaia_test::the_compiler_test_package`. Commit `tests: the script tests run the nikaia that tests them, not target/fast (CI red since 6370390)`.

## Files you may change
The four `.nika` files above (their `.stdout` only if a line truly changes: say why under Deviation). Anything else: stop and report under Needs.
Known facts: read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-script-tests.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
