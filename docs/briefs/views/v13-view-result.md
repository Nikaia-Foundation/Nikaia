---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# v13: a parameter handed back is a view of any type, not only text (#1239)

Score: the language topics (owner, 2026-11-03). Read issue #1239 whole and the report `v12-keep-view-copy.md.report.md` beside this brief (its `Deviation:` and `Found:` (1) are this brief). Code that is Rust is changed in Rust (#1214); where it is Nikaia, in Nikaia.

Supervisor decision on v12's `Needs:`: this is not an open question. ADR-views-3 D12 ("Returning a parameter hands back a view of it; it does not keep it") and D8 (every view a result holds points into the parameters its ways out reach) decide it for every type, and Part I, 6.6 says the same ("the result itself, or each view in a field, an element or a variant's value"). Text is only the case that is built. So a non-text result that hands back a lent parameter, or an element or field of one, is a view and lowers as one: `fn hand[T](x: T) -> T` becomes `fn hand<'a, T>(x: &'a T) -> &'a T`, with the lifetime tied to the result's sources the way the text path ties `&'a str`. On the caller's side, D9 and ADR-283 D23 (the anchor) decide what a caller may do while the view lives; a temporary argument whose view outlives its statement is held in the caller's frame the way the text path holds it. Use the text path's machinery; do not build a second one.

Probe 1 (#1239, `nikaia run`, gives NK2302 "`hand` keeps `x`", should print `3`):
```nika
struct Box[T] {
    item: T,
}

impl Box[T] {
    fn get(keep self) -> T {
        return self.item
    }
}

fn hand[T](x: T) -> T {
    return x
}

fn main() {
    let b = Box { item: 3 }
    let y = hand(b.get())
    println(f"{y}")
}
```

Probe 2 (a file without `keep`; reaches rustc today as "cannot move out of a shared reference", should print `[1, 2]` and then `2`):
```nika
fn first(x: Vec[Vec[i64]]) -> Vec[i64] {
    return x[0]
}

fn main() {
    let xs = [[1, 2], [3]]
    let f = first(xs)
    println(f"{f}")
    println(f"{xs.len()}")
}
```

Owns: the keep inference, the checker's typing of a call's result as a view, the emitter's view result type, their tests, the spec baselines. Not yours: the use-after-`keep self` check (#1241), the spec page text.


An earlier run of this brief stopped unfinished; its edits (496 lines, never committed, maybe not built) are in `docs/briefs/views/v13-view-result.partial.patch`: read it as a map, not as code that works; apply nothing blind.

## To build
1. **A non-text result handed back from a lent parameter is a view** (D8, D12): the checker types such a call's result as a view with its sources, the inference no longer counts it as keeping, the emitter writes the view result type, callers hold a temporary argument as the text path does. Probes 1 and 2 print as said. Tests in `tests/compiler/`: both probes, and a function that hands back a moving field taken out of its parameter into a place that outlives the call, still refused. Commit `Part of #1213`.
2. **Spec block #56 lowers again**: Part I, 4.6 (`10-nikaia-light.md` #56) compiles; regenerate `tests/specification/EXPECTED.txt` and `COMPILES.txt` (how: `docs/builder-facts.md`). Commit `Fixes #1239`.

List under `Body:` every Rust function added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
both probes print as said, block #56 compiles in the regenerated baselines, the tests pass, the gate is green.

## Stop and report if
the caller's side needs a rule that D9, D12 and ADR-283 D23 do not give (name the program and what it would need), or the text path's machinery cannot carry a non-text view without a new runtime type.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v13-view-result.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
