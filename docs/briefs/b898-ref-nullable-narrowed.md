---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: `x != null` narrows a `ref T?` parameter and a `ref` pattern binding (#898, `Fixes #898`)

Score: bugs lane (`docs/priorities.md`, bugs-first default): Part I (ADR-382) says the proof holds for a parameter; the checker refuses a valid program (NK1102). Blocks the converter's R205 output in the `emit_seeds` draft (self-hosting, three places).

## Issue text (#898)
Part I (ADR-382) says the proof holds "for a local, a parameter, or a path of fields from one". A by-value `i64?` and a field path `n.r` (with `n: ref N`) are narrowed. A **`ref T?` parameter** and a **pattern binding of a `T?` field reached through `ref`** are not:
```nika
struct Ty { k: i64 }
enum E { A { r: Ty? } }
struct N { r: Ty? }
fn g(t: ref Ty) -> bool { return t.k == 1 }
fn f(ret: ref Ty?, n: ref N, e: ref E) -> bool {
    let one = (ret != null && g(ret)) || (n.r != null && g(n.r))   // `g(ret)`: NK1102
    return match e {
        E::A { r } if r != null && g(r) => true,                     // `g(r)`: NK1102
        else => one,
    }
}
fn main() {
    println(f"{f(Ty { k: 1 }, N { r: null }, E::A { r: null })} {f(null, N { r: null }, E::A { r: Ty { k: 1 } })}")
}
```
`nikaia run`: NK1102 `g expects t to be ref Ty, but you're passing ref Ty?` at `g(ret)` and `g(r)`; `g(n.r)` passes. Expected output: `true true`.

## Locations
* `crates/nikaia-std/src/tools/check_narrow.nika:416` `check_narrow_as_proved`: lines 426-432 keep the `T?` on purpose when `inner.is_a_view()` (a `ref T?` is `Option<&T>` in Rust) or `lent_binding` (a name bound out of a lent value is `&Option<T>`): "either is put back into an option slot as it is". That is the cause. Caller: `check_narrow_read_as_proved` :389.
* Lowering of a narrowed read: `crates/nikaia-std/src/tools/emit_expr.nika:122` `emit_expr_narrowed` (bits: 1 copied, 2 operand, 4 lent out of the option, 8 given up, 16 written through). The `Some(` / `Some(&` stripping at :139-155 is the "put back into an option slot" case, which must keep working.
* Existing tests of narrowing: `tests/compiler/tests/*narrow*` and `it` filter `narrow`.

## To build
1. A proved read of a name whose type is `ref T?` (parameter, `match`/`let` pattern binding through `ref`) checks as `ref T`, and lowers to a `&T`: for `Option<&T>` `x.unwrap()` (the `&T` is `Copy`), for a `&Option<T>` binding `x.as_ref().unwrap()`. Put back into an option slot, it is still the option as it is (no `Some(x.unwrap())`). A new bit in `narrowed_reads` if needed (document it in `emit_expr_narrowed`'s comment).
2. Test `tests/compiler/tests/a_ref_nullable_narrowed.nika` + `.stdout` `true true` (the program above), plus in the same file: a proved `ref T?` parameter handed to a `ref T?` parameter and returned as `ref T?` (the option slot case), and a `let` pattern binding through `ref`.

## Done when
The test fails on the unfixed compiler and passes; `nikaia test tests/compiler --both-settings`; `it` filtered on `narrow handed_over typecheck`; `lower-std` (a tools file whose lowering moves is committed with it); the guard set of `.claude/agents/builder.md`; clippy clean. Last commit `Fixes #898`; `Changelog:` one sentence.

## Stop and report if
An open design choice (e.g. a `mut` binding of `ref T?`); an error in another agent's file.

Read `docs/builder-facts.md`.

* Not yours (another lane's uncommitted files in the checkout, 2026-10-27): `crates/rs2nika/convert/**`. Never stage them.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b898-ref-nullable-narrowed.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
