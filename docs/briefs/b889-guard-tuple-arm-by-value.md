---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a name proved by a guard in a tuple arm with `null` or a literal beside it (Fixes #889)

Score: bugs lane (`docs/priorities.md`, bugs-first default): `nikaia check` passes, rustc refuses (E0308); found by rs2nika lane L building #879.

## Issue text (#889)
```nika
fn g2(a: String?, b: String?) -> String {
    return match (a, b) {
        (x, null) if x != null => x,
        else => "none",
    }
}
fn main() { println(g2("a", null)) }
```
`nikaia check` passes. `nikaia run` fails in rustc with E0308: `x` is lowered as `x.as_ref().unwrap()`, a `&String`, where the result is a `String` handed back by value. The arm `(x, 2) if x != null => x` fails the same way. `(x, _) if x != null => x` works, so the fault is in how the arm's pattern is lowered when another position holds `null` or a literal: the binding is then by reference. Related to d89aba7 (a proved name handed back whole is given up) and #888, but a separate path.

## Locations
* `crates/nikaia-std/src/tools/check_arm_leaves.nika:394` `a_narrowed_read_given_up`, :419 `narrowed_reads_given_back` (d89aba7: `narrowed_reads` +8 marks a proved name given up, `x.unwrap()`).
* Match lowering: `crates/nikaia-std/src/tools/emit_core_match.nika` (`emit_match_subject` :93; a tuple subject with a `null`/literal position is likely matched by reference, `match (&a, &b)` or `.as_ref()`, so the binding is `&String?`); patterns `emit_pattern.nika`.
* Find first, with `nikaia run --keep` or the lowered Rust, which of the two differs between `(x, _)` and `(x, null)`: the subject's form or the narrowed read.

## To build
1. Emit: a name the guard proves, handed back whole from an arm whose pattern has `null` or a literal at another position, is given up by value (`x.unwrap()`, or bound by value), as in the `(x, _)` case. Test `tests/compiler/tests/a_guard_proved_name_beside_null_given_back.nika` + `.stdout`: the probe, the `(x, 2)` form with `(String?, i64)`, and a struct in place of `String`.

## Done when
The new test (fails on the unfixed compiler); `nikaia test tests/compiler --both-settings`; `it` filtered on `handed_over text_tiers typecheck narrow`; the guard set of `.claude/agents/builder.md`; clippy clean. Commit with `Fixes: #889`; `Changelog:` one sentence.

## Stop and report if
Binding by value would move a subject that is read after the `match` (then `blocked` with the lowered forms); an error in another agent's file.

Read `docs/builder-facts.md`.

* Not yours (another lane's uncommitted files in the checkout, 2026-10-27): `crates/rs2nika/convert/**`. Never stage them.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b889-guard-tuple-arm-by-value.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
