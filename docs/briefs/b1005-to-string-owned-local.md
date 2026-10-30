---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #1005, `.to_string()` of an owned or number local read as a view

Score: bug lane, a false refusal (S=1) in the text tiers b998 just touched; `let s = ...; return s.to_string()` is a common shape in moves (K3a part 2 builds text this way). Touches `text_tiers.nika` only. Builders have no GitHub: the issue text is copied below.

## Problem

`.to_string()` of an owned local is read by the text tiers as a view, so a `-> String` function that returns it is refused. ADR-282 D8: on text, `.to_string()` answers the receiver's own type (owned text stays owned); of a number it is new text. Found by b998 (f2599c1c, which fixed the declared-number and match-payload cases only).

## Probe (target/fast, develop f2599c1c)

```nika
fn f(n: i64) -> String {
    let s = f"a{n}"
    return s.to_string()
}
fn main() { println(f(3)) }
```
NK1104 at :3 "declared to return `ref String`". Expected `a3`.

```nika
enum BV { Int(i64), Text(String) }
fn g(got: BV) -> String {
    let s = match got { BV::Int(i) => i, else => 0 }
    return s.to_string()
}
fn main() { println(g(BV::Int(4))) }
```
NK2302 at :4. Expected `4` (`s` is an untyped `let` of a number).

## Locations

* `method_kinds`, `crates/nikaia-std/src/tools/text_tiers.nika:4589` (`to_string` of a receiver with kinds -> `viewed(received)`); `bind_a_payload` :1458 (b998 typed scalar payloads there).
* Tests: new `tests/compiler/tests/to_string_owned_local.nika` + `.stdout`.

## To build

1. Probe both rows on a fresh build; also `let s = "x".to_string() + "y"; return s.to_string()` and `let s: String = ...` typed.
2. A local whose kinds are owned (a new `String`: `f"..."`, `+`, `clone`) keeps owned through `.to_string()`; a local whose type is a number, bool or char (also untyped, from its initializer) gives new text. Fix where the local's kinds are formed, not at the report.
3. Test (all rows), `lower-std` fixpoint, the `it` guard set, `text_tiers::` and `views_kept::`. Commit `Fixes #1005`.

## Notes

* Read `docs/builder-facts.md`. D8 rules for text receivers that b998 kept stay: a view stays a view (`to_string_of_a_view_fills_a_string` still holds).
* Do not touch: the uncommitted files `git status` shows at start (`crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`, the k1/k3a1 drafts, other reports): another session's. Changelog: one sentence.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1005-to-string-owned-local.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
