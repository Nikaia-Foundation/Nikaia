---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v18a: a lent parameter passed on to a method's parameter; a `ref` pattern over a lent subject (#1213, split from v18)

Score: ADR-views-3 (D1: a plain parameter is lent, D4: `ref` leaves the language outside `extern`). Read `v18-matched-lent-param.debrief.md` beside this brief: v18 changed the keep rule of a matched parameter, and the fixpoint then lowered `lowered/` into Rust with 19 rustc errors in three classes. This brief fixes the two classes that are defects on their own, before the keep rule (v18b) comes back.

Facts: Checker and views, Lowering and lower-std, Tests and the gate

**(1) Probe** (fails in rustc on today's binary too: `both` lowers to `fn both(state: &mut State, op: &Op)` and calls `state.note(op)`, while `note` takes `Op` by value). Must check, run and print `true` then `1`:
```nika
enum Op {
    Add,
    Sub,
}

struct State {
    count: i64,
}

impl State {
    fn note(ref mut self, op: Op) {
        if op == Op::Add {
            self.count = self.count + 1
        }
    }
}

fn adds(op: Op) -> bool {
    return op == Op::Add
}

fn both(mut state: State, op: Op) -> bool {
    state.note(op)
    return adds(op)
}

fn main() {
    let mut s = State { count: 0 }
    println(both(s, Op::Add))
    println(s.count)
}
```
Cause (v18): `lends_with` never lends a method parameter, but `kept_by` takes the method's column as "not kept". The two must agree: a method parameter that is not kept is lent, as a free function's is.

**(2) Class**: `Some(ref x)` (a `ref` binding in the lowered pattern) against a subject that is already a borrow (`&Option[...]`, a lent parameter) is a rustc error. Cause: `check_core_flow.nika` `by_reference`. Build a probe of a few lines first (a lent `Option[String]` parameter matched with `Some(x) => x.len()`), and pin it as a test. A lent subject binds by view without `ref`.

**Owns:** the checker and lowering files the two rules need (`keeps.nika`'s `lends_with`/`kept_by` side, `check_core_flow.nika`), the lowered files `lower-std` changes, the new tests in `tests/compiler/`.

**Not yours:** the keep rule of a matched parameter (`Expr::Match` in `keeps.nika`: v18b), the migration tool (v19), examples.

## To build
1. **(1)**: the method-parameter rule, with the probe as a `tests/compiler/` test. Before any build: one `lower-std`, then `git diff --stat crates/nikaia-std/src/tools/lowered/`. Over about 5 changed files, sort what changed and report before rebuilding. Commit `Part of #1213`.
2. **(2)**: the `ref` pattern over a lent subject, with its probe as a test; the same `lower-std` count first. Commit `Part of #1213`.

`Rs2nika: none`.

## Done when
Both probes check and run; the touched tests pass; the fixpoint holds.

## Stop and report if
a rule change flips more than about 5 lowered signatures, or makes rustc refuse lowered code in a class this brief does not name: stop with `partial`, the classes and a probe for each.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v18a-method-param-lent.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
