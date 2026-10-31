---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: `process::run` with inherited streams and a stdin (#920 steps 1-3, Part #125; unblocks K3b, P5c, O2)

Score: self-hosting lane. #920 is the one `std` gap K3b, P5c and O2 (`docs/selfhosting-plan.md`) wait on: Cargo's build output reaches the user as it happens, Cargo's JSON comes back on stdout while its progress goes to the terminal, and test binaries and comptime runs read a stdin. The forms are decided in #920 (architect, 2026-10-31): options after `;`, ADR-243 style, each paying only where it is written (default keeps today's behaviour byte for byte).

Facts: Where things live, Traps already met, Commands

## Decided (architect, 2026-10-31)
* `stdout: process::Stream = process::Stream::Captured`, `stderr: process::Stream = process::Stream::Captured`, `enum Stream { Captured, Inherited }`; an inherited stream's text in `Output` is `""`.
* `stdin: process::Input = process::Input::Closed`, `enum Input { Closed, Inherited, Text(String) }`; `Text` is written by a thread of its own while the pipes are read (no deadlock on a child that writes before it reads), then closed. A child that exits before reading all of it is no failure (a broken pipe on that write is ignored).
* Order of the options after `;`: the existing ones unchanged, then `stdin`, `stdout`, `stderr` (append; Part III says the order).
* `within:` keeps working with inherited streams (the process group kill does not depend on the pipes).
* Part III paragraph beside the environment options (`30-nikaia-tooling.md` :1526ff), an amendment section in `docs/specification/adr/adr-243.md` (D10, citing #920), `std.contracts` entries beside `process::run`'s. No version line, no CHANGELOG (the supervisor's): say under `Changelog:` what it decided.

## Locations
`crates/nikaia-std/src/process.rs` (`run` :160ff, `Command` built at :86-102: `.stdin(Stdio::null())`, `.stdout/.stderr(Stdio::piped())`); `crates/nikaia-std/std.contracts` (`grep -n "process::" `); how an option after `;` reaches the Rust fn: follow `env_clear` (`nikaia run scripts/where.nika -- env_clear`), the pattern to copy for every layer it passes (contracts, emitter call, std signature).

## To build
1. `Stream` and `stdout:`/`stderr:`, spec, ADR amendment, contracts.
2. `Input` and `stdin:`.
3. Test `tests/compiler/tests/process_inherit.nika` (+ `.stdout`, `.stderr` if needed): a child (`sh -c`, as the existing `process_*` tests start one: `ls tests/compiler/tests/process_*`) that prints with `stdout: Inherited` (its line appears in the test's own stdout, and `Output.stdout` is `""`), one with stdout captured and stderr inherited, one that reads a `stdin: process::Input::Text("…")` and echoes it, one with the defaults (unchanged).
Commit per step; the last `std: process::run takes stdin, stdout and stderr options (Fixes #920)`.

## Done when
The new test passes with `nikaia test --project tests/compiler --both-settings` on it (singly: `gate_test_some`), the existing `tests/compiler/tests/process_*` tests pass, `cargo test --profile fast -p nikaia --test it -- process` passes, clippy (also `cargo +1.99.1`, builder-facts *Traps already met*), fmt, `pre-push-check.sh`. Stop about 15 turns before `max_turns`, commit what is finished, `Result: partial`.

## Stop and report if
An option form the existing `;` options cannot carry (a defaulted enum value, `process::Input::Text(..)` as an argument) fails in check or lowering: that is a `Found:` with a probe, not code around it; an edit needed outside `process.rs`, `std.contracts`, Part III, ADR-243, the files the option path passes (name them in `Body:`) and the test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (today among them `crates/nikaia-std/src/tools/lowered/project_members.rs`, `crates/rs2nika/convert/*`, `crates/rs2nika/facts/*`, `docs/briefs/*.report.md`), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s920-process-streams.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
