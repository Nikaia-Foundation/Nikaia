---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 12
effort: low
---
# Brief: #1130, `drain()` on `BTreeMap`, `HashSet` and `BTreeSet` (ADR-386)

Blocks: none (repays C1b's clone in `ledger_items_stored`; C1c also edits `ledger_items.nika`: if its commit is on develop, add beside it, do not reorder)

Score: std counterpart decided by the architect (ADR-386), self-hosting debt (a clone written round a missing form). Small: one ledger form known (HashMap::drain), effort low.

## Issue #1130

**`drain()` on `BTreeMap`, `HashSet` and `BTreeSet`** (ADR-386, architect's decision 2026-11-01). Only `Vec::drain` and `collections::HashMap::drain` are in `std.contracts` (:2971, :2982); C1b (547e6462) had to clone every set out of a borrowed `BTreeMap` for want of it.

Probe (`target/fast/nikaia` at e1a88e49):
```nika
use std::collections
fn main() {
    let mut m: collections::BTreeMap[String, Vec[i64]] = collections::BTreeMap()
    m.insert("a", [1, 2])
    for (k, v) in m.drain() {
        print(f"{k} {v.len()}\n")
    }
}
```
`NK1171: collections::BTreeMap has no method called drain`. Expected: `a 2`.

Facts: Where things live, Traps already met, Commands

## Locations
* `crates/nikaia-std/std.contracts`: `[fn."collections::HashMap::drain"]` (:2982) is the model; add `BTreeMap::drain` (`Seq[(K, V)] sync ends sized`), `HashSet::drain` (`Seq[T] sync sized`), `BTreeSet::drain` (`Seq[T] sync ends sized`), each taking `self` by value.
* Lowering: `HashSet` to Rust `drain()`; `BTreeMap`/`BTreeSet` have no stable `drain`: `into_iter()`, or `std::mem::take(&mut m).into_iter()` where the container is a place still named after the loop (ADR-386 D2). Find where `HashMap::drain` is lowered with `nikaia run scripts/where.nika -- drain` (emitter, `tools/emit_*.nika`); `crates/nikaia-std/src/map.rs:249` is std's own `HashMap::drain`.

## To build
1. The three ledger entries and their lowering; test `tests/compiler/tests/drain_every_map_and_set.nika` (+ `.stdout`): a `BTreeMap` drained in key order with each value handed to a `Vec` that keeps it, a `HashSet` drained into a sum, a `BTreeSet` drained in order, and a `BTreeMap` field of a `let mut` struct drained and then `len()` 0. Fails on the unfixed binary (NK1171), passes `--both-settings`.
2. Repay C1b's clone: `ledger_items_stored` (`crates/nikaia-std/src/tools/ledger_items.nika:488`) takes each unit's map over with `for (key, code) in unit.drain()` and `extend(code.resolved)` / `extend(code.free)` with no `.iter().cloned()`; `contracts/mod.rs:444`'s caller keeps passing owned maps (adapt to what the lowering asks). `lower-std`, its fixpoint, `cargo test -p nikaia --lib contracts::`, `--test it -- throws ledger contract`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b32-1130-drain-every-map.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
