---
agent: builder
model: opus
max_turns: 50
budget_usd: 10
---
# Brief: #1056, NK2105's help says `t.clone()` for an optional handed over and read again

Score: y18's `Found:` (f0ae806e): a help line that suggests code the compiler refuses. Small, one file plus a test; built here because this lane found it and the next steps (`y3`, `y19`) do not touch `check_takes.nika`. It changes `crates/`, so the gate of `.claude/agents/builder.md` applies in full (a change to `tools/check_*.nika` also runs `target/fast/nikaia test --both-settings --project tests/compiler`), and it raises the toolchain version (the supervisor's: write the `Changelog:` line, touch no version). Read `docs/builder-facts.md` and `git show f0ae806e -- crates/nikaia-std/src/tools/check_takes.nika` (the guard's refusal already writes `?.clone()` for an optional, `check_takes.nika:377-380`).

## Issue #1056 (copied)

The plain-call NK2105, `data_handed_over_is_used_again` (`crates/nikaia-std/src/tools/check_takes.nika:414`), writes ``If both need it, hand over a copy: `{name}.clone()`.`` whatever the type; `t.clone()` on a `T?` is refused, `t?.clone()` is the copy that compiles. Probe: a `fn keep(t: Node?) -> String` that binds `t` in a `match` arm, called twice with the same `t: Node?` in `main`. Today: NK2105 at the second call, help `` `t.clone()` ``. Wanted: `` `t?.clone()` ``.

## To build

1. Probe on the unfixed binary first; check that `t?.clone()` in the probe really compiles and runs (if it does not, stop: `Result: blocked`, `Found:` with the probe).
2. In `data_handed_over_is_used_again`, write the copy as the repeat-frame refusal does (`?.clone()` when the type's text ends with `?`); one shared helper for both sites if it reads better. Regenerate the lowered twin as `docs/builder-facts.md` says.
3. Test `tests/compiler/tests/takes_an_optional_handed_over_twice_says_its_copy.nika` + `.stderr`, failing on the unfixed binary. An existing `.stderr` whose help rightly changes is updated and named in `Body:`.
4. One commit, `Fixes #1056`.

## Done when

The test passes and fails on the unfixed binary (say so under `Tests:`); `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --both-settings --project tests/compiler > out.txt 2>&1` green; the package gate below green; pushed. Files you may change: `crates/nikaia-std/src/tools/check_takes.nika`, its lowered twin `crates/nikaia-std/src/tools/lowered/check_takes.rs`, the new test with its expected output, existing `tests/compiler/tests/*.stderr` whose help line rightly changes.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y20-optional-copy-help.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

