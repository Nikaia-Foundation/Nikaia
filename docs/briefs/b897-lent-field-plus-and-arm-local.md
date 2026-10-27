---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: a text `+` of a lent field, an owned local in an arm of a lent argument (Fixes #897)

Score: bugs lane (`docs/priorities.md`, bugs-first default): `nikaia check` passes, rustc refuses; found by the #894 builder.

## Issue text (#897)
Found by the #894 builder (b82ab26). `nikaia check` passes both probes; rustc refuses the lowered code.

## Probe 1: a text `+` of a field read through a lent parameter
```nika
struct P { name: String }
fn look(p: ref P) -> i64 { return p.name.len() }
fn f(t: ref P) -> i64 { return look(P { name: t.name + "!" }) }
fn main() { println(f(P { name: "ab" })) }
```
rustc: *cannot move out of `t.name` which is behind a shared reference*. Lowered `nikaia_std::concat::plus(t.name, "!")`. Expected `3`.

## Probe 2: an owned local named in an arm of a lent argument
```nika
struct P { name: String }
fn look(p: ref P) -> i64 { return p.name.len() }
fn main() {
    let c = true
    let t = P { name: "a" }
    let a = look(if c { t } else { P { name: "bb" } })
    println(a + t.name.len())
}
```
rustc: *borrow of moved value `t`*: the arm moves `t` into the `if`. Passing `t` to a lent parameter lends it (`look(t)` does); an arm of a lent argument must lend it too. Expected `2`.

## Locations
* Probe 1: `crates/nikaia-std/src/tools/emit_core_leaves.nika:213` `emit_core_concatenated` (lends only an `Expr::Index` side as `&str`); the `+` call at `emit_core_flow.nika:399`.
* Probe 2: `crates/nikaia-std/src/tools/check_arm_leaves.nika:153` `lent_arms_view_and_new` (#894: an arm counts as viewed only if it is a view or a lent name; an owned local goes to `made`, `names == 0` returns false); emit through `nikaia_std::arm::Arm` (`crates/nikaia-std/src/arm.rs`), decided in `check_arguments.nika:334`.

## To build
1. A side of a text `+` that is a place the function does not own (a field through a lent parameter or view, at any depth) is lent as `&str`, as an indexed side is. Test `tests/compiler/tests/a_text_plus_of_a_lent_field.nika` + `.stdout` (probe 1, also as a `let` and a `return`).
2. An owned local named in an arm of an argument to a lent parameter is lent through the call (`Arm::View(&t)` beside `Arm::Own(..)`), not moved; the local stays usable after the call. Both arms owned locals: both lent. Test `tests/compiler/tests/an_owned_local_lent_through_arms.nika` + `.stdout` (probe 2, `if` and `match`, struct and `String`).

## Done when
Both tests (each fails on the unfixed compiler); `nikaia test tests/compiler --both-settings`; `it` filtered on `handed_over text_tiers typecheck`; the guard set of `.claude/agents/builder.md`; clippy clean. One commit per step; the last says `Fixes: #897`; `Changelog:` one sentence.

## Stop and report if
Step 2 needs a language decision (an arm of a lent argument that moves is meant to move): then `blocked` with the forms tried, step 1 committed; an error in another agent's file.

Read `docs/builder-facts.md`.

* Not yours (another lane's uncommitted files in the checkout, 2026-10-27): `crates/rs2nika/convert/**`. Never stage them.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b897-lent-field-plus-and-arm-local.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
