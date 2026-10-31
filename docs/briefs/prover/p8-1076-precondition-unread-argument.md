---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief p8: #1076, a precondition is reported "proved at every call" when a call's argument is one the prover does not read

Facts: Where things live, Traps already met, Commands

Score: S=3 (the prover's own report claims a proof that is not there: the first thing an outside reader of `--asserts` or of claim (b) would find; also NK1300, ADR-380 D1, lets the program through), O=3 (every user's preconditions), C=4 M (one count in `prover_passes.nika`, tests). Area: prover. Before #1053 step 9 (claims in the figure) and before any `assert` goes into `tools/*.nika`.

## Decision (architect, 2026-10-31: a defect, kind 3; ADR-269 D5, D7, D20 and ADR-380 D1 already say what is right)

A call whose argument the prover does not read cannot check the precondition at the call, so it reaches the checked entry (`CallReach::Through`, ADR-269 D20). That call does not prove the precondition: it checks it when it runs, exactly like a `CallReach::Checked` call. `count_checked_calls` counts only `Checked`; it counts `Through` too. Then the claim reads `precondition of \`f\`: one call checks it when it runs; ...`, is listed under "not proved", and where the build refuses unproved claims (ADR-380 D1, the default) the build refuses it (`NK1300`) as it refuses a `Checked` call's precondition today. No new NK code.

## Problem (probed 2026-10-31 on the binary of 28acd94)

```nika
fn at(xs: ref Vec[i64], i: i64) -> i64 sync {
    assert(i >= 0 && i < xs.len())
    return xs[i]
}

fn main() {
    let xs = [1, 2, 3]
    let k = xs[2]
    print(f"{at(xs, k)}")
}
```

`nikaia check --asserts h.nika`: `proved 1 (proved 0, preconditions proved at every call 1)`, `precondition of \`at\`: proved at every call the compiler sees`; `Checked`. `nikaia run h.nika`: the program stops, `assertion failed` (the checked entry, k == 3). The same with `k` counted by a `for` over `cli::args()` (the counter is a name the prover walk drops after a `for`, item 11 of the lane, not this brief). With `at(xs, 7)` the call is refuted (NK1207), with `pick(xs, 2)` carried back (`k + 1`) it is refuted too: both stay as they are.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/prover_passes.nika` :182 `count_checked_calls` (the fix).
* `crates/nikaia-std/src/tools/prover_state.nika` ~:720-745 (where `CallReach::Through` is chosen when `goal == null`), read only.
* `crates/nikaia-std/src/tools/check_facts.nika` :668 `check_claim_held`, :694 `check_claim_refused` (the texts; change only if a text is wrong for a `Through` call, say so in `Body:`).
* `crates/nikaia-std/src/tools/prover_results.nika` :47 `CallReach`.
* Re-lower each changed `.nika`, commit its `lowered/*.rs`. None of these files is in another lane's current batch.

## Steps

1. Count `Through`. Tests: `tests/compiler/tests/precondition_unread_argument.nika` + `.stderr` (the probe: refused, `NK1300`, the message as it prints) and `tests/compiler/tests/precondition_unread_argument_warn/` only if a test can set `unproved-claims = "warn"` through a project's `nikaia.toml` in `tests/compiler` (look how existing tests do it, `grep -l unproved-claims tests/compiler`; if none can, skip the warn test and say so under `Tests:`). Both fail on the unfixed binary (say how). Keep the call that proves it (`at(xs, 1)`) proved: a second function in the test file, and the refuted and carried-back calls of the probe unchanged.
2. Keep passing: every `tests/compiler` test whose source holds `assert(` (22 files today; run them by name, `--both-settings`), `cargo test -p nikaia --test it -- assert prove claim precondition`. A test that now rightly reports an unproved precondition: re-bless and name it in `Body:`; a test that is newly refused: stop and report `partial` with the list, the fix stays committed only if every such test is a true unproved call. `lower-std` and its fixpoint clean (std and tools hold no `assert` today). `Fixes #1076`.

Changelog: write one sentence (it changes what is refused), not `none`.

## Done when

The tests above pass; build, fmt, clippy, `pre-push-check.sh`; committed locally. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if

The fix needs a file outside *Locations*, their lowered twins and the tests; an existing test that is not a true unproved call becomes refused; an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p8-1076-precondition-unread-argument.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
