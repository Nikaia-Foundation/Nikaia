---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #859 steps 1 and 4, `field.ty` (with the parser form `expr.name::member`) and `match` on a kind

Score: owner's direction (2026-10-31): #859 first; #1019 (ADR-grammar-2), `json::write[T]`, the schema check and `yaml`/`toml` wait on it. Step 3 is built (78897e3, b859a): `T::kind`, `T::name`, `T::kind == meta::Type::X` for the kinds that carry no type. Step 2 (a type as an argument, `schema(User)`) is the next run's: **do not use `fn f[T](T)` in tests**.

## Problem (probed at 79b5ce2)
```nika
struct User { name: String, age: i64 }
fn show[T: Struct](v: T) {
    for field in T::fields {
        println(f"{field.name}: {field.ty::name}")   // parser: "Expected `}` here, but found `:`"
    }
}
fn main() { show(User { name: "a", age: 1 }) }
```
`let k = field.ty` is `NK1180` ("A field from `T::fields` has no `ty`"). And `match T::kind { … }` with arms that carry types does not exist.

## Decision (nothing new; accepted records, no owner decision needed)
* `field.ty::kind` is **written in the specification** (ADR-330 D5: "asked as one (`field.ty::kind`)"; D4's example `schema(field.ty)`, `s::fields`). The missing parse is a defect of kind 3 (CLAUDE.md), not a gap: build it.
* [ADR-330](../specification/adr/adr-330.md) D4, D5, D3 (with [ADR-332](../specification/adr/adr-332.md) D1, `Fn(F)`), D6 (`NK1231`); [ADR-329](../specification/adr/adr-329.md) D1 as changed by D5; Part II 10.3; Part I 4.7.
* **Step 1:** `field.ty` is the field's **type**: `field.ty::kind`, `field.ty::name`, `field.ty::fields` (when that type is a struct), and `field.ty::kind == meta::Type::X` all work inside an unrolled `for field in T::fields`. A path after a member access, `expr.name::member`, parses where `expr.name` is a reflected field's `.ty`; anywhere else it is refused by the checker with an NK code that already fits (say which in `Body:`), never a parse error.
* **Step 4:** `match T::kind { Text => …, List(it) => …, Maybe(a) => …, Map(k, v) => …, Struct(s) => …, Enum(e) => …, Tuple(parts) => …, Fn(f) => …, I64 => …, else => … }`. Arms are written by the variant's bare name (or `meta::Type::X`). A binding in an arm is a **type**: `s::fields`, `it::kind`, `it::name` reachable, and `s` has the bound `Struct`. Every arm is checked under the kind it names; each copy of the function emits only the arm its type takes. The same works on `field.ty::kind`. A `match` on a kind that is not known while building is `NK1231`.

## Locations
* Parser (Rust, winnow-grammar rules): `crates/nikaia/src/parser/mod.rs`; paths `rule path_segment` about :1593, postfix member access near the `Expr::Field` rules.
* Step 3's work to build on: `check_core_pilot.nika` (`check_reflect_path` :374, `kind_compares` :367), `check_facts.nika` (`type_kinds`, `reflected_names`, `kind_compares`), `emit_misc.nika` (the unrolled walk, :198 and step 3's additions), `emit_expr.nika`, `check_function.nika` (copies per type).
* `NK1180` and a reflected field's members: `check_buildtime.nika:193-207` (help text gains `.ty`).
* `match`: `check_core_flow_match` in `check_core_flow.nika:1319`; its emitter in `emit_core_flow.nika`.

## To build
1. **Step 1.** Parser form and `field.ty`. Test `tests/compiler/tests/reflected_field_ty.nika` + `.stdout`: the probe above, plus `field.ty::kind == meta::Type::Text` printed per field. Commit `parser, check: a reflected field's .ty is its type; expr.name::member (ADR-330 D5, #859 step 1)`, `Part of #859`.
2. **Step 4.** Tests: `match_on_a_kind.nika` + `.stdout` (a generic `fn describe[T](v: T) -> String` whose `match T::kind` names `Text`, `I64`, `List(it)`, `Maybe(a)`, `Struct(s)` with `for field in s::fields`, called with a `String`, an `i64`, a `Vec[i64]`, an `i64?` and a struct holding a list); `match_on_a_kind_arm_checked.nika` + `.stderr` (an arm whose body is wrong for its kind, e.g. `Text => it::name` with `it` unbound, refused even when no call takes that arm). Commit `check, emit: a match on a kind checks each arm under its kind and emits the arm the type takes (ADR-330 D4, #859 step 4)`, `Part of #859`.

If step 4 is too large for this run, commit step 1 and as much of step 4 as is verified (tests green), and say what is left under `Needs:`.

## Done when
The new tests pass through `nikaia test --project tests/compiler`; each failed once against the compiler without your change (say so under `Tests:`); `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the gate of `.claude/agents/builder.md`; pushed. Changelog: one line per step.

## Stop and report if
An edit is needed outside `crates/nikaia/src/parser/mod.rs`, `crates/nikaia-std/src/tools/check_*.nika`, `emit_*.nika`, `crates/nikaia/src/build_time.rs`, their lowered files and the new tests; a form beyond ADR-330 is needed; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; delete your own scratch files at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b859b-field-ty-kind-match.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone), Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, Rs2nika (`none`). `none` where nothing, as the bare word. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
