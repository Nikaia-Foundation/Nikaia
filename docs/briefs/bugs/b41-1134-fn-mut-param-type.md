---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# b41: `fn(mut S)` in a function type (#1134)

The owner chose option A on #1134; Part I says it since 67614ebd (a function type keeps a parameter's `mut`: the argument is lent as to a named function's `mut` parameter). Build it. Read #1134 (probe and *To build*) first.

## Probe (today: a parse error at `fn(mut`)

```nika
struct S { pub n: i64 }
impl S {
    fn apply(ref mut self, f: fn(mut S)) {
        f(self)
        self.n += 1
    }
}
fn bump(mut s: S) { s.n += 10 }
fn main() {
    let mut s = S { n: 1 }
    s.apply(bump)
    s.apply(fn(mut t) { t.n += 100 })
    println(f"{s.n}")
}
```
Expected output: `112`.

## Steps

1. Parser and types: `mut` before a parameter type in a function type. A named `fn f(mut s: S)` and a lambda `fn(mut t) { … }` fit `fn(mut S)`; `fn(S)` and `fn(mut S)` are different types (a function without the `mut` does not fit and the refusal says which parameter).
2. Checker and lowering: the argument at the call is lent as `mut`, the same rule as a `mut` parameter of a named function; lowered as `impl FnMut(&mut S)` for an immediate parameter, `Box<dyn FnMut(&mut S)>` for a kept one. A test `tests/compiler/fn_type_mut_param.nika` with `.stdout` (the probe, `112`) and one refusal with `.stderr`. Commit with `Fixes #1134`.

Where the change sits in a `tools/*.nika` file, change the `.nika` and re-lower; new Rust only where the existing Rust code cannot be converted (CLAUDE.md, *Written in Nikaia only*), else stop and report `blocked`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b41-1134-fn-mut-param-type.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
