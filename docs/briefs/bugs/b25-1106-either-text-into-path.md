---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 10
---
# Brief: #1106, text holding both kinds (`EitherText`) bound to `fs::Path` passes check and fails in rustc

Blocks: k3b-comptime-bundle.md (K3b run 2, `docs/briefs/k3b-comptime-bundle.md.report.md`; its move waits on branch `wip/k3b-comptime-bundle-run2` 707d5e53)

Score: compiler/std defect that blocks a self-hosting move (K3b); small, cause known.

Facts: Where things live, Traps already met, Commands

## Issue #1106 (probe checked by the architect on `target/fast/nikaia` at 7ad2ea08)
```nika
struct Holder { name: String }

fn to_path(h: ref Holder, c: bool) -> fs::Path {
    let mut x = "none"
    x = if c { h.name } else { f"a{c}" }
    let p: fs::Path = x
    return p
}

fn main() {
    let h = Holder { name: f"abc{1}" }
    let a = to_path(h, true)
    let b = to_path(h, false)
    println(f"{a.file_name() ?? "-"} {b.file_name() ?? "-"}")
}
```
Expected `abc1 afalse`. Got rustc: ``The trait bound `PathBuf: From<EitherText<'_>>` is not satisfied`` (`x` holds both kinds, ADR-282 D13, D14; `let p: fs::Path = x` lowers to `PathBuf::from(x)`).

## Location (high confidence)
`crates/nikaia-std/src/either_text.rs`: the `From` impls (lines 63-92 and 287, `impl From<EitherText<'_>> for String`). K3b verified that `impl From<EitherText<'_>> for std::path::PathBuf` fixes its case; move owned text in without a copy where the value holds one (follow the `String` impl at 287).

## To build
1. Reproduce the probe on the unfixed binary (`Tests:`).
2. The `From` impl in `either_text.rs`.
3. Test `tests/compiler/tests/either_text_into_path.nika` + `.stdout`: the probe as written. It fails on the unfixed binary. Also run the tests named `either|mixed_arms|path`.
Commit `std: text holding both kinds converts into fs::Path (Fixes #1106)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line.

## Stop and report if
An edit is needed outside `either_text.rs` and the new test, or you hit an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (now: `tools/emit_core_leaves.nika` and its lowered twin, b22's), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b25-1106-either-text-into-path.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
