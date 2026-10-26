---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 15
---
# Brief: #655 step 4b, an operator on a type that has none is NK1191

Score: bugs lane: S=3 (the checker passes programs only rustc refuses), O=small, C~3 M, area checker (`tools/check_arith.nika`, `tools/check_core_expr.nika`; not the parser, which the brief before this one changed).

## Issue text (#655, item 3 second half, step 4)
`!5`, `!"x"`, `-"x"` and `"a" - 1` pass the checker; only rustc refuses them. To build 4: an operator on a type it does not take is refused by the checker; use an existing code if one fits.

## Correction and decision (architect)
`!5` is right: `!` on an integer flips its bits (ADR-285 D3); it stays. The code is the existing `NK1191` (arithmetic on something that has none), widened; no new code. Probes (architect, 252bcf2, all print `Checked t.nika` today): `let a = !"x"`, `let a = -"x"`, `let a = -true`, `let a = "a" - 1`, `let a = "a" * 2`, `let a = true - 1`, `let a = "a" + 1`. Wanted: each NK1191. Not in this step: `1 < "x"` (a comparison), left for #655.
Replace the `NK1191` row's sentence in `docs/specification/30-nikaia-tooling.md` (grep `NK1191\``) with, verbatim:
> `NK1191` an arithmetic operator, or its compound assignment, on something that has none: text or a `bool` (`"a" - 1`, `true + 1`; `+` joins two texts and nothing else), a list, a map or a set (Part I 4.5), or a `struct` or an `enum` this unit declares - there is no overloading and no trait that gives a type an operator. The same code refuses `-` on anything but a number and `!` on anything but a `bool`, an integer, a vector or a mask.

## An interrupted earlier run (architect, 7280a0b)
The main checkout holds this brief's uncommitted work from a run that ended without a report: `check_arith.nika`, `check_core_expr.nika`, `check_core_ops.nika`, `check_core_stmt.nika` (+ their `lowered/*.rs`, `lowered.rs`), the NK1191 row in `30-nikaia-tooling.md`, and the four tests below. These files are yours: read `git diff` first, keep what is right, finish, and commit them with the rest.

## Locations
* `crates/nikaia-std/src/tools/check_arith.nika`: `arithmetic_on_something_that_has_none` (315; today only collections and declared types), `arithmetic_on_a_declared_type` (~355); add the text/bool case there.
* `crates/nikaia-std/src/tools/check_core_expr.nika`: `check_core_expr_unary` (469): `UnaryOp::Not` returns `Ty::Unknown` silently for a non-bool non-integer (~512-522); `UnaryOp::Neg` (~493) checks nothing. Refuse there with NK1191 (only when the operand's type is known: an unknown type is NK1245's, no second error).
* Callers of the binary check: `check_core_ops_binary` (`tools/check_core_ops.nika:435`). Rust test holding NK1191: `crates/nikaia/tests/it/list_operators.rs` (do not add to it).
* After the change `target/fast/nikaia lower-std`; commit the changed `lowered/*.rs`.

## To build
1. The checks and the spec row above.
2. Tests in `tests/compiler/tests/`: `an_operator_on_text_is_refused.nika` + `.stderr` (`"a" - 1`), `a_minus_on_a_bool_is_refused.nika` + `.stderr` (`-true`), `a_bang_on_text_is_refused.nika` + `.stderr` (`!"x"`), `a_bang_on_an_integer_flips_its_bits.nika` + `.stdout` (`!5` prints `-6`).
Run: `cargo test --profile fast -p nikaia --test it -- operator arith`, `target/fast/nikaia test --both-settings --project tests/compiler`. Commit `Part of #655`.
Changelog: NK1191 also refuses arithmetic on text and `bool`, `-` on a non-number and `!` on a non-bool non-integer.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-operator-types.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
