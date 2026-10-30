---
agent: builder
model: opus
max_turns: 90
budget_usd: 20
---
# Brief: #1006, every set has `iter()` (ADR-collections-2 D3)

Score: std lane, blocks self-hosting row K3a part 2 (`k3a2-comptime-compute.md`, next in this batch: `computed` needs `named.iter().any(..)` over a `BTreeSet[String]`). S=1, a missing `std` entry the owner decided on 2026-10-30. Builders have no GitHub: the issue's *To build* is copied below.

## Decision (already written: ADR-collections-2 D3, Part III `std::collections`)

`s.iter()` on `collections::BTreeSet[T]` lends the elements one at a time, smallest first, as `Vec::iter()` does: `Seq[ref(self) T] sync ends sized`. On `collections::HashSet[T]` the same, in no order (as `HashMap::keys`): `Seq[ref(self) T] sync sized` (Rust's `HashSet` iterator does not run backwards). Lowered to Rust's `.iter()` (a trusted `HashSet` is `TrustedSet`). Pipeline methods on the set itself (`s.any(f)`) are refused by D3: do not add them. Do not edit the ADR or the specification.

## Probe (target/fast at 9d401a96)

```nika
use std::collections

fn main() {
    let mut s: collections::BTreeSet[String] = collections::BTreeSet()
    s.insert("b")
    s.insert("a")
    println(s.iter().any(fn(n) { n == "a" }))
}
```

Today: NK1171 `collections::BTreeSet` has no method called `iter`; the same on a `HashSet[i64]` (`h.iter().count()`). `for n in s` runs already (smallest first).

## To build

1. `crates/nikaia-std/std.contracts`: `[fn."collections::BTreeSet::iter"]` and `[fn."collections::HashSet::iter"]` with the signatures above (`pub = true`, `sync = true`, `touches = []`), next to each kind's other entries, each with a one-line comment (smallest first / no order, as `HashMap::keys` says it). Move the orphan comment *A copy of the ordered set ...* (it sits above `BTreeSet::first`) onto `BTreeSet::clone`.
2. Lowering: both lower to Rust's `.iter()`, also inside `crates/nikaia-std/src/tools/*.nika` under `lower-std` (K3a's code is lowered there). Add a row wherever the self-hosted checker or lowering needs one (find with `nikaia run scripts/where.nika -- <name>`; compare `BTreeMap::iter`/`keys`). If only the contract is needed, say so in `Body:`.
3. Test `tests/compiler/tests/a_set_is_walked_by_iter.nika` + `.stdout`, through `nikaia test --project tests/compiler` (output to a file, grep it). Rows: on a `BTreeSet[String]` `iter().any`, `iter().all`, `iter().filter(..).count()`, `iter().map(fn(s) { s.clone() }).collect()` into a `Vec[String]` printed (sorted order), `iter().rev()` first element; on a `HashSet[i64]` `iter().count()` and a sum (order-free output); the K3a form `named.iter().any(fn(name) { left_out.contains(name) || absent.contains(name) })` over three `BTreeSet[String]`. Then the K3a form once through `target/fast/nikaia lower` in a scratch file and compiled by rustc (as the k3a2 report probed), not committed.
4. `lower-std` fixpoint (no diff, or the diff committed), the gate of `.claude/agents/builder.md`. Commit `Fixes #1006`.

## Notes

* Read `docs/builder-facts.md`; `CLAUDE.md` rules for builders; report form of `docs/architect.md`, *Briefs and reports*. Changelog: one sentence (std gains `iter()` on both sets).
* Do not touch: the uncommitted files `git status` shows at start (`crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`, the k1/k3a1 drafts, other reports, `docs/briefs/released.txt`): another session's.
* `HashSet` also lacks `union`/`intersection`/`difference`: not this brief (ADR-collections-2 section 6).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1006-set-iter.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
