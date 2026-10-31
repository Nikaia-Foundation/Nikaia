---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief p10: #1084, a length through `Bytes(x)`, a bound `[v; n]`, and a list literal or repeat passed as an argument (fix-forward of p8's red suite)

Facts: Where things live, Traps already met, Commands

Score: S=3 (the lane's suite is red since p8, #1076: p7 and p8 cannot be pushed until it is green; and a length precondition that plainly holds is refused, NK1300, the "lengths of lists" outsiders name), O=3 (every user's `Bytes(..)` and `[v; n]`), C=3 M (two functions in two prover files, four test programs). Area: prover.

Pre-flight: `git log --oneline -8 | grep 788981d` (p8, the commit that made the four tests red). If absent, stop: `Result: blocked`, `Needs: p8 commit missing`.

## Problem (probed on 788981d, `target/fast/nikaia check --asserts`)

```nika
fn g(xs: ref Vec[u8]) -> i64 {
    assert(xs.len() >= 40)
    return xs.len()
}
fn h(b: ref Bytes) -> i64 {
    assert(b.len() >= 4)
    return b.len()
}
fn main() {
    let v: Vec[u8] = [59; 40]
    println(f"{g(v)}")            // refused: an immutable repeat's length is not recorded
    let mut m: Vec[u8] = [59; 40]
    println(f"{g(m)}")            // proved today (#782, prover_mutable_list)
    println(f"{g([59; 40])}")     // refused: a repeat or literal argument is not read
    let w: Vec[u8] = [1, 2, 3, 4]
    println(f"{h(Bytes(w))}")      // refused: `Bytes(x)` does not keep x's length
}
```

Also probed: a straight-line `push` chain, and `v.resize(64, 0)` followed by `for j in 0..<64 { v[j] = j as u8 }`, both give `g(v)` proved; a `for j in 0..<4 { u.push(j) }` does not (that is #1053 item 10, not this brief).

Since p8 these four `crates/nikaia/tests/it` tests fail with NK1300, each a true unproved call (`Bytes(v)` with `v` filled by a `for` of `push`): `prove_through_loops::a_break_guard_carries_its_bound_through_the_loop`, `prove_through_loops::a_break_guard_does_not_hold_past_the_loop` (program `SCANNED`, :252), `unsafe_follows_switches::with_every_switch_off_the_emitted_code_holds_no_unsafe`, `unsafe_follows_switches::with_the_switches_on_each_unsafe_is_a_call_into_proven` (program `DROPPED`, :12; it also passes the literal `[1, 2, 3, 4, 5, 6, 7, 8]` to `xs: ref Vec[i64]` with `xs.len() == 8` claimed).

## Decision (architect, 2026-10-31, #1084; obvious by the language principles, no specification change: what the prover knows of `std`)

* `Bytes(x)` has `x`'s length (as `find_all` keeps the length of what it searches, ADR-375 D5).
* `[v; n]` has length `n` wherever it is bound (`let` too, not only `let mut`, #782) or passed; `n` read by `lin_term`, not recorded where `n` mentions the bound name (as `prover_mutable_list` does).
* A list literal passed as an argument has its item count, as a bound one has.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/prover_walk.nika` :590 `prover_let` (the `match value` at :713: `ListLit` gives its count, `ListRepeat` nothing yet; add `ListRepeat` like :2568 and `Bytes(x)`); :2561 `prover_mutable_list` (the repeat rule to share; also `let mut b = Bytes(x)` if it goes through here).
* `crates/nikaia-std/src/tools/prover_state.nika` :602 `precondition_call` (:611-628: only an `Expr::Variable` argument gives `length_of(param)`; add a literal, a repeat, and `Bytes(<variable with a known length | literal | repeat>)`). How `Bytes(..)` appears in the tree (a call on the name `Bytes`, one argument): find it with a probe; a user function named `Bytes` cannot exist beside the `std` type, but check the checker resolves it to `std`'s.
* Tests: `crates/nikaia/tests/it/prove_through_loops.rs` :252 `SCANNED`, `crates/nikaia/tests/it/unsafe_follows_switches.rs` :12 `DROPPED` (changed only to keep them passing, CLAUDE.md).
* Re-lower both `.nika` files (`lower-std`, fixpoint): `crates/nikaia-std/src/tools/lowered/prover_walk.rs`, `prover_state.rs`.

## Steps

1. The prover rule (both functions). Test: new `tests/compiler/tests/prove_argument_lengths.nika` with `.stdout` (it runs) and `.bounds` / `.loops.bounds` (p5's form; take them from what the binary prints): the probe above, every call proved; soundness rows that stay refused or warned and so go into a second program `tests/compiler/tests/prove_argument_lengths_refused.nika` with its `.stderr` (NK1300): `h(Bytes(x))` with `x` a parameter of unknown length, `g([59; k])` with `k` a parameter, `g([1, 2, 3])` against `>= 40` (that one is NK1207, false at the call: whichever the binary says, name it in `Body:`). Run both on the unfixed binary (`/home/user/nikaia-suite/target/fast/nikaia` if present, else a copy of `target/fast/nikaia` taken before your build) and say under `Tests:` that the proved rows were refused there.
2. The four `it` tests: in `SCANNED`'s `main` build `v` as `let v: Vec[u8] = [59; 40]` (same bytes); in `DROPPED`'s `main` as `let mut v: Vec[u8] = []`, `v.resize(64, 0)`, `for j in 0..<64 { v[j] = j as u8 }` (same bytes). The test assertions stay as they are. Run `cargo test --profile fast -p nikaia --test it -- prove_through_loops unsafe_follows_switches` (or the profile the builder facts name): all pass.
3. Also run `target/fast/nikaia test tests/compiler --both-settings` filtered to `prove` and `precondition` (or whole, about 10 min) and `cargo test -p nikaia --test it -- prove bounds asserts`: nothing else changes; a test whose message rightly changes is re-blessed and named in `Body:`.

`Fixes: #1084`. Changelog: one sentence (a toolchain change).

## Done when

Build, `lower-std` + fixpoint, fmt, clippy (1.97 and, when installed, 1.99.1), `pre-push-check.sh`, the tests above green; committed locally. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if

The change needs a file outside *Locations*; an error in another agent's file; a fourth `it` test beyond the named ones turns red (report it, do not edit it).

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (`crates/nikaia-std/src/tools/lowered/project_members.rs` is residue, #1060), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p10-1084-argument-lengths.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
