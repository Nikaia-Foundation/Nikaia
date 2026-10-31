---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief p12: #1053, every length is at most 2^63 - 1 (ADR-385 D1), and the tools' lists of types from another file take space

Facts: Where things live, Traps already met, Commands

Score: S=3 (claim (b): counters of `tools/*.nika` left checked that the language should prove; the figure the owner reads), O=3 (every program with a list of zero-size or not-yet-known elements), C=3 M (one axiom in the bounds walk, one trace in the tools' check, tests). Area: prover. Figure before (architect, 2026-10-31, `prove-through-loops:on`): 1,576 / 2,677 indexes, 846 / 1,633 operations; 439 sites "a counter a loop changes", of them 82 `x += 1` inside a `for` over a collection and 52 inside a `while`.

## Decision (architect, 2026-10-31: ADR-385 D1, obvious; read `docs/specification/adr/adr-385.md`)

Every length (list, text, `Bytes`, map, set, a `for`'s turns) is `0 <= n <= 2^63 - 1` for a proof, sized or not; ADR-306 D12's `n <= 2^60` stays where elements take space.

## Problem (probed 2026-10-31 on the 21:11 `target/fast/nikaia`)

1. `nikaia check --bounds`, both settings, leaves all three checked ("a counter a loop changes"):

```nika
struct Z {}

fn a(hs: ref Vec[Z]) -> i64 {
    let mut index = 0
    let mut k = 0
    while index < hs.len() {
        index += 1
    }
    for h in hs {
        k += 1
    }
    return index + k
}

fn main() {
    println(f"{a([Z {}])}")
}
```

After D1, `index += 1` and `k += 1` are proved; `index + k` stays checked (rightly: two lengths of up to 2^63 - 1). The same program with `struct Z { v: i64 }` proves the first two today.

2. The same shape is proved in a program and not in the tools: `crates/nikaia-std/src/tools/cexport.nika` :176-190 (`while index < handles.len() { if !held[index] { index += 1; continue } ... }`, `handles = cx_handled_structs(words, items)`, a `Vec[CxHandled]`, `CxHandled` declared in `cexport_model.nika` :247) reports `index += 1` at :179 and :188 checked, while `handles[index]` at :182 is proved. A copy of that function with its own struct `H { name: String, fields: Vec[(String, String)] }` in one file proves all of them. Suspected (not traced, medium sure): in `lower-std`'s per-file check, `CheckState::takes_space` (`check_decls.nika` :123, `named_takes_space` :142, `self.structs` :167) does not see a struct declared in another file of the tools package, answers *no* ("anything not known is answered no"), and the length goes unsized (`bounding_length_named`). D1 frees `index += 1`; this step frees `len + 1` and the 2^60 bound for every such list.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/bounds_walk.nika` :1728 `bounding_length_named` (the unsized branch only puts the floor `n >= 0` into `walk.floors`; add `n <= 2^63 - 1` there), :223 `length_below`, :24 / :140 the `unsized`/axiom comments; :3110 `bounding_turn_bounds` (D13's turn count: check the turns of a `for` over an unsized collection get the same cap).
* `crates/nikaia-std/src/tools/check_decls.nika` :76 `elements_take_space`, :123 `takes_space`, :142 `named_takes_space`; `crates/nikaia-std/src/tools/check_core_stmt.nika` :1600 `check_core_stmt_for` (records `sized_lengths`), `check_core_method.nika` :917 (the `.len()` twin).
* Where the tools' per-file check gets its struct table: `crates/nikaia/src/sysroot.rs` :549 `bounds_tools` and what it calls; `sysroot.rs` is also edited by the main lane's `gr2` move: stage your own hunks only (`git add -p`), never the whole file if it holds others' changes.
* The prover walk's twin, if it keeps its own length axioms: search `prover_state.nika` / `prover_walk.nika` for `length_below` and apply D1 there too.
* Re-lower each changed `.nika` (`lower-std`, builder-facts), commit its `lowered/*.rs`; never stage `lowered/project_members.rs` (#1060 residue). Files of the other lanes' current briefs you must not edit: `grammar_run.nika`, `ledger_items.nika`, `text_tiers.nika`, `check_function.nika`, `check_overflow.nika`, `check_texts.nika`, `check_core_leaves.nika`, `crates/nikaia/src/contracts/mod.rs`. `check_core_stmt.nika` is b13's (bug lane): read only.

## Steps

1. **D1.** The unsized length gets `n <= 2^63 - 1`; a `for`'s turn count over it too. Test `tests/compiler/tests/length_of_zero_size_elements.nika` + `.stdout` + `.bounds` + `.loops.bounds` (p5's form: `NAME.bounds` at `off`, `NAME.loops.bounds` at `prove-through-loops:on`): probe 1, `index += 1` and `k += 1` proved, `index + k` checked. Show under `Tests:` that its `.bounds` differs on the unfixed binary (`/home/user/nikaia-suite/target/fast/nikaia` when no suite runs there).
2. **The tools' struct table.** Confirm or refute the suspicion with one temporary print (removed before commit) in a `lower-std --bounds` run: does `elements_take_space` answer *yes* for `handles` in `cx_export`? If *no* because the struct is another file's, make the tools' check see every struct of the package (the cause you find decides the place; say it in `Body:`). If the cause is elsewhere, fix it if it is in the bounds walk or the checker's facts, else stop, `partial`, with the trace. Test: a `tests/compiler` program cannot hold two files of the tools package, so guard it with the figure: `cexport.nika` :179 and :188 proved in the `lower-std --bounds` report, written under `Tests:`.
3. **Measure and record.** `target/fast/nikaia lower-std --bounds --asserts --optimization prove-through-loops:on > R` then `target/fast/nikaia run scripts/proved.nika -- R`, and the same without the optimization; put both `proved:` lines and the counter count before/after into `Body:`, and replace the figure table in `docs/formal-verification.md` §2 with today's (both settings, this commit's short sha). Part III: add to the `optimization` paragraph of `docs/specification/30-nikaia-tooling.md`, after "as below 2^60;", the clause "every length, and the number of turns a `for` takes, is at most 2^63 - 1" (ADR-385 D1; the driver raises the version).
4. Keep passing: every `tests/compiler` program with a `.bounds` or `.loops.bounds` file (re-bless one whose report rightly gains a proof and name it in `Body:`; one that loses a proof is a defect: stop, `partial`), `cargo test -p nikaia --lib -- bounds`, `cargo test -p nikaia --test it -- bounds prove`.

Changelog: one sentence (what is now proved), not `none`.

## Done when

Steps 1-4 built and tested; build, fmt, clippy (1.97 and 1.99.1), `pre-push-check.sh`, `lower-std` fixpoint; committed locally with `Part: #1053`. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p12-1053-length-is-an-i64.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
