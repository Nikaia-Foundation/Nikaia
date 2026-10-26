---
agent: builder
model: sonnet
max_turns: 80
budget_usd: 10
---
# Brief: `.trim().to_string()` must yield an owned `String` (#851)

Bug lane. Compiler change: the supervisor raises the version; you do not.

## Facts
* Probe (issue #851, reproduced on 0.0.1222):
  `let a = "  x ".trim().to_string()` then `process::run("echo", [a])` gives
  `NK1102: ... expects args to be Vec[String], but you're passing Vec[ref String]`.
  `.trim().clone()` is accepted.
* Two suspects: `to_string` on a `ref String` typed as `ref String`, or the list
  literal `[a]` taking the view type of the local's origin.
* Names: `nikaia run scripts/where.nika -- NAME`; known facts `docs/builder-facts.md`.

## Files (only these)
The checker side: `crates/nikaia/src/check/**` and `tools/check_*.nika`,
`tools/signature*.nika` / `described.contracts` if the typing lives there;
`tests/compiler/tests/to_string_owned.nika` + `.stdout`; `scripts/agent.nika`
(step 4 only). Do NOT touch `crates/nikaia/src/emit/**`, `tools/emit_*.nika`,
`scripts/self_hosting.py`, `docs/selfhosting-plan.md`: the #855 builder owns them.
If the fix needs an emit file, stop and say so under Needs.

## To build
1. `target/fast/nikaia check` on the probe and on a variant with `let b: String = a`;
   find which suspect it is. Write it in the commit message.
2. Fix in the compiler so `to_string()` yields `String` (and a list literal of
   owned locals is `Vec[String]`).
3. Test `tests/compiler/tests/to_string_owned.nika` + `.stdout` with the probe
   (use `echo`, as the issue does). Run it and the touched `it` tests
   (`cargo test --profile fast -p nikaia --test it -- <mods>`), grep the NK1102 tests.
4. Replace any `.trim().clone()` in `scripts/agent.nika` that exists only
   because of this with `.to_string()`; `nikaia run` it once with `--help` or a dry brief.
5. If any `tools/*.nika` changed: `target/fast/nikaia lower-std`, commit the diff.
   `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`.
   Commit `Fixes #851`.

## Rules (CLAUDE.md)
* `develop` in the main checkout; `git add <files>` only; push fast-forward, no rebase. Commit after each step.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite.
* Fix, do not work around; a second defect met is filed as a `ready` issue.
* `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/`; delete `target/release/nikaia-scratch`; report GB freed. Scripts in files, small reads, no foreground sleep.

## Report (at most 15 lines, `<brief path>.report.md`)
Result, Files, Commit, Fixes, Pushed, Tests, Deviation, Found (which suspect), Needs, GB freed, Owner (form: `docs/architect.md`, *Briefs and reports*).
