---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a struct field at a `T?` position narrows after its `null` arm (Fixes #886)

Score: bugs lane (`docs/priorities.md`, bugs-first default; #886 is the oldest of the open `ready` checker bugs #886 #888 #889 #890 #891).

## Issue #886 (copied)
Found by the #879 builder. #879 assumed that struct fields already narrow per position. They do not.
```nika
struct F { name: String?, n: i64 }
fn len(v: F) -> i64 {
    match v { F { name: null, .. } => 0, F { name, .. } => name.len() }
}
```
The checker passes this program. rustc then refuses it ("Method `len` is private"): `name` stays an `Option<String>` in the lowered code. ADR-352 D1 is written for patterns in general, so a struct field gets the per-position rule a tuple position has.

**To build**
1. Checker: in a `match`, a bare field binding at a `T?` field binds the `T` once an earlier unguarded arm has `null` at that field and covers the other fields. This is the rule `set_positions` already applies to tuples.
2. Emit: such a field pattern is written `Some(name)`. A variant or literal at that field is matched against the `T`.
3. Tests in `tests/compiler/tests/`: the probe above (also `F { name: x, .. }` with a rename, and a field after `null` of an earlier arm that names other fields `..`), and a field without its `null` arm that stays `T?` (NK1125; its test file: `nikaia run scripts/where.nika -- NK1125`).

## Locations
* `crates/nikaia-std/src/tools/check_core_flow.nika`: `check_core_flow_same_part` :617, `check_core_flow_covers_null_at` :652, `check_core_flow_set_tuple` :673 (writes `state.facts.set_positions` :714), `check_core_flow_parts` :720 (the bindings of a pattern). Build the struct case beside the tuple one; key the fact so the emit can tell the field (a position = the field's index in the struct's declaration order is enough, if the emit has it; say which in the commit).
* `crates/nikaia-std/src/tools/check_facts.nika:275` `set_positions`.
* `crates/nikaia-std/src/tools/emit_core_match.nika:342` reads `set_positions` for tuple arms; the struct pattern writer is in the same file.
* Tools files changed are re-lowered (`docs/builder-facts.md`, lowering).

## Facts
Read `docs/builder-facts.md`.

## Done when
The probe fails before, passes after; new tests pass at both settings; `nikaia test --project tests/compiler` filters `narrow`, `null`, `match`, `tuple` green; the `it` filter for the NK1125 test green. Last commit says `Fixes #886`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b886-struct-field-narrowing.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
