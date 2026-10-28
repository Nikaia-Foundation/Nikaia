---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: P2, the refusal summary of `check` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, largest Rust files first; #125, `docs/selfhosting-plan.md` *Large files: project.rs and parser/mod.rs*, row P2, the first slice it names). S=2 (~420 Rust lines, converter draft 100%), C~3-5 M (comparable: e12-4a).

## Facts
* `crates/nikaia/src/project.rs` `pub fn check` (:1894). After the analyses it calls `lint_where_nothing_crosses` (:2293), `let_refuted_claims_through` (:2262), `let_unproved_claims_through` (:2279) on `&mut all` (each: a switch word, then for every finding of one code set `severity = Warning` and push one note), then `sync::check`, renders, keeps the errors (`findings: Vec<&check::Finding>`, :1975) and builds the tally (:1989-2233): `count(family)` by code prefix, `plural(n)`, one `refused.push(format!(..))` per family, the `others` remainder, the `may_not` line, then `Err(diagnostics::refuse(refused.join(", ")))`.
* `check::Finding` / `check::Severity` are `CheckFinding` / `CheckSeverity` of `crates/nikaia-std/src/tools/check_state.nika` (:387, :412), re-exported in `crates/nikaia/src/check/mod.rs:45`. Not `findings.nika`'s `Finding` (the plan's row says so wrongly: use `CheckFinding`).
* `write_proofs` (:2385) and `write_ledger` (:2432): the `--locked` refusal texts (:2416, :2440, :2447). Only the texts move (functions taking the path as `ref String`); the file I/O stays Rust.
* The tally text is byte for byte in `.stderr` files of `tests/compiler` and `tests/` (about 30): any change of wording is a failure, not a choice.

## To build
1. New `crates/nikaia-std/src/tools/project_check.nika`: `project_check_summary(findings: ref Vec[CheckFinding], violations: i64) -> String` (counts only `severity == Error`, returns the joined tally; `check` keeps `Err(diagnostics::refuse(..))` and the early return when there is nothing), the three filters as `project_check_lint_where_nothing_crosses(findings: mut Vec[CheckFinding], user_parallelism: ref String)` and the two `let_*_through` (an index loop: the `iter_mut().filter` has no Nikaia form; `count` and `plural` become functions), and the `--locked` texts as functions returning `String`. Keep the doc comments (they carry the ADR references). Run the converter first on a file holding exactly the moved Rust (`rs2nika --profile check --file`) and report `--score` under `Rs2nika:`.
2. `project.rs` calls the lowered functions; the moved Rust is deleted. `lower-std`: only `project_check`'s lowered file and the index line in `lowered.rs` change. Add the row to `scripts/self_hosting.py` (as `diffs.nika`'s) and mark P2 built in the plan's table (one line); report the % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`); if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia test tests/compiler --both-settings`; `it` tests filtered on `refus claim locked parallelism sync`; the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. Commit `project: the refusal summary of check into Nikaia (P2)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice; a `.stderr` that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p2-project-check-summary.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
