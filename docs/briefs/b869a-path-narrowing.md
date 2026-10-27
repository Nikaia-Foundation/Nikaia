---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #869 steps 1, 2, 4: a `null` test narrows a path of fields (ADR-382 D2, D3, D5)

Score: S 2 (builds a decided, specified rule, 0.0.1227; today `if p.name != null { p.name.len() }` is a false NK1125), O 1 M (the converter's `if let Some(x) = &self.f` gap, step 6 later), C 8 M (#655 check+lowering runs), bugs lane (no open compiler bug is `ready` besides #871).

## Issue text (#869)
ADR-382 (`docs/specification/adr/adr-382.md`, read D1-D5 first; Part I `10-nikaia-light.md` ~:592) decided that a `null` test narrows a path of fields from a local or a parameter, not only a name. Probe (must run after this brief):

```nika
struct P { name: String?, age: i64 }
fn main() {
  let mut p = P { name: "a", age: 1 }
  if p.name != null { p.age = 2; print(p.name.len()) }   // sibling write keeps it
}
```

## Locations
* Check: `crates/nikaia-std/src/tools/check_narrow.nika` (the issue's `check/narrow.rs` does not exist): proofs keyed by name in `check_narrow_tested` :76, `check_narrow_proved_names` :93, `check_narrow_proofs_for` :141, `check_narrow_proved` :202, `check_narrow_read_as_proved` :277, `check_narrow_a_proof_taken_back` :335.
* The refusal: `check_refusals.nika:104-144` (`NK1125`).
* Lowering of a proved read: `emit_core_places.nika`, `emit_expr.nika`, `emit_misc.nika` (`grep -n narrow`).
* ADR-306 D17's path walk (a field is a name while nothing can change it): `contracts/keeps.rs`, reused per ADR-382 D5.

## To build (each its own commit, `Part of #869`)
1. Check: key the proofs by a path (root plus field list), not a name alone; a path is proved in every place D2 lists.
2. D5: the proof ends at an assignment to the path or a prefix, at a `mut` hand-over (a `ref mut self` method, a `mut` argument), and at a lambda, task, `overlap` branch or `select` arm that sees the root. A loop doing one of these ends it for the whole loop. `NK1102` names the write.
3. (issue step 4) Lowering: a use of a proved path is written `.as_ref().unwrap()`, or `.as_mut().unwrap()` where written through.
4. Tests in `tests/compiler/tests/` (`.nika` + `.stdout`/`.stderr`), one each: a path narrowed in each D2 place; a sibling write that keeps the proof (the probe); a `ref mut self` call that ends it; an assignment to a prefix that ends it.
Not in this brief (next brief, b869b): issue steps 3 (`v[i].f` not a path), 5 (`if let` refused), 6 (converter, gaps row, ADR Built cell).

## Done when
Steps 1-4 built, the probe runs; `lower-std` clean; `nikaia test tests/compiler --both-settings`; `it` filters `nullable`, `narrow`, `views_kept`, `match_arms` and the guard set of `.claude/agents/builder.md`. Changelog: one line (a path of fields narrows after a `null` test, ADR-382).

## Stop and report if
D2/D5 leave a case open that the tests need (`Owner:` only if the ADR truly does not answer it), or an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b869a-path-narrowing.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
