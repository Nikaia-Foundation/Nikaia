---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #859 steps 3 and 1, `T::kind` as a `meta::Type`, and a reflected field's `.ty`

Score: owner's direction (2026-10-31): #859 goes first now, because #1019
(ADR-grammar-2, the typed grammar entry `Json::read[T]`), `json::write[T]`, the
schema check and later `yaml`/`toml` all wait on it. Steps 4 (`match` on a kind)
and 2 (a type as an argument) are the next run's.

## Problem (probed at 1b1ce4d with `target/fast/nikaia check`)
```nika
struct User { name: String, age: i64 }
fn show[T: Struct](v: T) {
    for field in T::fields {
        let k = field.ty            // today NK1180: "A field from `T::fields` has no `ty`"
        println(field.name)
    }
}
fn main() { show(User { name: "a", age: 1 }) }
```
`User::kind` is `NK1171` ("`User` has nothing called `kind`"). Both are
specified and unbuilt.

## Decision (nothing new; the build of accepted records)
* [ADR-330](../specification/adr/adr-330.md) D2, D3 (with [ADR-332](../specification/adr/adr-332.md) D1: a function type's kind is `Fn(F)`), D5, D6, D8;
  [ADR-329](../specification/adr/adr-329.md) D1 as changed by ADR-330 D5; Part II 10.3, Part I 4.7.
* `T::kind` is a `meta::Type`: `I64`, `I32`, `U8`, `U64`, `U32`, `F64`, `Bool`,
  `Text` (String, Bytes is `Bytes`), `Scalar`, `Maybe(A)`, `List(A)`, `Map(K, V)`,
  `Array(A, N)`, `Tuple(parts)`, `Struct(T)`, `Enum(T)`, `Opaque(T)`, `Fn(F)`;
  the parts are **types**. `ref A`, `Shared[A]`, `SharedMut[A]` have `A`'s kind.
* `T::name`, `T::access` (D8) as far as step 3 of #859 lists them; `T::doc` is step 6, not yours.
* `field.ty` **is the field's type** (a type, not a `meta::Type`): usable as
  `field.ty::kind` and later as a type argument.
* A `meta::Type` exists only while building: one that would reach the running
  program is `NK1231` (specified code, Part III C.3; take it, no new code).

## Locations
* `NK1180` and the members a reflected field answers: `crates/nikaia-std/src/tools/check_buildtime.nika:193-207`.
* `T::fields` / `T::variants` as a list, and what a `Ty::member` path resolves to: `check_core_expr.nika:455-470`, `check_members.nika:230-245`.
* Unrolling a `for` over `T::fields` in the emitter: `emit_misc.nika:198`.
* Build-time evaluation of the unrolled body: `crates/nikaia/src/build_time.rs` and `check_buildtime.nika`.
* Where `meta::` would be declared: follow how `T::fields`' element type is named today (`check_core_expr_shape_element`).

## To build
1. **Step 3:** `T::kind` (and `T::name`) for every kind of D3's table, as a
   build-time value; `meta::Type` with its variants; `NK1231` where one would
   reach run time. Tests in `tests/compiler/tests/`: `type_kind_of_each_type.nika`
   (+ `.stdout`, prints a name per kind for a struct, enum, `i64`, `String`,
   `Vec[i64]`, `i64?`, `HashMap[String, i64]`, a tuple, `fn(i64) -> i64`; reach the
   name through whatever build-time form exists today, e.g. `T::name` or an
   `if`/`==` decided while building, without needing step 4's `match`); and
   `type_kind_kept_at_run_time.nika` + `.stderr` (`NK1231`). Commit
   `check: T::kind is a meta::Type; T::name (ADR-330 D2, D3, D6, #859 step 3)`.
2. **Step 1:** `field.ty` is the field's type; `field.ty::kind`, `field.ty::name`
   work in an unrolled `for`. `NK1180`'s help text names `.ty`. Test
   `reflected_field_ty.nika` + `.stdout` (the probe above, printing each field's
   name and `field.ty::name`). Commit `check: a reflected field's .ty is its type
   (ADR-330 D5, ADR-329 D1, #859 step 1)`. No `Fixes` (steps 2, 4-8 remain): write `Part of #859`.

If printing a kind's name needs `match` on a kind (step 4) and no other
build-time form can show it, stop after what is verified and say so under
`Needs:`: do not build step 4 half.

## Done when
Both tests pass through `nikaia test`; each new test was run once against the
compiler without your change and failed there (say so under `Tests:`);
`lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the gate of
`.claude/agents/builder.md`; pushed. Changelog: two lines (`T::kind` and
`T::name` exist, a `meta::Type` only while building; a reflected field's `.ty`
is its type).

## Stop and report if
The parser needs a change for `T::kind` or `field.ty::kind` (say which form);
an edit is needed outside `crates/nikaia-std/src/tools/check_*.nika`,
`emit_*.nika`, `crates/nikaia/src/build_time.rs`, `std`'s declarations of
`meta`, their lowered files and the new tests; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked
at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in, is not done.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b859a-type-kind.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone), Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, Rs2nika (`none`). `none` where nothing, as the bare word. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
