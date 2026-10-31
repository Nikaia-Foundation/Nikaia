---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #859 steps 2 and 8, a type as an argument (`schema(User)`) and `f[T]()` with `NK1233` narrowed

Score: owner's direction (2026-10-31): #859 first; #1019 (`Json::read[T]`, ADR-grammar-2) waits on exactly these two steps. Steps 1, 3, 4 are built (78897e3, 3feae8b, 528c72a; 0.0.1317): `T::kind`, `T::name`, `field.ty::…`, `match T::kind`. Steps 5-7 are not yours.

## Problem (probe it with `target/fast/nikaia check` at your start)
```nika
struct User { name: String, age: i64 }
fn schema[T](T) -> String {
    match T::kind {
        Text => "string",
        I64 => "integer",
        Struct(s) => {
            var out = "{"
            for field in s::fields { out += f"{field.name}:{schema(field.ty)} " }
            out + "}"
        }
        else => "?"
    }
}
fn pick[T](n: i64) -> String { T::name }
fn main() {
    println(schema(User))          // step 2: a type the call names
    println(pick[User](1))         // step 8: no NK1233, nothing else says what T is
}
```
`fn schema[T](T)` is expected to be a parse error today, and `pick[User](1)` too (or `NK1233`-less refused); confirm and say what you found.

## Decision (accepted records, no owner decision needed)
* [ADR-330](../specification/adr/adr-330.md) D1 (a type parameter written in the parameter list is filled by the type the call writes there; the argument is a type the call names, a parameter, `field.ty`, or a type a `match` arm binds; a value there, or a type where a value belongs, is `NK1230`; nothing reaches run time; emitted once per type), D9 (`f[T]()`: the types fill the type parameters in order; a function's form, a method's is not decided), as changed by [ADR-grammar-2](../specification/adr/adr-grammar-2.md) D11: **`NK1233` (a warning) only where a value argument's type could have said the type**; none where the brackets are the only source (`json::read[Contact](bytes)`). `read(Contact, bytes)`, a type in a value's place, stays `NK1230`.
* Part I (10-nikaia-light.md, near :1867), Part II 10.3 (20-nikaia-advance.md :296), Part III C.3 for the codes. `NK1230` and `NK1233` are specified codes: take them, no new code. If `NK1233` is missing from Part III's table, say so under `Found:`.
* The warning's text names #514 (ADR-330 D9).

## Locations
* Parser: `crates/nikaia/src/parser/mod.rs` (winnow-grammar rules): function parameters, and the postfix call / generic-argument rules (`f[T](…)` collides with indexing `a[i](…)`: decide by what the checker knows, or parse both as one form and let the checker tell a type from a value; say which under `Body:`).
* `NK1230` today: `check_core_expr.nika:431` (#704) and `check_buildtime.nika:223` (bare `field.ty`).
* Copies per type and the unrolled walks: `check_function.nika`, `emit_misc.nika`, step 4's match work in `check_core_flow.nika` / `emit_core_match.nika`.

## To build
1. **Step 2.** `fn f[T](T)` parses; a call fills `T` from the type written there (a named type, a type parameter, `field.ty`, a match-arm binding), recursion included (`schema(field.ty)`, `schema(it)`). Tests in `tests/compiler/tests/`: `type_as_argument.nika` + `.stdout` (the `schema` probe above over a struct holding a `String`, an `i64` and a nested struct); `type_as_argument_value_refused.nika` + `.stderr` (`schema(5)` and, in a normal `fn g(x: i64)`, `g(User)`: both `NK1230`). Commit `parser, check, emit: a type as an argument where the parameter list holds one (ADR-330 D1, #859 step 2)`, `Part of #859`.
2. **Step 8.** `f[T](…)` at a call fills the type parameters in order. Tests: `type_arguments_at_a_call.nika` + `.stdout` (`pick[User](1)`, no warning) and `type_arguments_warned.nika` + `.stderr` (`fn id[T](x: T) -> T`, `id[i64](3)`: `warning[NK1233]` naming #514). Commit `parser, check: f[T]() fills type parameters; NK1233 only where a value could say the type (ADR-330 D9, ADR-grammar-2 D11, #859 step 8)`, `Part of #859`.

If step 8 is too large for this run, commit step 2 and as much of step 8 as is verified, and say what is left under `Needs:`.

## Done when
The new tests pass through `nikaia test --project tests/compiler`; each failed once against the compiler without your change (say so under `Tests:`); `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the gate of `.claude/agents/builder.md`; pushed. Changelog: one line per step. Run the existing reflection tests (`reflected_field_ty`, `match_on_a_kind*`, `type_kind_*`) too.

## Stop and report if
An edit is needed outside `crates/nikaia/src/parser/mod.rs`, `crates/nikaia-std/src/tools/check_*.nika`, `emit_*.nika`, `crates/nikaia/src/build_time.rs`, their lowered files and the new tests; a form beyond ADR-330 / ADR-grammar-2 is needed (a method's `x.f[T]()` is not decided: refuse it as today); an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* A copied compiler binary must be named `nikaia` (tests that call the toolchain find it by that name).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; delete your own scratch files at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b859c-type-argument.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone), Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, Rs2nika (`none`). `none` where nothing, as the bare word. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
