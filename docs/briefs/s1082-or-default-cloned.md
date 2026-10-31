---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: `Entry::or_default` and `Seq::cloned` in `std` (#1082, ADR-384; unblocks C1a)

Score: self-hosting lane. C1a (`contracts/mod.rs`, plan row C1) wrote around both (`or_insert_with(fn() { collections::BTreeSet() })` for `or_default`, `.map(fn(p) { p.clone() })` for `.cloned()`), which `CLAUDE.md` forbids; C1a re-runs after this and #1080 (bug lane b19). Decided by the architect in ADR-384 (obvious `std` counterparts of Rust calls); nothing goes to the owner.

Facts: Where things live, Traps already met, Commands

## Decided (ADR-384, read it whole)
* D1: `m.entry(k).or_default()` = `or_insert_with(fn { V() })`, both maps, `sync = true`, `touches = []`, result `ref V` as `or_insert_with`'s. Offered for a `V` with a no-argument constructor (collection, `String`, number, `bool`, a struct whose fields all have defaults). Another `V` is refused with the **existing** code for a call whose type does not fit, help naming `or_insert_with`. If no existing code fits, stop: `Needs: NK code` (codes are the supervisor's).
* D2: `xs.iter().cloned()` = `xs.iter().map fn(x) { x.clone() }`; signature as `Seq::map`'s minus the lambda (`[T](self: Seq[T]) -> Seq[T] ends sized`, with `sync`/`touches` as `map` passes them). Views lower to Rust's `.cloned()`; owned elements to any form rustc accepts.

## Locations
* `crates/nikaia-std/std.contracts`: `[fn."Entry::or_insert_with"]` (:2681), `[fn."Seq::map"]` (:3064). Put each new entry beside them, with a comment in the style of its neighbours.
* Runtime: `crates/nikaia-std/src/map.rs:591` (`or_insert_with` on the crate's own entry, `TrustedMap`'s) and `crates/nikaia-std/src/seq.rs:631` (the pausing map's `or_insert_with`) need an `or_default` twin where the lowering reaches them; `seq.rs` :263/:472 (`map` on `Paused`/`Failing` sequences) for `cloned` on those. Check with `nikaia run scripts/where.nika -- or_insert_with` which of the 13 callers are the emitter's.
* Part III `docs/specification/30-nikaia-tooling.md`: the `std::collections` paragraph (:1420, `or_insert`, `or_insert_with` and `and_modify`) names `or_default`; the `Seq[T]` paragraph (:382) names `cloned` beside `map`. One clause each, linking ADR-384.

## To build
1. The probe of #1082 on the unfixed binary (both `NK1171`), under `Tests:`:
```
use std::collections

fn main() {
    let mut m: collections::BTreeMap[String, collections::BTreeSet[String]] = collections::BTreeMap()
    m.entry(["a"].join("")).or_default().insert(["x"].join(""))
    let names = [["b"].join(""), ["c"].join("")]
    let copies: Vec[String] = names.iter().cloned().collect()
    println(f"{m.len()} {copies.len()} {names.len()}")
}
```
   Expected `1 2 2`.
2. `Entry::or_default` (contracts, runtime twins, refusal); `Seq::cloned`.
3. Part III text; set ADR-384's status to `Accepted` (drop `, to build`).
4. Tests `tests/compiler/tests/entry_or_default_seq_cloned.nika` + `.stdout` (the probe, a `HashMap[String, Vec[i64]]` with `or_default().push(1)`, `cloned()` over owned elements after a `map`), and `entry_or_default_refused.nika` + `.stderr` for a `V` with no no-argument constructor.
Commit `std: Entry::or_default and Seq::cloned (Fixes #1082)`.

## Done when
The tests pass through `nikaia test --project tests/compiler --both-settings`, with the tests whose names contain `entry`, `seq`, `clone`, `map` (by name); `lower-std` fixpoint clean if a `tools/*.nika` changed; build, fmt, clippy, `pre-push-check.sh`; committed locally. `Changelog:` one line (the supervisor raises the version). Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
A fix is needed in the checker or the emitter beyond the call lowering of `cloned` and `or_default` (those you may change; name the functions in `Body:`); an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s1082-or-default-cloned.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
