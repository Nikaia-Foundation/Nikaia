---
agent: builder
model: opus
max_turns: 100
budget_usd: 16
effort: medium
---

# v18a: a lent parameter passed on to a method's parameter; the `??` fallback kept (#1213, split from v18)

Score: ADR-views-3 (D1: a plain parameter is lent, D4: `ref` leaves the language outside `extern`). Read `v18-matched-lent-param.debrief.md` beside this brief: v18 changed the keep rule of a matched parameter, and the fixpoint then lowered `lowered/` into Rust with 19 rustc errors in three classes. This brief fixes the method-parameter class, narrowed after a first run (below).

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


## First run (2026-11-05, ran out of turns, nothing committed)

Read `v18a-method-param-lent.debrief.md` and the draft report `v18a-method-param-lent.first-run.report.md` beside this brief. Its edits are saved in `v18a-method-param-lent.partial.patch` (lends.nika, keeps.nika, four `&` fixes in Rust callers, two tests, 40 lowered files); the tree is back at HEAD. Applied whole, the rule flipped about 60 tools signatures, and 15 `it` tests failed in two classes: (a) a method parameter whose type holds views and that is stored into `self` lowered to `&Reading<'_>` and rustc asked for a lifetime (`659_field_map`, 1brc); (b) a method used as a function value (`Summary::joined` handed to `fold_merged`, `in_place`) took `&Summary` where `fn(Summary, Summary)` is expected (E0631). Class (2) of the old brief cannot be reached before v18b: it moved there.

## To build
1. **The `??` fallback** (a defect of its own, from the patch): `keeps_handed` in `keeps.nika` follows the fallback of `??`, so `store(first ?? other)` keeps `other`. The test `a_coalesce_fallback_handed_to_a_keeper_is_kept` (its `.nika` and `.stderr` are in the patch). Commit `Part of #1213`.
2. **(1), narrowed**: an own method's parameter that `kept_by` finds not kept is lent, except (a) one whose type holds a view, and (b) every parameter of a method that is used as a function value anywhere in the program (that signature stays by value). Take what fits from the patch's `lends_with`. Before any build: one `lower-std`, then `git diff --stat crates/nikaia-std/src/tools/lowered/`, then `cargo check -p nikaia-std --features compiler`; then the `it` tests `accepted_compiles`, `parallel_folds` and `stored_views`. The Rust callers of a now-lent lowered method take a `&` (the patch has four). The probe (1) above as a test (`a_lent_parameter_handed_to_a_method_stays_lent`, in the patch). Commit `Part of #1213`.

`Rs2nika: none`.

## Done when
Probe (1) checks and runs; the touched tests and the three `it` tests pass; the fixpoint holds.

## Stop and report if
any rustc error class outside this brief shows after the first fixpoint: stop with `partial`, the classes and a probe for each. The number of flipped signatures alone is no reason to stop.

**Owns:** the checker and lowering files the two rules need (`keeps.nika`, the Rust callers of a lent lowered method), the lowered files `lower-std` changes, the new tests in `tests/compiler/`.

**Not yours:** the keep rule of a matched parameter (`Expr::Match` in `keeps.nika`: v18b), the `ref` pattern over a lent subject (moved to v18b), the migration tool, examples.

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
