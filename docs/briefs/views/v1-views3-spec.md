---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# v1: the specification of ADR-views-3 (#1213 step 1)

Score: the focus moves to the language topics (owner, 2026-11-03). #1213 carries the whole topic decided by the owner (option D): `keep` on parameters, `ref` leaving the language outside `extern "C"`, inferred views, and a result's source inferred or written `-> T from a | b`. The record is `docs/specification/adr/adr-views-3.md` (D1-D10, pushed); this brief writes it into the specification. Text only; no compiler change. S=1, C~4 M.

**Owns:**
* `docs/specification/10-nikaia-light.md`, `20-nikaia-advance.md`, `30-nikaia-tooling.md` (the parts views-3 touches), the ADRs named below, `docs/specification/adr/README.md` (index rows), the moves into `docs/history/adr/`, and every citation those moves break (`grep -rn "ADR-202\|ADR-353\|ADR-354"` over `docs/`, `guide/`, `crates/`, `tests/`, `scripts/`).

**Not yours:**
* Compiler, `std`, packages, examples and `guide/` code (the migration tool, step 6 of #1213, rewrites code later; a code sample in the specification is changed, one in `guide/` is not).
* `adr-views-3.md` itself: it is the record; a contradiction you find in it is reported under `Found:`, not fixed.
* The async ADRs and Part I 8.x async text (another lane).

## To build
1. Part I ch. 6 (6.5, 6.6, 6.7 and every rule they feed) and the Part III text say what ADR-views-3 decides: `keep` on parameters and receivers (`self`, `mut self`, `keep self`), `mut` on a changed parameter, `ref` refused outside `extern "C"`, inferred views in fields, elements and local bindings, a result's source inferred or written `from s` / `from a | b` (required on trait methods, function types and foreign functions), the caller rule of D9 and the ledger spelling of D10. Every `ref` / `ref mut` outside `extern "C"` leaves the specification's code and rules, including the `fold` step of Part II 10.x, which becomes `fn(mut A, T)` / `fn(mut A, A)`, and the same wording in ADR-372 D9 and 10.7. Error codes: name only existing ones; where a rule needs a code that does not exist, write `NK????` and list it under `Needs:` (the supervisor assigns codes). Commit `Part of #1213`.
2. As one commit with every citation moved: move ADR-202, 353 and 354 to `docs/history/adr/` (ADR-decision-records-1 D3; views-3 restates what of them still holds), and rewrite ADR-094 D2 and D6, ADR-184 D1, ADR-251 D4 and ADR-358 D3 in place under their *Changed by* notes. Index rows in `adr/README.md` follow. Commit `Part of #1213`.

Write by `docs/writing-guide.md`, last section (compact, one rule in one place, one meaning per word). A rule views-3 leaves open (its section 6) stays open: no new decision in the text; report it under `Found:`.

## Done when
No `ref` outside `extern "C"` in the specification's rules and code; `nikaia run scripts/pre-push.nika` (ADR citations) clean; `tests/specification` (the spec's code blocks, `COMPILES.txt`) gives the same counts as before, a block that now uses `keep` or `from` and does not yet compile listed in `COMPILES.txt` as such. No changelog line.

## Stop and report if
views-3 contradicts itself or another live ADR on a point the text must state; a citation cannot be moved without changing what the citing text says.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v1-views3-spec.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
