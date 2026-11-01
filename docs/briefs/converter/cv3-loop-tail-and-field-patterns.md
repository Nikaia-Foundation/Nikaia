---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 14
effort: medium
---
# Brief: rs2nika writes a loop's trailing `if let` as `?? continue`, a `Some(x)` field pattern with its `null` arm, and `let (mut a, mut b)` as `let mut (a, b)`

Score: converter; the rest of S1's needs (`facade`'s `Item::Fn { name: Some(name), .. }`, `shared`'s `let (mut at, mut depth, mut start)`), and the architect's wanted pattern (5) that P5a/M1/P2 rewrote by hand ("Option matches drafted as `match x { v => .., null => {} }` rewritten as `?? continue`"). Medium.

Facts: Where things live, Traps already met, Commands

## Problems (each probed on `crates/rs2nika/convert/target/nikaia/bin/rs2nika` at c60babc2, `--profile none --file`)

1. A loop body whose last statement is `if let Some(p) = e { body }` with no `else` becomes `match e { p => { body }, null => {} }`. Wanted: `let p = e ?? continue` then `body` at the loop's indentation (`let (k, v) = l.split_once("=") ?? continue` for `Some((k, v))`; Part I's `text::Cursor` example writes this form). Only when it is the last statement of a `for`/`while`/`loop` body; elsewhere the `match` stays (Part I 2.3: `match` binds what a `T?` holds; `if let` is refused).
2. A struct pattern with a field `f: Some(x)` drops the `Some`: `Item::Fn { name: Some(name), .. } | Item::Const { name: Some(name), .. } => …` becomes `Item::Fn { name, .. } => …` (and the or-pattern is split into two arms with the body copied), which also matches a `null` name: a draft with another meaning. Wanted (the #886 form, which runs today): before the arm, an arm `Item::Fn { name: null, .. } => <the body of the match's `_` arm>` for each such variant, then the arm with the bare field and the or-pattern kept as one arm. A match with no `_`/`else` arm writes `TODO(rs2nika): the null arm of a Some(..) field` instead of guessing.
3. `let (mut at, mut depth, mut start) = (0usize, 0i64, true)` becomes `let (at, depth, start) = …`, losing `mut`. Wanted `let mut (at, depth, start) = (0, 0 as i64, true)` (NK1307's help; `docs/selfhosting-gaps.md`). The parser takes that form since 40944720 (#1126, b29).

Pre-flight: run `target/fast/nikaia run` on `tests/compiler/tests/`-style scratch files for the wanted forms of 1 and 2 (single pattern, no or-pattern) and see them print; the or-pattern form of 2 and rule 3's `let mut (..)` run since 1b7fa366 (#1127, b30) and 40944720 (#1126, b29): probe them too, after `cargo build --profile fast -p nikaia` if your binary is older than those commits.

## Locations

`crates/rs2nika/convert/src/tr_stmt.nika` (the `if let` and `let` writers, block bodies of loops) and the pattern writer (`nikaia run scripts/where.nika -- --outline crates/rs2nika/convert/src/tr_stmt.nika` and `tr_expr.nika`); the or-pattern split happens where a `|` pattern of struct patterns is written.

## To build

1. Rule 1, test `crates/rs2nika/convert/tests/loop_tail_if_let.{args,in/f.rs,stdout}`: a `for` and a `while` with a trailing `if let`, one with a tuple pattern, one `if let` not last (stays `match`), one with `else` (stays).
2. Rule 2, test `some_field_pattern.{args,in/f.rs,stdout}`: S1's `facade` shape (an or-pattern of two struct patterns, one `Some(name)`, `_ => continue`), and a match without `_`.
3. Rule 3, test `let_mut_tuple.{args,in/f.rs,stdout}`: all names `mut` (rule applies), one name `mut` (stays as written; NK1307 is right there).
4. `nikaia build` and `nikaia test` in `crates/rs2nika/convert`; `target/nikaia/bin/rs2nika --corpus tests/corpus.in`, the four score lines into `Body:`; a floor rises only to what these rules earned. One commit per rule.

Stop about 15 turns before `max_turns`, commit what is finished, report `partial`.

Commits `rs2nika: a loop's trailing if-let is ?? continue`, `rs2nika: a Some(x) field pattern gets its null arm (#886 form)`, `rs2nika: let with every name mut is let mut (..)`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/converter/cv3-loop-tail-and-field-patterns.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
