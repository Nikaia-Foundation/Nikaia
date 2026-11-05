---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# v9: the `ref` migration tool, run over `guide/`, `examples/` and `README.md` (#1213 step 6)

Score: the language topics (owner, 2026-11-03), ADR-views-3 is the record, D4 above all (`ref` leaves the language outside `extern "C"`). Read issue #1213 whole, step 6 is yours in part. The tool is Nikaia (`crates/nikaia-std/src/tools/*.nika`, lowered by `lower-std`); the subcommand's Rust shell is allowed (#1214), kept as thin as `crates/nikaia/src/fmt.rs`, which is the model: a Rust door that calls a lowered tool.

Surface today: `ref` is still written about 6800 times in std's `.nika` files and in `guide/`, `README.md` and the examples (`ref self`, `ref mut self`, `x: ref T`, `return ref s`). The checker already treats a plain parameter as lent and checks `keep` (v3), `from` on results and function types (v4, v6). NK1261 warns that a `ref` is redundant.

Facts:
* The rewrites (D4): `ref self` -> `self`, `ref mut self` -> `mut self`, a parameter `x: ref T` -> `x: T`, `x: ref mut T` -> `mut x: T`, `return ref s` -> `return s`, `ref` before a type in a field, element or binding is dropped (#1115: before a tuple too). Inside `extern "C"` nothing changes.
* `keep`: a parameter the ledger's `keeps` names and that is not yet marked gets `keep` (D1); the tool reads the same inference the checker uses (`keeps.nika`), not a second one.
* Comments, strings and text that is not Nikaia code are left alone; in Markdown only the ` ```nikaia ` blocks are rewritten.
* A test file inside the project runs with `nikaia test tests/compiler --filter <name>`, not `nikaia run`.

**Owns:** the new tool, its subcommand, its tests, and the rewritten `guide/`, `examples/` and `README.md`.

**Not yours:** std, the packages and `tests/` (later briefs run the tool there), the checker, the ledger, NK1261 (it retires once std is migrated).

## To build
1. **The tool**: `nikaia migrate views-3 <path>...` rewrites `.nika` files and the Nikaia blocks of `.md` files in place, by the rewrites above, and prints one line per changed file. Tests in `tests/compiler/tests/migrate_views3_*`: each rewrite, an `extern "C"` block left alone, a `keep` written from the ledger, a comment and a string left alone; a second run changes nothing. Commit `Part of #1213`.
2. **Run it** over `guide/`, `examples/` and `README.md`; every example and guide test that ran before still runs (the tests that check `guide/` and the examples, whatever `docs/builder-facts.md` names). Commit `Part of #1213`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
`grep -rnw ref guide examples README.md` finds only `extern "C"` code and prose about it; the tool's tests pass; the guide and example tests pass.

## Stop and report if
a rewritten program no longer checks because the checker still needs the `ref` (that is a gap in steps 2-4, not something to write around), or writing `keep` needs a ledger the tool cannot reach for a single file.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v9-ref-migration.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
