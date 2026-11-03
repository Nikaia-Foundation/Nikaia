---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# p1199a: `immutable`, the binding and the deep rule (#1199, ADR-389 D1, D2, D6)

Build steps 1-3 and the function-level half of step 6 of #1199. Read
`docs/specification/adr/adr-389.md` first; Part I 2.1 and Part III C.3
(`NK1331`) carry the specified text. `NK1331` is reserved for this work: use
it, take no other code.

Owns: the lexer's reserved list, the parser grammar for the binding (in the
`.nika` source of the parser where the rule lives, with its lowered file), the
checker's rule for `NK1331` D2, function-level code generation, and the tests
`tests/compiler/immutable_*.nika`.

## Steps
1. Lexer: `immutable` on the reserved list (a name `immutable` is now refused
   like any reserved word). Commit.
2. Parser: `immutable x = e` and `immutable x: T = e` inside a function body;
   at item level parse it too (with `pub`), but item level is built in p1199b:
   for now an item-level `immutable` is refused with a plain "not built yet"
   error under an existing code, never silently accepted. `immutable mut` is a
   syntax error. Commit.
3. Checker, D2: a type reaching `SharedMut`, `Locked`, a channel end or another
   shared changeable handle (through fields, elements, variants, captures) is
   refused with `NK1331` at the initializer, naming the path to the first such
   part (`config.cache: SharedMut[Map[String, Int]]`); `Shared[T]` passes when
   `T` passes. A `ref mut self` call on it and a move out of it are refused
   (`NK1331`); reading through views works. Code generation inside a function:
   a plain binding. Commit.
4. Tests in `tests/compiler/` (with `.stdout`/`.stderr`): an accepted binding
   with and without a type, a `Shared` that passes, each refusal of D2 with its
   path, the `ref mut self` call, the move out, `immutable mut`, the reserved
   name.

A missing form or a defect met on the way: CLAUDE.md, *Moving Rust into
Nikaia* (stop and report a gap; fix a defect with its own test and commit).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p1199a-immutable-binding.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
