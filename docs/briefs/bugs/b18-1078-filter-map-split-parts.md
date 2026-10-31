---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1078, `filter_map` over `split_once` loses the parts' text kinds

Blocks: k3b-comptime-bundle.md

Score: compiler bug in the #901 family, a lowering rustc refuses (`&str` where `String` is meant) and a checker refusal (NK1245) of a plain iterator chain; it blocks the K3b move (`runtime_dependencies`' record parse). Traced to a missing branch, fairly sure.

Facts: Where things live, Traps already met, Commands

## Issue #1078 (summary)
After `filter_map(fn(line) { line.split_once("=") })`, a following `map(fn((name, rlib)) { (name.clone(), ...) })` lowers `name` as a `&str` view and `name.clone()` stays `&str`, so collecting into `Vec[(String, String)]` fails in rustc. With `map(fn(line) { line.split_once("=") ?? ("", "") })` in place of the `filter_map`, the chain works; a `for (name, rlib)` loop works too.

Probe (checked on 4520d400's binary with `nikaia run`):
```
fn tag(text: String) -> String {
    f"<{text}>"
}

fn pairs(lines: Vec[String]) -> Vec[(String, String)] {
    lines.iter().skip(1).filter_map(fn(line) { line.split_once("=") }).map(fn((name, rlib)) { (name.clone(), tag(rlib)) }).collect()
}

fn main() {
    for (n, p) in pairs(["head", "a=x.rlib", "b=y.rlib"]) {
        println(f"{n} {p}")
    }
}
```
Row a (above): rustc *Mismatched types*, expected `Vec<(String, String)>`, found `Vec<(&str, String)>`. Expected output `a <x.rlib>`, `b <y.rlib>`.
Row b: the same with `.map(fn(pair) { (pair.0.clone(), tag(pair.1)) })`: the checker refuses with NK1245 (*can't tell what type `pair` is*). Expected the same output.

## Suspected location (traced by reading)
* Row a, fairly sure: `crates/nikaia-std/src/tools/text_tiers.nika`, the method-kinds walk around :4740-4770. `map` binds its lambda's parameters to what came in and hands back what the lambda makes (`mapped(params, body, received, ask, walk)`, ADR-293 D26); `filter_map` has no branch, so the kinds of the option's inside are lost and the next `map`'s tuple parameters read as views. Give `filter_map` the `map` treatment with the option's inside as the element (`flat_map` too if a probe shows the same gap). Name in `Body:` every other function that repeats the `map` rule (`scripts/where.nika -- mapped`).
* Row b, not traced: the checker's element type after `filter_map` of a lambda handing back `T?`. Find it with `nikaia run scripts/where.nika -- filter_map` (the iterator method table of the checker); if row a's fix makes row b pass, say so.

## To build
1. Reproduce rows a and b on the unfixed binary; keep the result under `Tests:`.
2. The `text_tiers.nika` fix for row a, with its lowered twin (`lower-std`, fixpoint).
3. The checker fix for row b, with its lowered twin.
4. Test `tests/compiler/tests/filter_map_split_parts.nika` + `.stdout` with rows a and b, failing on the unfixed binary.
Commit `tiers, check: filter_map hands on the parts of a split it keeps (Fixes #1078)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`, with the tests whose names contain `split`, `tier`, `lambda`, `map`, `view` (by name); `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `text_tiers.nika`, the checker file of row b, their lowered twins and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b18-1078-filter-map-split-parts.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
