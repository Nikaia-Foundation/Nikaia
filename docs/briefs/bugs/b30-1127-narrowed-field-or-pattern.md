---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
effort: medium
---
# Brief: #1127, a `T?` field narrowed by a `null` arm (#886) reaches rustc as `Option` inside an or-pattern

Blocks: S1 (main lane, brief not written yet: `sysroot.rs` `facade` matches `Item::Fn { name: Some(name), .. } | Item::Enum { name, .. }`)

Score: compiler bug, checker accepts and rustc refuses (an accepted program must build). Location suspected, not traced: medium.

Facts: Where things live, Traps already met, Commands

## Issue #1127

```nika
enum Item { Fn { name: String?, public: bool }, Enum { name: String, public: bool }, Other }
fn names(items: ref Vec[Item]) -> Vec[String] {
    let mut out: Vec[String] = []
    for item in items {
        let name = match item {
            Item::Fn { name: null, .. } => continue,
            Item::Fn { name, public: true } | Item::Enum { name, public: true } => name.clone(),
            else => continue,
        }
        out.push(name)
    }
    return out
}
fn main() {
    let v = [Item::Fn { name: "a", public: true }, Item::Fn { name: null, public: true }, Item::Enum { name: "e", public: true }, Item::Other]
    println(names(v).join(","))
}
```

Expected `a,e`. Got (architect, `target/fast/nikaia` at c60babc2) rustc E0308: `&Option<String>` against `&String`, "a binding must have the same type in all alternatives". Without `| Item::Enum { name, public: true }` it prints `a`: #886's rule works for a single pattern.

## Locations

Suspected, not traced: the emitter's struct-pattern writer applies #886's `Some(name)` only to a whole arm that is not an or-pattern, or the checker records the narrowed position (`CheckFacts::set_positions`, `check_facts.nika`) for the arm and not per alternative. Resolve both with `nikaia run scripts/where.nika -- set_positions` and the callers of the struct-pattern writer in `crates/nikaia-std/src/tools/emit_*.nika`. `emit_core_leaves.nika` holds another agent's uncommitted edit: do not stage it.

## To build

1. Each alternative of an or-pattern gets #886's narrowing on its own: the `Item::Fn` alternative writes `Some(name)`, the `Item::Enum` one `name`.
2. Test `tests/compiler/tests/a_narrowed_field_in_an_or_pattern.nika` + `.stdout`: the probe. Fails on the unfixed binary.
3. `lower-std` and its fixpoint.

Commit `emit: a narrowed field in an or-pattern alternative is Some(..) (Fixes #1127)`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b30-1127-narrowed-field-or-pattern.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
