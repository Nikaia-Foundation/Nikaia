---
agent: builder
model: opus
max_turns: 70
budget_usd: 12
effort: medium
---

# a6: the guide, README and website teach `async`, no longer `overlap` and `spawn` (owner, 2026-11-04)

Score: the owner asked for the documentation to explain `async` and not `overlap` and `spawn`. ADR-async-5 is the record; read `docs/specification/adr/adr-async-5.md` (D1, D6, D9, D10, D20, D21) and Part I, Chapter 8 first, then `docs/writing-guide.md` whole: you write by it. Docs only: no compiler change, no test change except what the guide test project needs.

Surface today: the pages teach `overlap { … }` and `spawn fn { … }` as what compiles, and describe `async` as "the design being built, not yet accepted by the compiler". a4 (c3c7e170) and a5/a5b built `async e`, `async e` as a statement and `async { … }` groups through the checker and the lowering. Check `git log` for a5b's commits; if a5b's report is not `done`, stop and report `blocked`. Still not built: `async(handle)`, `async(deferred)`, `async(detached)`, `limit:`/`within:`/`admission:`, `par_iter`, and the pool at `user_parallelism: yes` (tasks run one after another there for now).

**Owns:**
* The concurrency passages of `guide/syntax.md` (the program at "both files are read at the same time", the text after it, *What the compiler checks about tasks*, *Data that more than one task changes*), `guide/ownership.md` (*Keeping a value beyond the call*, the task example), `README.md` (the `async` paragraph), and the `website/` pages that teach `overlap` or `spawn`: `easy/async-without-colouring.html` above all, and the `overlap`/`spawn` passages of `index.html`, `easy/index.html`, `safe/index.html`, `safe/conflicting-actions.html`, `safe/unhandled-errors.html`, `good/index.html`, `good/built-in-the-open.html` and `fast/against-tuned-rust.html`.
* The guide test project `tests/guide/` if an example there mirrors a page.

**Not yours:**
* `docs/specification/` and the ADRs, compiler messages (NK2101's note still says "A task started with `spawn`": report it under `Found:`, do not change it), `docs/*.md` other than reading them, `CHANGELOG.md`.

## To build
1. **The guide and README.** Teach `async` as the way to run work beside other work: `async e` starts `e` at once and the program waits for the value where it is first needed; `f(async g(), async h())`; a group `async { … }` that waits for every task in it at its end, also on an early `return`; a failure is thrown where the program waits; tasks that would change the same value at once are refused. Replace the `overlap` program with an `async` one, and the `SharedMut` counter's `spawn`/`join` with a group. Every code block shown as runnable must compile and give the output shown with `/home/user/Nikaia/target/fast/nikaia` (write each to your scratchpad and run it; say under `Tests:` which you ran). Handles, detached tasks, options and `par_iter` are described in prose as specified and not built yet, with a link to #988 / #983, without a runnable block. `overlap` and `spawn` are no longer taught: at most one sentence says older programs use them and that `async` replaces them. *Keeping a value beyond the call* gets an example that does not depend on `spawn` (a task that may outlive its function is `async(detached)`, not built: say so in prose), or keeps its point with a stored value instead of a task. Commit.
2. **The website.** The same change on the pages listed under *Owns*, in the pages' own style; `async-without-colouring.html` leads with a runnable `async` example. Commit.

## Done when
No page in `guide/`, `README.md` or `website/` teaches `overlap` or `spawn` as the way to write concurrency (`grep -rnw "overlap\|spawn" guide README.md website` shows at most the one sentence per page about older programs); every runnable block was run; `nikaia run scripts/pre-push.nika` passes. No changelog line, no version change.

## Stop and report if
an example the pages need does not compile or gives other output than the specification says: that is a defect, report it under `Found:` with the probe, show nothing as runnable that is not, and go on with the rest.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a6-docs-teach-async.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
