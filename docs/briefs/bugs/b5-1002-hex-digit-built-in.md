---
agent: builder
model: opus
max_turns: 70
budget_usd: 12
---
# Brief: #1002, the grammar built-in `hex_digit` (Part II 10.8) is "Undefined rule"

Score: S=3 (a wrong refusal of a specified built-in; found by the yaml lane, which wrote around it), O=0.3 M, C=2 M, area bugs. Cheap and certain.

## Issue #1002 (copied; probe rewritten by the architect and run on 832b07dd+)

```nika
grammar Hex {
	rule WS = "" -> { }
	entry rule one -> String = "x" h:hex_digit eof -> { f"{h}" }
}
fn main() throws {
    println(Hex::one("xa"))
}
```

Today: `error: Undefined rule: 'hex_digit'.` at 3:35. Expected `a`. Part II 10.8 (`docs/specification/20-nikaia-advance.md`, the vocabulary table near line 746) lists `hex_digit` as one hexadecimal digit, value a `scalar`. The checker already knows the name (`check_grammar.nika:80,118`, `bounds_matched.nika:39`, `fold_bound.nika:200`); what lacks it is whoever resolves a called rule name when the grammar is built or lowered (find the message with `nikaia run scripts/where.nika -- "Undefined"` or by the place `digit` is handled beside it).

## To build

1. `hex_digit` resolves to a built-in matching `0-9`, `a-f`, `A-F`, one scalar, as `digit` does for `0-9`; `hex_digit+` gives text as `digit+` does. Then try every other built-in of the 10.8 table with a value or a plain match (`digit`, `alpha1`, `any`, `char`, `raw_ident`, `string`, `multispace0`, `multispace1`, `line_ending`, `empty`, `eof`); any other one refused the same way is the same defect: build it here when it is the same few lines, else name it under `Found:`.
2. Test `tests/compiler/tests/grammar_the_built_in_rules.nika` with `.stdout`: the probe above, `hex_digit+` over `"x1fA"` (prints `1fA`), and one line per built-in step 1 found working or made to work. Run it against the unfixed compiler first; then the tests whose names start with `grammar_` and hold `digit` or `builtin`.

Only the file that resolves the built-ins (its `.nika` and lowered `.rs`, and `lowered.rs`) and the new test are yours; another file needed: stop, report `partial` with what and why. If the fix needs the specification changed (the list is wrong rather than the engine), stop and report `blocked` with `Needs: owner decision`.

The commit that finishes it says `Fixes #1002`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b5-1002-hex-digit-built-in.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
