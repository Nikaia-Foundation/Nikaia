---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief p3: #1053 step 3, the catch-all reason of `--bounds` split by how the unbounded name was bound

Facts: Where things live, Traps already met, Commands

Score: S=2 (the lane's order rests on it: 1,580 of 1,677 unproved sites in `tools/*.nika` share one reason today, so the next prover extension cannot be chosen by count), O=2 M (each later brief picks the extension that frees the most from this table), C=6 M (a syntax-only pass over one function body in `bounds_reasons.nika`; like the reason move, #435), area prover.

## Problem (probed on d206915, `target/fast/nikaia check --bounds FILE`)

`crates/nikaia-std/src/tools/bounds_reasons.nika` reads the reason off the site's syntax only: a field, a list of lists, a product, a call, and otherwise `unbounded()`, which returns *no fact bounds what it reads - a counter, a loop, a length, or one list as long as another (#432, #383, #388)*. Over the tools files that is 1,580 of 1,677 unproved sites. The probe:

```nika
struct S { held: Vec[i64], why: Vec[i64], n: i64 }

fn count(xs: ref Vec[i64]) -> i64 sync {
    let mut n = 0
    for x in xs {
        if x > 0 { n += 1 }          // checked: catch-all; a counter grown in a loop
    }
    return n
}

fn ifval(xs: ref Vec[i64], k: i64) -> i64 sync {
    let i = if k > 0 && k < xs.len() { k } else { 0 }
    if xs.len() > 0 { return xs[i] } // checked: catch-all; an `if` value
    return 0
}

fn field(mut s: S) sync {
    let mut i = 0
    while i < s.held.len() {
        s.why[i] = s.held[i]         // s.why[i] checked: catch-all; the length of another list
        i += 1
    }
}

fn param(xs: ref Vec[i64], k: i64) -> i64 sync {
    return xs[k]                     // checked: catch-all; a parameter no precondition bounds
}

fn element(xs: ref Vec[i64], ks: ref Vec[i64]) -> i64 sync {
    if ks.len() == 0 { return 0 }
    let k = ks[0]
    return xs[k]                     // checked: catch-all; a value read out of a list
}

fn main() {
    let v = [1, 2, 3]
    let mut s = S { held: [1], why: [0], n: 0 }
    field(s)
    print(count(v) + ifval(v, 1) + param(v, 0) + element(v, [0]))
}
```

## Locations

* `crates/nikaia-std/src/tools/bounds_reasons.nika`: `blocked_index`, `blocked_operation`, `site_shape`, `unbounded` (line 126).
* `crates/nikaia/src/bounds_report.rs`: the `Walk` (line 154) calls them at lines 204, 219, 236 and 252 through the adapters at lines 287 and 291. It walks one function's block. Hand the enclosing function (its parameters and body) to the Nikaia side through the adapters with unchanged roles. The classification itself is written in Nikaia, not in Rust.
* `docs/formal-semantics.md` and `docs/solver-workload.md` quote the old text. Leave them unless they claim it as current output.
* A changed `.nika` is re-lowered with `lower-std` (builder-facts), and the diff in `crates/nikaia-std/src/tools/lowered/` is committed.

## Steps

1. **The reason from the binding.** For a site the syntax rules leave at `unbounded()`, find the first name it reads that has no fact, and how that name was bound in the enclosing function. Name the reason from that, each with where its fact would come from:
   * a `let mut` changed by `+=`/`-=` (or `= x + k`) inside a loop: *a counter a loop changes, with no bound tied to the loop's turns (#1053 step 4, #432)*;
   * a `let` from an `if` or a `match`: *an `if` or `match` value, which the walk does not read (#1053)*;
   * a parameter: *a parameter no precondition bounds: an `assert` at the top of the function makes it one (ADR-269 D5)*;
   * a `let` or `for` binding read out of a list or a map: *a value read out of a list, of which the walk knows no fact (#797, #798)*;
   * an index of one list in a loop bounded by another list's length: *the length of another list (#388)*;
   * otherwise the old text, unchanged.
   The exact words are yours, but each names its issue or ADR. Test `tests/compiler/tests/` cannot show `--bounds` output, so check the probe above with `check --bounds` and quote the five reasons under `Tests:`. Keep every existing test that prints a reason passing (update one whose text rightly changes and name it in `Body:`). If a `--bounds` test exists in `crates/nikaia/tests/it/`, keep it passing. Add no Rust test.
2. **The table.** If `scripts/proved.nika` exists (brief p1), run `nikaia lower-std --bounds` and the script, and paste the reason table (count, reason) into the commit message and under `Tests:`. Otherwise do the same with the per-file `check --bounds` over `crates/nikaia-std/src/tools/*.nika`, and say which you used.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `docs/briefs/prover/p3-1053-reasons.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
