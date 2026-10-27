---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: a slice pattern over a lent field in a tuple; `.to_string()` of a `String` view parameter (Part of #896, steps 1-2)

Score: bugs lane (`docs/priorities.md`, bugs-first default): `nikaia check` passes, rustc refuses; blocks the rs2nika drafts of `attributes` and `seeds` (self-hosting). Steps 3-4 of #896 (std gaps, `filter_map` closures) are a later brief: not yours.

## Issue text (#896, steps 1-2)
### 1. A slice pattern over a borrowed field inside a tuple
```nika
struct A { args: Vec[i64] }
fn f(a: ref A, b: bool) -> String {
    return match (ref a.args, b) {
        ([], true) => "none",
        ([x], true) => f"one {x}",
        else => "more",
    }
}
fn main() { println(f(A { args: [3] }, true)) }
```
`nikaia run`: rustc *Expected an array or slice, found `Vec<i64>`*. The lowering writes `&a.args` where the slice pattern needs `&a.args[..]`. Without `ref`: NK2106 (right). `match (xs, b)` with `xs: ref Vec[i64]` works. Expected `one 3`.

### 2. `.to_string()` of a `String` view parameter, stored in a struct
```nika
struct B { name: String, held: Vec[String] }
fn add(mut bodies: Vec[B], name: String, args: ref Vec[String]) {
    let held: Vec[String] = args.iter().filter(fn(a) { a.len() > 0 }).map(fn(a) { a.clone() }).collect()
    if !held.is_empty() {
        bodies.push(B { name: name.to_string(), held })
    }
}
fn main() { let mut bs: Vec[B] = []; add(bs, "x", ["a"]); println(bs[0].name) }
```
rustc: *lifetime may not live long enough*. `name.clone()` and plain `name` compile. Expected `x`.

## Locations
* Step 1: `crates/nikaia-std/src/tools/emit_core_match.nika:93` `emit_match_subject` (ADR-355 D5: `&(x)[..]` when an arm is a list pattern, but `emit_match_is_list` :146 looks at the top-level pattern only, not at a tuple pattern's positions).
* Step 2: `.to_string()` of text is "the text as is" (ADR-282 D8): fact `text_as_is` set in `crates/nikaia-std/src/tools/check_core_method.nika:513`, emitted in `emit_core_calls.nika:906`. Here the receiver is a view parameter (`name` inferred lent) and the place it goes (a struct field) needs an owned `String`: there the call must make one (`.to_string()` / `String::from`), or the parameter must not be inferred lent.

## To build
1. A tuple subject whose arms put a list pattern at a position lowers that position as a slice (`&(a.args)[..]`), and leaves the others as they were; also a `ref` place matched alone. Test `tests/compiler/tests/a_list_pattern_in_a_tuple_over_a_lent_field.nika` + `.stdout` (probe 1, plus a two-list form `([a], [b])`).
2. `.to_string()` of a `String` view whose value is stored (struct field, `push`, a `let` that escapes) yields an owned `String`. Test `tests/compiler/tests/to_string_of_a_lent_parameter_stored.nika` + `.stdout` (probe 2, plus a direct `return B { name: name.to_string() }`).

## Done when
Both tests (each fails on the unfixed compiler); `nikaia test tests/compiler --both-settings`; `it` filtered on `handed_over text_tiers typecheck`; the guard set of `.claude/agents/builder.md`; clippy clean. One commit per step, each `Part: #896` (the issue stays open for steps 3-4); `Changelog:` one sentence.

## Stop and report if
An open design choice; an error in another agent's file.

Read `docs/builder-facts.md`.

* Not yours (another lane's uncommitted files in the checkout, 2026-10-27): `crates/rs2nika/convert/**`. Never stage them.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b896a-slice-in-tuple-to-string.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
