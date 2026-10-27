---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 20
---
# Brief: #655 step 5, a view result of a function with two `ref` parameters lowers with its lifetime

Score: bugs lane (`docs/priorities.md`): the checker passes code rustc refuses (a program dies in rustc); last step of #655, so this commit says `Fixes #655`.

## Issue text (#655, item 4, step 5)
A function with two `ref` parameters that returns `ref E?` passes the checker, then rustc reports a missing lifetime. To build 5: lowered with the right lifetime, or refused per ADR-353. Test.
Decided: ADR-353 D1/D2 (`docs/specification/adr/adr-353.md`): each view a result holds has its own sources, which the compiler reads off the body; where the body chooses at run time, the view points into the union. Nobody writes a lifetime (Part I 6.5). So: **lower**, do not refuse: one lifetime `'a` on the parameters the result views (all the sources the body returns from), none on the others.

## Probes (architect ran them on b9aa489's binary, `struct E { n: i64 }`)
* (a) `fn pick(a: ref Vec[E], b: ref Vec[E], c: bool) -> ref E? { if c { return a[0] }; return b[0] }` (one statement a line), main `pick(a, b, false) ?? return` prints `n`: rustc *Missing lifetime specifier*. Expected `2`.
* (b) the same with `-> ref E` (not optional): the same error. Expected `2`.
* (c) `fn first(a: ref Vec[E], b: ref Vec[E]) -> ref E? { return a[0] }`: expected `1`, `b` gets no lifetime (then its warning about unused `b` stays as it is).
* (d) control: one `ref` parameter returning `ref E?` (already fine).

## Locations
* Lifetimes of a signature: `crates/nikaia-std/src/tools/emit_host.nika:114` `split_lifetimes` (trait side), `crates/nikaia/src/emit/host.rs:390`, `crates/nikaia/src/emit/mod.rs:1758`; caller in `crates/nikaia-std/src/tools/emit_function.nika` (find with `nikaia run scripts/where.nika -- split_lifetimes`). The sources a result views: the ledger's view sources (ADR-353 D5), find where `emit_function` reads them.
* Tests: new `tests/compiler/tests/a_view_result_of_two_ref_parameters.nika` + `.stdout` (a-c).

## Done when
(a)-(c) print as stated, (d) unchanged; `nikaia test tests/compiler --both-settings`; `it` filters `lifetime`, `views_kept`, `element_views`, `view_result`. Commit `A view result of two ref parameters is lowered with their lifetime (Fixes #655)`. Changelog line: yes.

## Stop and report if
The sources are not known at the lowering (the ledger does not carry them for a private function): `Result: blocked` with the probe, no refusal built instead.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-two-ref-lifetime.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
