---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: `??` on a part of a lent tuple parameter moves it (Fixes #891)

Score: bugs lane (`docs/priorities.md`, bugs-first default; #891 is a checker-passes/rustc-fails bug, found by the #887 builder; probe reproduced on 2026-10-27 develop d25c5b9).

## Issue text (#891)
```nika
fn f(p: (String?, i64)) -> String { p.0 ?? "none" }
fn main() { println(f(("xy", 1))); println(f((null, 2))) }
```
The checker passes this program. rustc refuses it: "cannot move out of `p.0` which is behind a shared reference". The parameter `p` is lent (a view), and `??` on its part moves the `Option<String>` out. The same `??` on a lent `String?` parameter works, and so does a struct field of a lent struct parameter (`fn g(s: S) -> String { s.name ?? ".." }` prints `a`, probed): only the tuple part misses the lowering the field gets.

## Locations
`emit_core_coalesce` `crates/nikaia-std/src/tools/emit_core_flow.nika:593` (the `??` lowering); how it recognises a lent place: compare the `Expr::Field` path with `Expr::TupleIndex` (or whatever the tuple part's node is) in `emit_core_places.nika` and `check_boundary.nika:261` `a_place_of_a_lent_parameter` / `:288` `place_path`. Other names: `nikaia run scripts/where.nika -- NAME...`.

## To build
1. Probes that fail on today's develop: the program above; `let x = p.0 ?? "none"`; `p.0 ?? "none"` as a call argument; a nested `(i64, (String?, i64))` part `p.1.0 ?? ".."`. Keep the struct-field form as a control (already passes).
2. Emit (and the checker fact, if the lowering asks it): `??` on a tuple part of a lent parameter is lowered as on a struct field of one (no move out of the view). Fix at the place where the field case is recognised, so both share one path.
3. A test `tests/compiler/tests/coalesce_on_a_lent_tuple_part.nika` with `.stdout`, holding all forms of step 1 and the struct control. Last commit `Fixes #891`.

## Done when
The new test; `nikaia test tests/compiler --both-settings`; the guard set of `.claude/agents/builder.md`; `lower-std` unchanged or changed only where a tools file used the form (say which under `Tests:`). Changelog: one line (a lent tuple part under `??` no longer fails in rustc).

## Stop and report if
An open design choice; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b891-lent-tuple-part-coalesce.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
