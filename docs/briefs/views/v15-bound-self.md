---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v15: `Self` of a bound's method is the type parameter (#1244)

Score: the language topics (owner, 2026-11-03). Read issue #1244 whole and the report `v14-clone-not-keep.md.report.md` beside this brief (its `Found:` is this brief). Code that is Rust is changed in Rust (#1214); where it is Nikaia, in Nikaia.

Supervisor decision: a defect against Part I, 4.7 ("the bound answers the whole call: … what it hands back"). In a body bound by `[T: Tr]`, a call of a method of `Tr` reads `Self` in its signature as `T`.

Probe (should print `3`; today NK1104 "returning `Self`, but declared to return `T`"):
```nika
trait Twin {
    fn twin(ref self) -> Self
}

struct N {
    v: i64,
}

impl Twin for N {
    fn twin(ref self) -> Self {
        return N { v: self.v }
    }
}

fn id[T: Twin](x: T) -> T {
    return x.twin()
}

fn main() {
    let n = id(N { v: 3 })
    println(f"{n.v}")
}
```

Owns: the checker's typing of a call through a bound, its tests. Not yours: a `Clone` trait (#1245, v16).

## To build
1. **`Self` in a bound's method is the type parameter**, in the result and in the parameters: the probe prints `3`; a second test with `fn same(ref self, other: Self) -> bool` called as `x.same(y)` on two `T`. Tests in `tests/compiler/tests/`. Commit `Fixes #1244`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v15-bound-self.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
