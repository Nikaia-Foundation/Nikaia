---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# d3: the guide on the prover: what it proves today, its limits, the plans (owner, 2026-11-03)

Score: the owner wants the guide to say what the prover can do now, where its limits are and what is planned. `guide/contracts.md` shows contracts and the report but not the prover's reach as a whole. Docs only; no compiler change. S=1, C~3 M.

**Owns:**
* `guide/prover.md` (new), links into it from `guide/contracts.md` and the *Where next* lists of the other `guide/*.md` pages.

**Not yours:**
* `docs/specification/`, the compiler, `std`. A claim of the specification the compiler does not meet goes under `Found:` with a probe.

## To build
1. `guide/prover.md`, *What it proves today*: from the compiler as built (the checker and codegen sources under `crates/nikaia-std/src/tools/` such as `emit_contracts.nika` and the prover/solver files, Part III 14.2 and 13.x on `optimization` = `remove-bounds-checks`, `remove-overflow-checks`, `prove-through-loops`, and `guide/contracts.md`): which claims it decides (preconditions, postconditions, `assert`, index bounds, integer overflow, facts through loops without an invariant, calls between functions), what happens to a claim it proves (the check is left out where enabled) and to one it cannot (a runtime check stays, or an error, per the specification), and how to read the report. Each statement shown by a small program checked with `target/fast/nikaia` (paste the real output or report line). Commit with a plain message.
2. *Where its limits are* and *What is planned*: the kinds of claims it gives up on today (by probe: nonlinear arithmetic, quantifiers over collections, heap/aliasing facts, floats, recursion, whatever the probes show), each with a short program and what the compiler says; then the plans as recorded in ADRs and open issues (grep `docs/specification/adr/` for the prover and contracts ADRs; list issues by number and title only, no dates or deadlines, nothing promised). Link the chapter from `contracts.md`. Plain commit message.

Write by `docs/writing-guide.md`: lead with safety, no absolute claims ("proves every..." is wrong; say what it proves and where it stops), natural prose without bold lead-ins, nothing shown as runnable before it is built.

## Done when
Both steps committed; every statement backed by a checked program; links resolve; `nikaia run scripts/pre-push.nika` clean. No changelog line (docs only).

## Stop and report if
the prover's behaviour contradicts the specification or `contracts.md` on a point the chapter must state (report under `Found:` with a probe).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/docs/d3-prover-guide.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
