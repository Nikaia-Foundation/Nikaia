---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# d2: the guide's grammar chapter: writing grammars and building parsers (owner, 2026-11-03)

Score: the owner wants an extended guide chapter on how to write and use grammars and build parsers. `guide/syntax.md` only points at `examples/calc/src/main.nika`. Docs only; no compiler change. S=1, C~3 M.

**Owns:**
* `guide/grammars.md` (new), the *Grammars* section and *Where next* of `guide/syntax.md`, the *Where next* lists of the other `guide/*.md` pages.

**Not yours:**
* `docs/specification/`, the compiler, `std`, examples, and the user's packages `winnow` and `winnow-grammar` (read them, never change them). A form that is specified but does not work goes under `Found:` with a probe.

## To build
1. `guide/grammars.md`, part one, from Part II ch. 10 (10.1-10.8, `docs/specification/20-nikaia-advance.md`), ADR-grammar-1 and -2 and the examples: what a `grammar` is (scannerless: one description for tokens and structure); a first grammar that parses a small format and the result it gives; rules, alternatives, repetition, optional parts, literals and character classes; how the parser's result maps to Nikaia types; using the generated parser from `main`, and what a parse error looks like and how a program reports it. Teach from small to large; every runnable block compiles and runs with `target/fast/nikaia` as shown (check each; paste the real output). Sections end with a **Specification:** link like `syntax.md`. Commit with a plain message (no issue).
2. Part two, building a real parser: walk through a complete one step by step (the calculator of `examples/calc` or a small configuration or JSON-like format), then the advanced features of 10.6 and the vocabulary of 10.8 (precedence, left recursion as specified, error recovery, positions/spans for messages) that work today; what does not work yet is named as such, with the issue number if one exists (grep the specification and `docs/`), never shown as runnable. `syntax.md`'s *Grammars* section links the chapter. Plain commit message.

Write by `docs/writing-guide.md`: lead with safety, no absolute claims, natural prose without bold lead-ins, nothing shown as runnable before it is built.

## Done when
Both steps committed; every runnable block checked with the compiler; links resolve; `nikaia run scripts/pre-push.nika` clean. No changelog line (docs only).

## Stop and report if
a feature the chapter needs for a complete parser does not work although specified (report it under `Found:` with the probe and leave it out).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/docs/d2-grammar-guide.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
