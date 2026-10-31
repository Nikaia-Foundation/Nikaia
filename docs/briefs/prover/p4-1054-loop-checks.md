---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief p4: #1054 step 2, the bounds walk asks a loop's checks with the prover's invariants, and states a sum's goal against a value of its type

Facts: Where things live, Traps already met, Commands

Score: S=3 (ADR-361 D10 is decided and not built for the checks: p2 built it for `assert`s only; 281 of the tools package's unproved operations are `x += 1`), O=3 M (every counter extension of the lane, #432 next, rests on it), C=10 M (two places in `bounds_walk.nika`, the portfolio reused from `prover_loops.nika`), area prover.

## Problem (probed 2026-10-31 on 1ebd8fc's binary)

p2 (1ebd8fc) proves an `assert` inside a loop through the loop. Its step 2, the checks the compiler writes itself (D10), was not built. The report found two causes:

1. **The bounds walk guesses its own loop invariants.** `bounding_loop_guesses` only offers halves of the facts the loop starts with. It never offers the prover's closed forms (ADR-361 D5 item 1), so `s == i` is never a candidate. ADR-361 D10 says a check is asked "through the loop's problem (D2)", with the same portfolio as an `assert`. One portfolio, not two.
2. **A sum is proved only below 2^62.** `bounding_fits` asks `t <= min(type max, solvable())`, where `solvable()` is 2^62 (#444: the solver counts in `i64`, so the goal `t <= i64::MAX` cannot be stated). With `while i < n`, `n: i64` unbounded, `i + 1 <= n` holds, but `n` may be above 2^62, so `i += 1` stays checked even with no loop reasoning at all.

Probe (`target/fast/nikaia check --bounds --optimization prove-through-loops:on probe.nika`, outside `tests/compiler`):

```nika
fn k() -> i64 sync {
    let mut i = 0
    let mut s = 0
    while i < 10 {
        s += 1          // checked today; proved after (s == i, i < 10)
        i += 1
    }
    return s
}

fn g(n: i64) -> i64 sync {
    let mut i = 0
    while i < n {
        i += 1          // checked today; proved after (i + 1 <= n, n an i64), also under `off`
    }
    return i
}

fn f(n: i64) -> i64 sync {
    let mut i = 0
    let mut j = 0
    while i < n {
        j += 1          // checked today; proved after (j == i, i + 1 <= n)
        i += 1          // checked today; proved after
    }
    return j
}

fn w(xs: ref Vec[i64]) -> i64 sync {
    let mut i = 0
    let mut s = 0
    while i < xs.len() {
        s += 1          // checked today; proved after (s == i, i < xs.len())
        i += 1          // proved today
    }
    return s
}

fn c(xs: ref Vec[i64]) -> i64 sync {
    let mut n = 0
    for x in xs {
        if x > 0 { n += 1 }   // stays checked: a counter bounded by the turns (#432, the next brief)
    }
    return n
}

fn main() {
    print(k() + g(3) + f(3) + w([1, 2]) + c([1, -1]))
}
```

Today: `11 operations, 1 proved`. Lane figure today (`lower-std --bounds`, then `scripts/proved.nika`): 1,493/2,583 indexes and 712/1,594 operations under `off`; 1,525 and 719 under `prove-through-loops:on`.

## Decision (architect, 2026-10-31, under *Language decisions*: it changes what is proved, never what a program means)

* **D10 uses the prover's portfolio.** The candidates the bounds walk checks for a loop come from `prover_loops.nika` (`loop_found` / `loop_candidates`), as an `assert`'s do. The existing guesses stay as extra candidates. There are two ways to do it: build a `LoopRecord` over the bounds walk's `TermArena` and entry facts, or move the closed-form candidate step out of `loop_candidates` into a function both walks call. Pick one and say which in `Body:`. D4 is unchanged: every candidate is checked at entry and for one turn before it is used (the walk's own checks, `bounding_proves` and `bounding_loop_walk`).
* **A goal relative to a value of the type.** If `bounding_fits` cannot prove the constant goal for a step that moves one way (`bounding_one_way` > 0, so `t + k` with `k >= 0`), it may also ask `t <= y`, for each name `y` that a fact of the point bounds a name of `t` by (`x < y`, `x <= y`) and whose type's range lies inside `ty`'s. This is sound because `y` is a value of a type inside `ty`. The goal is linear and needs no constant above 2^62. The downward side is the mirror (`t >= y`). At most 8 such names per goal, the newest facts first, so a long body costs what a short one does (like `most_facts()`). This works under `off` too: it is ADR-306's walk, not only D10's.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/bounds_walk.nika`: `bounding_loop_stmt` (line 2183, the D10 path: guesses, entry check, `bounding_could_pay`, re-walk), `bounding_loop_guesses` (2361), `bounding_loop_walk` (2350), `bounding_fits` (834), `bounding_one_way`, `bounding_proves_goal`, `solvable` (225), `most_facts` (232).
* `crates/nikaia-std/src/tools/prover_loops.nika`: `LoopRecord` (22), `loop_record` (94), `loop_record_range` (104), `loop_candidates` (449), `loop_found` (896), `loop_facts` (972), `loop_turn_facts` (1017).
* `crates/nikaia-std/src/tools/prover_state.nika`: `loop_invariant` (244), shared by p2. It shows how the prover builds and caches a record.
* Re-lower each changed `.nika` (`lower-std`, builder-facts) and commit the diff in `crates/nikaia-std/src/tools/lowered/`. None of these files is in the main lane's or the bug lane's current batch.

## Steps

1. **The relative goal** in `bounding_fits`. With `check --bounds` (under `off`), `g`'s `i += 1` is proved. Soundness rows, which must stay checked: `while i < n { s += 2; i += 1 }` with `s` starting at 0 (`s + 2 <= n` does not follow); an `i32` sum bounded only by an `i64` name; `while true { s += 1 }`. Put these rows in a second probe file and quote its header line under `Tests:`. Commit.
2. **The portfolio for D10.** Under `prove-through-loops:on`, `k`'s `s += 1`, `f`'s `j += 1` and `i += 1`, and `w`'s `s += 1` are proved. `c`'s `n += 1` stays checked. Soundness rows that must stay checked: `while i < 100 { s += 2; i += 1 }` with `s: i8` (200 leaves `i8`); a counter assigned `s = s * 2` in the body. Keep `crates/nikaia/tests/it/prove_through_loops.rs` and the `prove_*` tests in `tests/compiler/tests/` passing. Commit.
3. **The figure.** Run `lower-std --bounds`, with and without `--optimization prove-through-loops:on`, then `scripts/proved.nika` on each. Give both `proved:` lines under `Tests:` and in `Body:`. `tests/compiler` cannot show `--bounds` output, so `check --bounds` on the probes stands for the tests: add no Rust test (CLAUDE.md). This finishes #1054: the report says `Fixes: #1054`.

Stop about 15 turns before `max_turns`. Commit what is finished and report `partial`, with what you ruled out.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `docs/briefs/prover/p4-1054-loop-checks.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
