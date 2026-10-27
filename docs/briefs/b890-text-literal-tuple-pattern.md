---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: a text literal in a tuple pattern (Fixes #890)

Score: bugs lane (`docs/priorities.md`, bugs-first default; #890 is a checker-passes/rustc-fails bug from rs2nika lane L, plus the narrowing rule the architect decided on 2026-10-27, ADR-352 D1 as amended in this commit).

## Issue text (#890)
```nika
fn k1(a: String?, b: i64) -> String {
    return match (a, b) { ("out", 1) => "o1", (_, _) => "rest" }
}
fn k2(a: String, b: i64) -> String {
    return match (a, b) { ("out", 1) => "o1", (_, _) => "rest" }
}
fn k3(a: String?, b: String?) -> String {
    return match (a, b) { ("out", s) if s != null => s, (_, _) => "rest" }
}
fn main() { println(k1("out", 1)); println(k2("out", 1)); println(k3("out", "s")) }
```
`nikaia check` passes all three. In rustc all fail with E0308: a text literal at a `String` position of a tuple pattern is written as a `&str` literal pattern over a `String`, and at a `String?` position also without `Some(..)`. A text literal at the top level of a `match` over a `String` works (`text_scrutinees`, #704), so tuple positions do not get the same treatment.

**Architect's decision (step 3):** a literal at a position counts as "the same part", as `null` and a variant do. ADR-352 D1 now reads "...or the same `null`, variant or literal this arm has there": after `(false, null)`, `(false, why)` binds `why` as the `T`. Sound: a literal is one fixed value, so the earlier unguarded arm took `null` at that position for exactly the values the later arm meets. Part I's table row (`10-nikaia-light.md:1198`) is general and stays.

## Locations
Emit: `crates/nikaia-std/src/tools/emit_core_match.nika:216` `emit_core_pattern_written`, `:341` `emit_core_arm_pattern_written`, `:834`/`:853` (`text_scrutinees`, how a top-level text `match` is lowered); the fact is filled in the checker (`nikaia run scripts/where.nika -- text_scrutinees`). Checker: `crates/nikaia-std/src/tools/check_core_flow.nika:617` `check_core_flow_same_part` (only `null` and paths today), callers `:659`, `:805`.

## To build
1. Emit: a text literal at a tuple position over `String` or `String?` matches through a view of that position (for example the scrutinee lowered as a tuple of views `(a.as_deref(), b)` / `(a.as_str(), b)`, so the literal is a `&str` pattern and `Some("out")` at a `String?` position), the same idea as the top-level text `match`. Bindings at other positions keep their types (k3's `s` is still handed back by value as d89aba7 does). Your choice of form; say it in `Body:`.
2. Test `tests/compiler/tests/a_text_literal_in_a_tuple_pattern.nika` with `.stdout`: k1, k2, k3, each with a matching and a non-matching call, and a nested tuple `(("out", n), m)` if the checker accepts it.
3. Checker: `check_core_flow_same_part` also says true for two equal literals (text, integer, bool; compare the literal values, not the spelling `0x1`/`1` if the AST keeps the value). Test `tests/compiler/tests/a_literal_is_the_same_part.nika` with `.stdout`: `match (flag, why) { (false, null) => "a", (false, w) => w, (true, _) => "c" }` over `(bool, String?)` runs with `w` used as `String` (`w.len()`), and a `.stderr` test `a_different_literal_is_not_the_same_part.nika` where `(1, null)` before `(2, w)` leaves `w` a `T?` (`w.len()` refused, NK1125 or what the checker says today). Last commit `Fixes #890`.

## Done when
The new tests; `nikaia test tests/compiler --both-settings`; the guard set of `.claude/agents/builder.md`; `lower-std` unchanged or changed only where a tools file used the form. Changelog: two lines (step 1 the lowering; step 3 "a literal at a tuple position counts as the same part for narrowing (ADR-352 D1)": a language change, the supervisor raises the version with ADR-352).

## Stop and report if
An open design choice; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b890-text-literal-tuple-pattern.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
