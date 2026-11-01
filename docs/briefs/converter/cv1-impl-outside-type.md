---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 10
effort: low
---
# Brief: rs2nika frees the methods of a type the cut does not declare

Score: converter, the costliest pattern of the moves' `Rs2nika:` lines (P4b, P5a, S2, C1, M1, P5c: "the draft wrote methods of `Sysroot`/`Program`/`Settings`/`Workshop`, which stay Rust; every signature and `self.` read redone"). Known form (a53f456 builds it for a declared struct): low.

Facts: Where things live, Commands

## Problem

Under a `-free` profile with `--prefix`, a53f456 writes `impl T { fn m(&self) }` as `fn X_m(t: ref T)`, `self.f` as `t.f`, `self.m(a)` / `Self::m` / `T::m` as `X_m(t, a)`, but only when the input also declares `struct T` (`struct_here`). A move's `Cut:` holds the methods and never the struct (it stays Rust), so the draft keeps `impl T { … ref self … }`, which no `tools/*.nika` can say for a Rust-held type.

Probe (checked on `crates/rs2nika/convert/target/nikaia/bin/rs2nika` at c60babc2), `e.rs`:

```rust
impl Codegen {
    fn get(&self) -> u32 { self.n }
    fn twice(&self) -> u32 { self.get() * 2 }
}
```

`rs2nika --profile check-free --prefix s_ --file e.rs` writes `impl Codegen { fn get(ref self) … self.get() * 2 }`. With `pub struct Codegen { n: u32 }` above it, it writes `fn s_get(codegen: ref Codegen) -> u32 { return codegen.n }` and `return s_get(codegen) * 2`. Expected: the second output for both inputs (without the struct, of course).

## Locations

* `crates/rs2nika/convert/src/tr_items.nika`: `struct_here` (511), `impl_has` (522), the `free` decision in the impl writer (582, `self.struct_here(self.self_ty)`), the method head (345).
* `crates/rs2nika/convert/src/tr_expr.nika`: `impl_has` callers (633: `T::m` calls), 732 (`self.m(..)`).
* Test pattern: `crates/rs2nika/convert/tests/free_impl_tree.{args,in/f.rs,stdout}`.

## To build

1. The rule becomes: a type the input declares as an `enum` keeps its methods (as now); every other `impl T` (not `impl Trait for T`) under `frees_impls()` is freed, whether or not `struct T` is in the input. Change the decision in one place (a helper such as `frees_type(ty)` used by the impl writer and `impl_has`), not by copying the condition.
2. Test `crates/rs2nika/convert/tests/free_impl_outside.{args,in/f.rs,stdout}`: the probe plus a `&mut self` method (`mut codegen: Codegen`), a `Self::helper()` call and an `impl Display for Codegen` that stays an impl. The `.stdout` is what the converter now writes, read line by line against the rule.
3. `nikaia build` and `nikaia test` in `crates/rs2nika/convert` (all its tests; `free_impl_tree` must not change). Run `target/nikaia/bin/rs2nika --corpus tests/corpus.in` there and put the four score lines into `Body:`; raise a floor in `tests/corpus.in/manifest` only to what this rule earned.

Commit `rs2nika: a type the cut does not declare has its methods freed too`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/converter/cv1-impl-outside-type.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
