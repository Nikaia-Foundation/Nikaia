---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1107 rest, a view of a type that copies is copied where the value is wanted (ADR-387)

Blocks: none (b26 left #1107's rows 4-5 open for structs that copy; no move waits on it now, C1c's `FnContract` does not copy)

Score: language decision by the architect (ADR-387, 2026-11-01), closes #1107. One mechanism exists at calls and is widened to four more places (effort medium).

Facts: Where things live, Traps already met, Commands

## Problem
ADR-233 D4 lets a view of a value that copies go where the value is wanted only at a call, by a name (`check_arguments.nika` ~:420: `a_view_that_copies` + `unviewed_type(found) == want` + `check_arguments_a_variable`, recorded in `state.facts.copied_args` so the emitter writes the `*`). ADR-387 D1 widens it to every place that wants the value. Probe (`target/fast/nikaia` at e1a88e49, the architect ran it):
```nika
use std::collections

struct C { n: i64 }
struct Holder { c: C }

fn first(xs: ref Vec[C]) -> C {
    let e: ref C = xs[0]
    return e
}

fn pick(m: ref collections::BTreeMap[String, C], k: String) -> C? {
    let c = m.get(k) ?? return null
    return c
}

fn main() {
    let xs = [C { n: 1 }, C { n: 2 }]
    let e: ref C = xs[1]
    let g: C = e
    let h = Holder { c: e }
    let mut kept: Vec[C] = []
    kept.push(e)
    let ys = [3, 4]
    let f: ref i64 = ys[0]
    let i: i64 = f
    let mut m: collections::BTreeMap[String, C] = collections::BTreeMap()
    m.insert("a", C { n: 5 })
    let p = pick(m, "a") ?? return
    print(f"{first(xs).n} {g.n} {h.c.n} {kept.len()} {i} {p.n}\n")
}
```
Today: NK1104 at :8 (`return e`), NK1103 at :19 (`let g: C = e`), NK1106 at :20 (`Holder { c: e }`), NK1103 at :25 (`let i: i64 = f`). `push` and `pick`'s `return c` were not reported, perhaps only because the first errors stop a later phase: check each row alone. Expected after the fix: `1 2 2 1 3 5`. Also the #1107 issue's own rows 4-5 with `C { n: i64 }` (the unannotated `let c = m[k] ?? return null` handed back as `ref C`, and as `C`).

## Locations (suspected, fairly sure)
* `crates/nikaia-std/src/tools/check_expect.nika:115` `expect_type` (the `let` and field refusals, `check_core_places.nika:834`), and `check_arm_leaves.nika:824` `expect_kept_asked` (the `return` at `check_core_leaves.nika:739`, the field at `check_core_places.nika:617/622`): before refusing, accept where `a_view_that_copies(world, found)` (`check_reads.nika:190`) and `unviewed_type(found) == want`, and record the value's `id_of` in the fact the emitter reads for the `*` (`copied_args`, or a new set beside it in `check_facts.nika` if the emitter reads `copied_args` only at calls: find its reader with `nikaia run scripts/where.nika -- copied_args`).
* b26's report (a062b45a): binding the unannotated read past a jump as a view for a copy struct made `let d: C = c` refuse where it compiled before; with D1 that row compiles again, and `check_core_stmt_a_lent_struct` (`check_core_stmt.nika`) drops its `does_not_copy` condition. Do that here.
* A view of a type that does not copy stays refused, with the same codes (ADR-387 D2): keep a row for it.

## To build
1. The widening, the emitter's `*` at each new place, and `tests/compiler/tests/a_copy_view_is_copied_where_the_value_is_wanted.nika` (+ `.stdout`) from the probe above, plus one `.stderr` test row that a non-copy struct is still refused (`let g: D = e` with `D { s: String }`, NK1103). Refused by the unfixed binary, passes `--both-settings`. Then `an_unwrapped_map_view_stays_a_view` (b26's test) still passes, and `cargo test -p nikaia --test it -- coalesce view map_read element_read copy`. `lower-std` and its fixpoint. Commit `check: a view of a type that copies is copied where the value is wanted (ADR-387)`, `Fixes: #1107`.
## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b33-1107-copy-view-copied.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
