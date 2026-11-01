---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
effort: medium
---
# Brief: #1126, `let mut (a, b) = …` is a parse error, though NK1307 tells the user to write it

Blocks: S1 (main lane, brief not written yet: its draft from converter brief cv3 writes `let mut (at, depth, start) = …` for `sysroot.rs` `shared`)

Score: compiler bug; two messages send the user in a circle, and the form the spec's help names is refused. Location not traced: medium.

Facts: Where things live, Traps already met, Commands

## Issue #1126

```nika
fn main() {
    let mut (at, depth) = (0, 0)
    at += 1
    depth += at
    println(f"{at} {depth}")
}
```

Expected `1 1`. Got (checked by the architect on `target/fast/nikaia` at c60babc2) a parse error: "`mut` must be attached to each individual binding … Add `mut` to each binding." Written `let (mut at, mut depth) = (0, 0)` it is NK1307, whose help says "Write `mut` in front of the whole pattern, which marks every name in it: `let mut (tx, rx) = …`, `for mut (a, b) in pairs`." `for mut (a, b) in [(1, 2)] { a += b; println(f"{a}") }` runs (`3`) but warns "Variable does not need to be mutable" on the `mut` though `a` is assigned.

## Locations

The message text is not in the repository's sources (the architect searched `crates/` and `~/.cargo/registry`): it is built from pieces or comes from the parser's expected-token report. Find the let-statement rule with `nikaia run scripts/where.nika -- --outline` on the parser files (`crates/nikaia/src/parser*`, or the grammar the parser is generated from) and compare it with the `for mut <pattern>` rule, which accepts the form. The unused-`mut` warning: `nikaia run scripts/where.nika -- unused_mut` or the warning's text.

## To build

1. Parser: `let mut <tuple pattern> = e` marks every name of the pattern mutable, as `for mut (a, b)` does; lowering `let (mut a, mut b) = …` in Rust.
2. The unused-`mut` warning does not fire on `let mut (…)` / `for mut (…)` when a name of the pattern is assigned.
3. Test `tests/compiler/tests/a_let_mut_tuple_marks_every_name.nika` + `.stdout`: the probe and the `for mut (a, b)` row, no warning. Fails on the unfixed binary. `a_let_tuple_with_one_mut_name_is_not_supported_yet` stays as it is.
4. `lower-std` and its fixpoint if a `tools/*.nika` changed.

Commit `parser: let mut (a, b) marks every name mutable (Fixes #1126)`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b29-1126-let-mut-tuple.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
