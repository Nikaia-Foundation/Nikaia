---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #996, `return m.get(k)` as `String?` from a lent map of literal text reaches rustc

Score: bug lane, a false acceptance (checker passes, rustc E0308; S=2) in a form every move over a name map writes (`return m.get(k)`); K3a part 2 waits on the owner (#1012). Builders have no GitHub: the issue text is copied below, narrowed by the architect's probes.

## Problem (issue #996, narrowed)

A function that hands back a lent map's text element as `String?` passes the checker and reaches rustc. Probes on a5663bf6:

```nika
use std::collections

fn word(m: collections::BTreeMap[String, String], k: String) -> String? {
    return m.get(k)
}

fn main() {
    let mut w: collections::BTreeMap[String, String] = collections::BTreeMap()
    w.insert("b", "two")
    print(word(w, "b") ?? "none")
}
```
* Lowered today: `fn word(m: &collections::BTreeMap<&'static str, &'static str>, k: &str) -> Option<&'static str> { m.get(nikaia_std::index::as_text(&k)) }`; rustc: expected `Option<&'static _>`, found `Option<&&_>`. The map holds literal text (the `&'static str` tier, which copies), so the read must be copied out: `m.get(..).copied()`. Expected output `two`.
* The same with `let r = m.get(k); return r`: same E0308 (cover it too).
* With an owned value (`w.insert("b", f"two{n}")`) the checker already refuses with NK1102 and the `?.clone()` help: correct, keep it so (it is ADR-293 D15's rule).
* Numbers (`-> i64?`) work: the read is copied.

So the defect: the literal-text tier is a value that copies (`!takes_away`), and the `return`ed (and `let`-then-returned) method-form read does not get its `.copied()`.

## Locations
* `crates/nikaia-std/src/tools/check_reads.nika:91` `a_map_read_kept` (ADR-293 D15): inserts `copied_reads` when the wanted `T?` copies or does not take away; the NK1102 refusal is below it. Check whether the `return` of a method-form `m.get(k)` reaches it, and with which `found`/`want` (`read.is_a_view()` returns early: a literal-text read may be typed as a view there).
* `crates/nikaia-std/src/tools/emit_expr.nika:199`: writes `.copied()` for a node in `copied_reads`; `emit_core_calls.nika:1482`: `.copied()` for `copied_walks`. Check that the method-call path reaches one of them.
* Text tiers (which text lowers `&'static str`): `text_tiers.nika`; use `nikaia run scripts/where.nika -- NAME` for further definitions.
* Lowered twins: `crates/nikaia-std/src/tools/lowered/*.rs` (written by `lower-std` only).

## To build
1. Fix it so both rows lower with the copy (`.copied()` or any lowering rustc accepts and ADR-293 D15 allows) and print `two`; the NK1102 row stays refused. New test `tests/compiler/tests/a_returned_map_read_of_literal_text_is_copied.nika` + `.stdout` (both function forms, one `print` each; take the `.stdout` from the real run), and a refusal test for the owned row only if none exists yet (`nikaia run scripts/where.nika -- NK1102` lists the tests). `lower-std` fixpoint (lower-std, build, lower-std: the second run changes nothing). `Fixes #996`.

## Done when
The new test passes through `nikaia test` (copy it to the scratchpad to `nikaia run` it alone first), the `lower-std` fixpoint is clean, the gate of `.claude/agents/builder.md` is green; pushed. Commit subject: `check: a returned map read of literal text is copied (#996)` (or `emit:` where the fix lies), `Fixes #996`. Changelog: one line.

## Stop and report if
The fix needs an owned `String` map value returned as `String?` without `?.clone()` (that is ADR-293 D15's refusal, not this defect); an error in another agent's file.

**Not yours, never stage**: `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}` and every `docs/briefs/*` file that `git status` shows modified or untracked except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b996-map-read-literal-text.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
