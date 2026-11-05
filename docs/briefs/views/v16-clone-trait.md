---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v16: `Clone` is a trait of the language (#1245)

Score: the language topics (owner, 2026-11-03). Read issue #1245 whole (the supervisor decision is there) and the report `v14-clone-not-keep.md.report.md` beside this brief. Code that is Rust is changed in Rust (#1214); where it is Nikaia, in Nikaia. v15 (`Self` of a bound's method, #1244) runs before this brief; build on it.

Probe (should print `3` and `x`; today NK1135 / NK1126):
```nika
struct P {
    s: String,
}

fn id[T: Clone](x: T) -> T {
    return x.clone()
}

fn main() {
    let a = id(3)
    let p = id(P { s: "x" })
    println(f"{a}")
    println(p.s)
}
```

Owns: Part I 4.7's paragraph on traits of the language (add `Clone` only; follow `docs/writing-guide.md`), the `NK1126` help text, the checker's bound resolution for `Clone`, the lowering's `T: Clone`, their tests, the spec baselines. Not yours: `impl` rules of other traits.

## To build
1. **Specification and checker**: `Clone` as #1245 says; the probe prints as said; a type that does not copy (find which ones the language has, a task handle or a lock; if every type copies, say so in the report and leave the refusal out) passed for `[T: Clone]` is `NK1164`; the `NK1126` help names the bound. Tests in `tests/compiler/tests/`. Commit `Part of #1245`.
2. **Lowering and baselines**: the Rust written for `[T: Clone]` carries `T: Clone`; regenerate `tests/specification/EXPECTED.txt` and `COMPILES.txt` if the spec block changes them (how: `docs/builder-facts.md`). Commit `Fixes #1245`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v16-clone-trait.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
