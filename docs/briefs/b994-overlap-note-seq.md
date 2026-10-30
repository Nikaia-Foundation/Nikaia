---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: #994, the NK2104 note for stdout against stderr still recommends `seq`, which ADR-292 D7 removed

Score: bug lane, a wrong message (S=1, cheap): a refusal that names a form the language does not have sends the reader to a dead end. Builders have no GitHub: the issue text is copied below.

## Problem (issue #994, probed by the architect on 9ffc1ec2)

Two `overlap` branches where one writes stdout and the other stderr are refused with `NK2104`, correctly (the two count as one destination). The note says `Use `seq` to run them in order.`, but `seq { … }` does not exist: ADR-292 D7 removed it, and `seq` is not a reserved word.

```nika
fn a() -> i64 { println("a"); 1 }
fn b() -> i64 { eprintln("b"); 2 }
fn main() {
    let (x, y) = overlap {
        a()
        b()
    }
    println(f"{x} {y}")
}
```
`nikaia check` today:
```text
error[NK2104]: These two branches can't run at the same time.
  --> p994.nika:6:9
   ...
   = note: One touches stdout and the other stderr, which might be the same place. Use `seq` to run them in order.
   = note: `overlap` runs its branches at once, so they can't depend on each other.
   = help: If they should run in order, write them as ordinary statements.
1 branch that cannot run beside the others
```

## Locations

* `crates/nikaia-std/src/tools/order.nika:119` (`Verdict::SameDestination`), lowered `crates/nikaia-std/src/tools/lowered/order.rs:69`.
* `crates/nikaia-std/src/tools/touch.nika:58-62` (doc comment of `family`: "D7's `seq` is the escape").
* The architect's grep found no other message naming `seq` in `crates/nikaia-std/src/tools/*.nika` (the other hits are names such as `read_seq`).
* Known facts: read `docs/builder-facts.md` (re-lowering a tools file, running a test through `nikaia test`).

## To build

1. In `order.nika`, drop the `seq` sentence: the note ends after "which might be the same place" (the help already gives the way out). Re-lower as `docs/builder-facts.md` says.
2. Correct the comment in `touch.nika` (`family`): the escape is ordinary statements (ADR-292 D1), not `seq`.
3. Test `tests/compiler/tests/overlap_stdout_stderr_note_names_no_seq.nika` with the program above and its `.stderr` taken from what the compiler prints (run it through `nikaia test`). If an existing `.stderr` holds the old sentence (`grep -rl 'Use `seq`' tests/ crates/nikaia/tests`), update it.
4. Commit line: `order: the NK2104 note for stdout and stderr no longer names seq (#994)`, `Fixes #994`.

## Tests

* The new test and the gate's guard set of `.claude/agents/builder.md` (no `check_*`/`emit_*` change, so no `--both-settings` run).

## Files

May change: `order.nika`, `touch.nika`, their lowered `.rs`, new files in `tests/compiler/tests/`, an existing `.stderr` holding the old sentence. Not yours: `crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`, any other uncommitted file in the checkout.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b994-overlap-note-seq.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
