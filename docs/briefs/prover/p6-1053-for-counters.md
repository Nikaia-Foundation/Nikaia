---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief p6: #1053 step 4, a `for` counts its turns, and a counter is bounded by them (ADR-361 D13)

Facts: Where things live, Traps already met, Commands

Score: S=3 (claim (b), the owner's 100 % target: 542 of the 1,954 unproved sites in `tools/*.nika` are "a counter a loop changes", the largest named reason; most are `at += 1` in a `for x in xs`), O=3 M (every user's counters too: `for x in xs { n += 1 }` keeps its overflow check in every program today; #432's loop part), C=12 M (the turn count and the candidates in `bounds_walk.nika`). Area: prover.

Pre-flight: p5 (#1069, `.bounds` expectations) is on HEAD: `git log --oneline -8 | grep -i bounds` shows its commit and `tests/compiler/tests/prove_loop_counter_bounds.bounds` exists. If not, stop: `Result: blocked`, `Needs: p5 first`.

## Problem (probed 2026-10-31 on ecf435a's binary)

Probe (`target/fast/nikaia check --bounds [--optimization prove-through-loops:on] c.nika`, outside `tests/compiler`); today every counter below is `checked: a counter a loop changes, with no bound tied to the loop's turns`, at both settings, and only `whiles`' `xs[i]` and `i += 1` are proved:

```nika
fn every(xs: ref Vec[String]) -> i64 sync {     // at += 1: proved after
    let mut at = 0
    for x in xs {
        at += 1
    }
    return at
}

fn some(xs: ref Vec[String]) -> i64 sync {      // n += 1: proved after (n <= t)
    let mut n = 0
    for x in xs {
        if x.len() > 2 {
            n += 1
        }
    }
    return n
}

fn same(xs: ref Vec[i64]) -> i64 sync {         // xs[at] and at += 1: proved after (at == t < xs.len())
    let mut at = 0
    let mut s = 0
    for x in xs {
        if xs[at] > 0 {
            s = 1
        }
        at += 1
    }
    return s
}

fn ranged(n: i64) -> i64 sync {                 // k += 1: proved after (k == t < n - 0)
    let mut k = 0
    for i in 0..n {
        k += 1
    }
    return k
}

fn whiles(xs: ref Vec[i64]) -> i64 sync {       // n += 1: proved after (n - n@ <= i - i@, i < xs.len())
    let mut i = 0
    let mut n = 0
    while i < xs.len() {
        if xs[i] > 0 {
            n += 1
        }
        i += 1
    }
    return n
}
```

## Decision (ADR-361 D13, architect, 2026-10-31; read it in `docs/specification/adr/adr-361.md`)

A `for` over a collection with a length has a turn count `t`, `0 <= t < len` at the start of each turn, `len` the length where the loop is reached (a `for` cannot change what it walks: NK2305), `t` grows by one at each end of a turn, `continue` included. Over `a..b`: `t < b - a`, binding `== a + t`. A `while` uses a name grown by exactly 1 on every path of a turn as its `t` (`i - i@`). For a whole-number name `n` changed only by adding constants of one sign, at most `k` along any path of a turn, the candidates are `n - n@ <= k * t` (mirror for subtraction) and `n - n@ == k * t` where every path adds exactly `k`. Each is checked like any other candidate (ADR-361 D4). Both settings of `prove-through-loops`: these are the walk's own candidates, beside #711's growing counters (`bounding_candidates`); under `on` they also go with p4's portfolio. None for a name grown by an inner loop, by a non-constant amount, or reset in the loop. ADR-306 D12 (lengths below 2^60) closes the overflow goal.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/bounds_walk.nika`: `bounding_for_stmt` (2197) and `bounding_for` (2938) - where a `for` is walked and its binding gets its facts; `bounding_while_stmt` (2228); `bounding_loop_guesses` (2472, the walk's candidates, no candidate reading a `for` binding today); `bounding_loop_proposals` / `bounding_while_proposals` (2546, 2556, p4's portfolio, `while` only); `bounding_turn_end` (2680, D4's check at each end of a turn); `bounding_candidates` (2724) and `bounding_grow` (2753), #711's growing counters; `bounding_fits` (851).
* `crates/nikaia-std/src/tools/bounds_reasons.nika` (p3): the counter reason. A counter these candidates now prove must not keep that reason; one they cannot (inner-loop growth, a reset) keeps it.
* Part III 14.2 (`docs/specification/30-nikaia-tooling.md`, *A loop of a constant number of turns...* paragraph, about line 468): one paragraph after it, saying D13.
* Re-lower each changed `.nika` (`lower-std`) and commit its file in `crates/nikaia-std/src/tools/lowered/`. Neither file is in the main lane's (`docs/briefs/batch.txt`) or the bug lane's current batch.

## Steps

1. **A `for` counts its turns; counters relative to them.** `every`, `some`, `same` and `ranged` proved as noted, at both settings. Soundness rows that must stay checked (a second probe file; quote its header line under `Tests:`): `at += 1` in a `for` with `at` starting from an unbounded parameter (`n@` not bounded: `at + 1` may overflow at entry); `n += 2` in a `for x in xs` with `n: i8` from 0 (200 turns leave `i8`; `xs.len()` is not bounded below 2^60 by anything smaller); a counter reset inside the loop (`if c { n = 0 }`); a counter grown inside an inner `for`; `n = n * 2`. Test: `tests/compiler/tests/prove_for_counts_its_turns.nika` + `.stdout` + `.bounds` + `.loops.bounds` (p5's form), the probe's four `for` functions and the soundness rows, the files read through. Commit.
2. **A `while`'s counters relative to its step counter.** `whiles`' `n += 1` proved at both settings; `while i < n { s += 2; i += 1 }` with `s: i8` stays checked. Test `tests/compiler/tests/prove_while_counter_relative.nika` with the same four files. Keep the `prove_*` tests and `crates/nikaia/tests/it/prove_through_loops.rs` passing. Commit.
3. **The figure.** `target/fast/nikaia lower-std --bounds > R` and `... --optimization prove-through-loops:on > R2` (about 45 s each), `target/fast/nikaia run scripts/proved.nika -- R` and `-- R2`: both `proved:` lines and the counter reason's count under `Tests:` and in `Body:`. Today (ecf435a): off 1,518/2,628 indexes, 729/1,612 operations; on 1,550/2,628, 736/1,612; counter reason 542 (on). `Part: #1053`.

Changelog: a prover change raises the version: write the `Changelog:` line (one sentence), not `none`.

## Done when

The two new tests and the `prove_*` tests pass by name through `nikaia test --project tests/compiler --both-settings`; `cargo test -p nikaia --test it -- prove bounds` passes (anchored `grep -E '^test result:|FAILED|panicked'`); `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Stop about 15 turns before `max_turns`, commit what is finished and report `partial` with what you ruled out.

## Stop and report if

An edit is needed outside `bounds_walk.nika`, `bounds_reasons.nika`, `prover_loops.nika` (only to share a candidate function), their lowered twins, the Part III paragraph and the new tests; a soundness row turns `proved` (stop at once: that is a wrong proof); an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (`crates/nikaia-std/src/tools/lowered/project_members.rs` is uncommitted residue at the time of writing; do not commit it), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p6-1053-for-counters.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
