---
agent: builder
model: sonnet
max_turns: 50
budget_usd: 8
---
# Brief: BT2 - the build-time default read moves into Nikaia (BT step 5, Part #125)

Score: self-hosting lane, row BT of `docs/selfhosting-plan.md`: the one step BT left (`Result: partial`), now that its blocker #1055 is fixed (4de39626, b9). Small: one function body, one forward.

Facts: Moving Rust into Nikaia, Where things live, Traps already met, Commands

Cut: crates/nikaia/src/comptime_run.rs:642-660
Convert: --profile check-free --prefix bt_
Shape: faithful

## Pre-flight (the blocker's fix, before anything else)
`tests/compiler/tests/a_field_of_a_coalesce_is_read_through_a_view.nika` (b9's test) has `row_c`, the exact chain this step writes. Run it: `cd tests/compiler && ../../target/fast/nikaia run tests/a_field_of_a_coalesce_is_read_through_a_view.nika > $SCRATCH/out.txt 2>&1; diff $SCRATCH/out.txt tests/a_field_of_a_coalesce_is_read_through_a_view.stdout` (after `cargo build --profile fast -p nikaia` if `target/fast/nikaia` is older than 4de39626). A diff or a rustc error: stop, `Result: blocked`, `Found:` with the output.

## What moves
`known_default`'s body (`comptime_run.rs:642`, the `KNOWN.with` closure) becomes, in `crates/nikaia-std/src/tools/build_eval.nika` beside `bt_may_not_run` (:589):

    pub fn bt_known_default(ledger: ref Ledger, package: String?, key: ref String, at: i64) -> String? sync

The key is `f"{package}::{key}"` where a package is given, else `key`; then `let text = (((ledger.functions.get(k) ?? return null).signature ?? return null).config.get(at) ?? return null).default` (the chain b9 fixed), and `text` where it is not empty, else null. `Ledger`, `Signature`, `ConfigContract` are `ty.nika` (:1233, `FnContract.signature` :1086, `config` :992); `build_eval.nika` already reads `FnContract`.

The Rust `known_default` keeps its signature and `KNOWN` (a `thread_local`, plan *Stays Rust*), and becomes one forward inside `KNOWN.with`: `held.borrow().as_ref().and_then(|ledger| bt_known_default(ledger, package.map(str::to_string), &key.to_string(), at as i64))` (or what the lowered signature asks; `crate::contracts::Ledger` **is** the lowered `ty::Ledger`, `contracts/mod.rs:55`). Caller `contracts/mod.rs:1217` (`evaluate_defaults`) unchanged.

## To build
1. The Nikaia function; `target/fast/nikaia lower-std`; the fixpoint changes only `lowered/build_eval.rs` (and its index line in `lowered.rs` if any). Check it first: `cargo check --profile fast -p nikaia-std --features compiler --message-format short`; a rustc error in `lowered/build_eval.rs` is a lowering defect: stop, `Found:` with the lowered line and a probe.
2. The Rust forward; delete the moved body. `cargo build --profile fast -p nikaia`.
3. Tests: `cargo test --profile fast -p nikaia --test it -- comptime` and `-- build_time` (singly; the defaults read is #468's `comptime` path). No new test for a faithful move; no text a test compares may change.
4. Row BT of `docs/selfhosting-plan.md`: step 5 built (drop "except step 5" / the #1055 wait), the share as `scripts/self_hosting.py` counts it. Commit (`Part: #125`).

## Done when
`known_default`'s read is Nikaia, the Rust left is the `KNOWN.with` forward, the tests pass, committed. Commit subject: `build time: the default read moves into Nikaia (BT step 5)`. Changelog: none (a move).

## Stop and report if
A text a test compares would change; a missing form (CLAUDE.md, *A missing form stops the work*); an edit needed outside `comptime_run.rs`, `build_eval.nika`, its lowered twin, `lowered.rs`'s index line, the plan row; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (today among them `crates/nikaia-std/src/tools/lowered/project_members.rs`, `crates/rs2nika/convert/*`, `crates/rs2nika/facts/*`, `docs/briefs/*.report.md`), and every `docs/briefs/*` file except your report. If `lowered.rs` or `lowered/build_eval.rs` shows another session's hunk, stage only yours (builder-facts trap).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bt2-known-default.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
