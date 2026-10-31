---
agent: builder
model: opus
max_turns: 100
budget_usd: 25
---
# Brief: #999, a grammar rule whose result is a tuple mixing `String` with another type refuses the views its action builds (NK1104)

Score: S=3 (a wrong refusal, blocks the `yaml` lane's flow mappings, #928), O=1 M (the yaml lane waits or writes a struct around it), C=5 M (b-tuple tiers of 2f73c24, #929), area bugs. First of this batch: a defect another lane is blocked on.

## Issue #999 (copied; re-probed by the architect at c30b7bc0)

```nika
grammar G {
    rule WS = "" -> { }
    rule Pair -> (String, i64) = k:until(":") ":" n:dec[i64] -> { (k, n) }
    entry rule doc -> (String, i64) = p:Pair eof -> { p }
}

fn main() throws {
    let (k, n) = G::doc("ab:12")
    println(f"{k} {n}")
}
```

Today: `error[NK1104]: This action builds `(ref String, i64)`, but its rule returns `(String, i64)`.` at the `p` of `doc` (line 4). Expected `ab 12`. Also refused the same way (architect, c30b7bc0): `rule Pair -> (String, Value) = k:until(":") ":" n:dec[i64] -> { (k, Value::Num(n)) }` with `enum Value { Str(String), Num(i64) }`. An all-`String` tuple takes the views (2f73c24, #929, test `grammar_a_tuple_result_takes_the_views_its_action_builds`), and so does a struct `{ key: String, value: Value }`.

A **function** with a mixed tuple result already works: `fn pair(s: String) -> (String, i64) { return (s.trim(), 1) }` called with a named argument prints `ab1 1`. So the gap is in the grammar pass of the tier walk (the error names the rule `doc`, which hands on `p:Pair`). Not this brief: the same function called with a temporary argument (`pair(f" ab{1} ")`) fails in rustc; that is #1036, leave it.

## Locations

* `crates/nikaia-std/src/tools/check_function.nika:317`: the NK1104 of a grammar action (`expect_type`, "This action builds ...").
* `crates/nikaia-std/src/tools/text_tiers.nika:806` `a_followed_tuple`, `:866` `tuple_text_words` (2f73c24's tuple tiers; lowered in `tools/lowered/text_tiers.rs`). A rule's result tier is decided a second time by `grammar_pattern_kinds` in `text_tiers.nika`; a label's type comes from `CheckState::pattern_type` (`check_grammar.nika`), see `docs/builder-facts.md` on grammar rustc errors.
* The test beside `tests/compiler/tests/grammar_a_tuple_result_takes_the_views_its_action_builds.nika`.

## To build

1. A tuple result of a grammar rule whose parts mix `String` (or an enum/struct holding views) with other types takes the views its action builds, no `ref` written (ADR-282 D1, D2), as an all-`String` tuple does, also when another rule hands it on through a label (`p:Pair -> { p }`).
2. Test `tests/compiler/tests/grammar_a_mixed_tuple_result_takes_views.nika` with its `.stdout`: the probe above (`ab 12`) and the `(String, Value)` rule beside it, its `Value::Num` matched and printed. Run it against the unfixed compiler first (it must fail with NK1104), then fixed; run the existing `grammar_a_tuple_result_*` and `grammar_*tuple*` tests.

Only `text_tiers.nika`, `check_grammar.nika`, `check_function.nika`, their lowered files (and `lowered.rs`) and the new test are yours. Another file needed: stop, report `partial` with what and why. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

The commit that finishes it says `Fixes #999`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b1-999-mixed-tuple-views.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
