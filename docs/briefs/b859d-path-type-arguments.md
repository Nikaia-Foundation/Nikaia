---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: type arguments at a call reached by a path (`json::read[Contact](bytes)`, ADR-grammar-2 D11)

Score: owner's direction (2026-10-31): #1019 (`json::read[T]`) waits on exactly this. #859 steps 2 and 8 are built (a104be7, 0.0.1319), but `f[T](…)` is type arguments only where `f` is a generic function **declared in this file**; a callee reached by a path (another module, `std`, a package) still reads `[T]` as a value and gives `NK1230`. That is a defect (kind 3): ADR-grammar-2 D11 names `json::read[Contact](bytes)` itself.

## Problem (probe it with `target/fast/nikaia check` at your start)
Find a generic function in `std` or write one in a second module of a small project (`fn pick[T](n: i64) -> String { T::name }`), then call it as `mod::pick[User](1)` / `std::…::f[T](…)`. Expected today: `NK1230` (a type where a value belongs). Confirm and say what you found.

## Decision (accepted records, no owner decision needed)
* [ADR-330](../specification/adr/adr-330.md) D9 (`f[T]()`: the types fill the type parameters in order; a function's form, a method's `x.f[T]()` is not decided: refuse it as today), as changed by [ADR-grammar-2](../specification/adr/adr-grammar-2.md) D11: `NK1233` only where a value argument's type could have said the type (`id[i64](3)`); none where the brackets are the only source (`json::read[Contact](bytes)`, `parse[Config](text)`). The callee's path does not change the rule: a path to a generic function is the same form as a bare name.
* No new code; `NK1230` and `NK1233` exist.

## Locations
* The checker's test "a generic function declared in this file and no local of that name": `check_core_call.nika` / `check_core_pilot.nika` (a104be7). It must also accept a path whose last segment resolves to a generic function, through the same resolution the checker uses for path calls today.
* The emitter: `emit_core_args.nika` (the copy per type, or `path::f::<T>(…)`).
* Parser: `crates/nikaia/src/parser/mod.rs` already parses `Call{func: Index{base, index}}`; check that `base` may be a path.
* `NK1230` today: `check_core_expr.nika:431`, `check_buildtime.nika:223`.

## To build
1. Type arguments at a call whose callee is a path to a generic function (module of the same project, `std`), with the same filling, copying and `NK1233` rule as a bare name. Tests in `tests/compiler/tests/` (if a test cannot have a second module there, say how you tested under `Tests:`): `type_arguments_on_a_path.nika` + `.stdout` (no warning where the brackets alone say `T`), `type_arguments_on_a_path_warned.nika` + `.stderr` (a path to `id[T](x: T)`, `warning[NK1233]`). Commit `check, emit: type arguments at a call reached by a path (ADR-grammar-2 D11)`, `Part of #1019`.

## Done when
The new tests pass through `nikaia test --project tests/compiler`; each failed once against the compiler without your change (say so under `Tests:`); the four tests of a104be7 still pass; `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the gate of `.claude/agents/builder.md`; pushed. Changelog: one line.

## Stop and report if
An edit is needed outside `crates/nikaia/src/parser/mod.rs`, `crates/nikaia-std/src/tools/check_*.nika`, `emit_*.nika`, the resolution files they call, their lowered files and the new tests; `json::read` itself (that is #1019, not yours); a form beyond ADR-330 / ADR-grammar-2; an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b859d-path-type-arguments.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone), Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, Rs2nika (`none`). `none` where nothing, as the bare word. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
