---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: an `if`/`match` argument mixing a lent view and a new value (Fixes #894)

Score: bugs lane (`docs/priorities.md`, bugs-first default): `nikaia check` passes, rustc refuses; found by the #892 builder.

## Issue text (#894)
## Probe
```nika
struct P { name: String }
fn look(p: ref P) -> i64 { return p.name.len() }
fn pick(t: ref P, c: bool) -> i64 { return look(if c { t } else { P { name: "bb" } }) }
fn main() {
    let t = P { name: "a" }
    println(pick(t, true))
    println(pick(t, false))
}
```
`nikaia check` passes; `nikaia run`: rustc *Mismatched types* at the `P { .. }` arm, *Consider borrowing here*. Lowered: `look(if c { t } else { P { name: "bb" } })` - the view arm `t` is `&P`, the new arm is `P`. The same with `fn keep(p: P) -> String { return p.name }` (inferred lent, lowered `&P`). Expected: `1`, then `2`.
Runs already: the same `if` with an owned local `t` (no `ref` parameter) passed to `keep`.

## Locations
* `crates/nikaia-std/src/tools/emit_core_args.nika:291` `emit_core_argument` (a lent argument's `&`), `emit_arg_lent_params` :692.
* Rules: ADR-282 D13 (a choice of view or copied arm). Edition 2024: a `&P { .. }` temporary in an arm's tail dies at the arm's end, so `if c { t } else { &P { .. } }` alone is not enough as an argument; hoist the new value (e.g. `let __arm; ... else { __arm = P { .. }; &__arm }`), or another form rustc accepts.

## To build
1. A lent argument (declared `ref` or inferred lent) given an `if`/`match` whose arms mix a view and an owned value is lowered so every arm is a view of something that lives through the call. Test: `tests/compiler/tests/a_lent_argument_from_arms_view_and_new.nika` + `.stdout`, forms `if` and `match`, declared `ref` and inferred lent, a struct and `String`.

## Done when
The new test; `nikaia test tests/compiler --both-settings`; `it` filtered on `handed_over text_tiers typecheck`; the guard set of `.claude/agents/builder.md`; clippy clean. Commit with `Fixes: #894`; `Changelog:` one sentence.

## Stop and report if
No form rustc accepts keeps every arm a view without a copy (then `blocked` with the forms tried); an open design choice; an error in another agent's file.

Read `docs/builder-facts.md`.

* Not yours (another lane's uncommitted files in the checkout, 2026-10-27): `crates/nikaia-std/src/tools/emit_core_calls.nika`, its `lowered/emit_core_calls.rs`, `crates/rs2nika/convert/**`. Never stage them; if `lower-std` rewrites `lowered/emit_core_calls.rs`, leave it out of your commit and say so in `Body:`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b894-lent-argument-arms.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
