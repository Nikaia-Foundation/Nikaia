---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# v11: keep inference across files, then a bare `self` is lent (#1213)

Score: the language topics (owner, 2026-11-03), ADR-views-3 is the record: D1 "`self` looks, `mut self` changes in place, `keep self` consumes the object", and a plain parameter is lent. Read issue #1213 whole and the report `v10-self-lent.md.report.md` beside this brief (its `Found:` is step 1 here). Code that is Rust is changed in Rust (#1214); where it is Nikaia, in Nikaia.

Supervisor decision on v10's `Needs:`: the cross-file keep inference is a defect (the same program passes with the struct in one file), so it is fixed; a bare `self` of a `Copy` type is **not** given a by-value rule. EmitFlow's `self with {..}` builders take `keep self`, as ADR-118 asks.

Probe 1, a 2-file package (`nikaia check src/main.nika` gives NK2302 "`pf` keeps `ty`"; with `Type` in main.nika it passes):
```nika
// src/types.nika
pub struct Type { pub name: String, pub is_slice: bool, pub is_mut: bool }
```
```nika
// src/main.nika
pub enum EP { Run { mutable: bool } }
pub fn pf(ty: ref Type) -> EP? {
    if ty.is_slice { return EP::Run { mutable: ty.is_mut } }
    return null
}
pub fn k(keep x: String) -> String { return x }
```
Also from v10: opting emit_kinds.nika into the keep check asks for `keep named: bool` / `keep statement: i64` on `EmitFlow::handling`/`at`. A value of a `Copy` scalar is never a view; asking `keep` for it is the same kind of defect.

Probe 2 (`nikaia check` passes, `nikaia run` refuses with "Use of moved value: `b`"; should print 2 twice):
```nika
struct Box { name: String }
impl Box {
    fn show(self) -> i64 { return self.name.len() }
}
fn main() {
    let b = Box { name: "hi" }
    print(b.show())
    print(b.show())
}
```

**Owns:** keep inference (`keeps.nika`) for fields of a struct from another file and for `Copy` scalars, the receiver's lowering, `keep self` on EmitFlow's builders in emit_kinds.nika, with tests. **Not yours:** the migration tool, std's other `.nika` (it still writes `ref self`, which keeps working).

## To build
1. **Keep inference reads a field of a struct declared in another file, and never asks `keep` for a `Copy` scalar**: probe 1 checks clean; a package test in `tests/compiler/` (or a package's tests) with the struct in a second file, plus a test where a function that really keeps a field of a cross-file struct is still NK2302. Commit `Part of #1213`.
2. **A bare `self` is lent**: probe 2 runs and prints 2 twice; `keep self` still consumes (a test that uses the value after a `keep self` call is refused); EmitFlow's `self with {..}` methods take `keep self` and the compiler builds. Tests in `tests/compiler/`. Commit `Part of #1213`.

List under `Body:` every Rust function added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
both probes behave as said, the tests pass, the gate is green.

## Stop and report if
lending `self` needs a decision ADR-views-3 does not make, other than the `Copy` question decided above.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v11-keep-cross-file.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
