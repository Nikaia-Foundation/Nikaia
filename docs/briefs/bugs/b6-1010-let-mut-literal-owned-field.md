---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1010, a `let mut` text literal reassigned in an `if` and pushed as an owned variant field reaches rustc (E0308)

Score: S=3 (a false acceptance: the checker passes, rustc refuses; found by the yaml lane, y4b), O=0.3 M, C=4 M (the text tier walk, like #943 at 417fc047), area bugs.

## Issue #1010 (copied; probe completed by the architect and run on 832b07dd+)

```nika
enum E { V(String), N }
fn main() {
    let c = true
    let mut held: Vec[E] = []
    held.push(E::V(f"{1}"))
    let mut why = "a"
    if c { why = "b" }
    held.push(E::V(why))
    for e in held { match e { E::V(s) => println(s), E::N => println("n") } }
}
```

Today: rustc *Mismatched types* at `held.push(E::V(why))` (`&str` vs `String`). Expected `1` then `b`. The variant field is owned text (another push builds it with `f"..."`), so `why` must be owned where it goes in, or the push converts it.

## Locations

* `crates/nikaia-std/src/tools/text_tiers.nika`: the tier of a `let` literal and of its reassignments (417fc047, #943: a literal bound by a `let` is declared `String`); how a push into a variant whose field owns decides (#942, #969 in adfaf97a's list). ADR-282 D12-D14 (`docs/specification/adr/adr-282.md`) govern which tier a text takes.
* Check the same program without the `if` (`let mut why = "a"`, `why = "b"`, push) and with `let why = "a"` (no `mut`): if they run, the defect is the join over the `if`; say which in `Body:`.

## To build

1. The tier of a `let mut` text literal follows its uses through every assignment, the ones in an `if`/`match` arm included: a use that hands it to an owned field makes it owned at the `let` (or converts at the use, whichever ADR-282 D12-D14 prescribe; name the D in `Body:`).
2. Test `tests/compiler/tests/a_let_mut_literal_into_an_owned_variant.nika` with `.stdout`: the probe, the same with `match` in place of `if`, and a struct field in place of the variant. Run against the unfixed compiler first; then the tests whose names hold `tier`, `literal` or `let_mut`.

Only `text_tiers.nika` (and, if the tier is read there, the emit file that spells the `let`), their lowered files and `lowered.rs`, and the new test are yours; another file needed: stop, report `partial` with what and why.

The commit that finishes it says `Fixes #1010`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b6-1010-let-mut-literal-owned-field.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
