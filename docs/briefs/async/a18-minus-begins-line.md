---
agent: builder
model: opus
max_turns: 60
budget_usd: 8
effort: medium
---


# a18: a `-` that begins a line begins an expression (#1247, ADR-390)

Score: [ADR-390](../../specification/adr/adr-390.md) is the record. A subtraction's `-` is written on the line of what it subtracts from. A `-` that is the first thing on its line begins what is written there, as `[` (ADR-135 D3) and `(` (ADR-317) already do. Part I 3.1 already says so (next to "A statement begins a line").

**The defect** (a16 found it). Probe:

```
let n = run() catch {
    println("x")
    -1
}
```

The last line is read as `println("x") - 1`, and rustc refuses it with *Cannot subtract {integer} from ()*.

**Where it is**: the parser's grammar block, `crates/nikaia/src/parser/mod.rs`. The binary `-` arms are near line 3111 (`BinaryOp::Sub`), and the `head_*` copy of the additive level sits near line 3672. Look at the error arm near line 2922 too. `same_line` (ADR-339 D2) is used the same way for `[` near line 3244 and for a call's `(` near line 3360: follow those. This change is in Rust because the grammar block is Rust, which is allowed for a defect in it. `-=` stays as it is.

**Build early:** `cargo build`, then `lower-std`, after each file you change. A run that ends without a build commits nothing.

**Owns:** the binary `-` rule in the grammar block, the rewrite of the leading-`-` chain in `crates/nikaia-std/src/tools/project_check.nika` (around line 171, with each `-` moved to the end of its line, and its lowered file regenerated), and the tests.

**Not yours:** the specification text and ADR-390 (already written), other operators, async run time.

## To build
1. **A leading `-` begins an expression**: add `same_line` to the binary `-` arms. Rewrite the `project_check.nika` chain. Add tests `tests/compiler/tests/a_minus_that_begins_a_line_*`:
   * the probe, the block's value `-1`, printed;
   * a plain block `{ f()` then `-1 }`, whose value is `-1`;
   * a subtraction broken after its `-` (`let d = a -` then `b` on the next line), which still subtracts.

   Commit `Fixes #1247`.

A further defect: fix it in the compiler with a test as its own commit if it fits; otherwise file nothing and report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
The probe prints `-1` where `run` throws; all three tests pass; `nikaia check` passes on `crates/nikaia-std` and `packages/`.

## Stop and report if
* a program in `crates/nikaia-std`, `packages/` or `examples/` other than the `project_check.nika` chain uses a `-` at the start of a line as a subtraction (list them under `Found:`, do not change them);
* the change needs anything besides `same_line` (a new lexer token, a newline in the token stream): `Result: blocked`, `Found:` why, `Needs: supervisor decision`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a18-minus-begins-line.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
