---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 10
---
# Brief: `.to_string()` of an inferred number local or tuple field, returned as a view (#866)

Score: bug lane 1, the third of a chain (#864, #865) in the same walk; a rustc failure on a plain program. Compiler change: the supervisor raises the version.

## Issue #866 (copied)
Found by the #865 builder (635b59f, report `docs/briefs/b865-to-string-local-view.md.report.md`). Reproduced on develop 79a95b3.

**Probe.**
```
fn f() -> String {
    let p = (3, 4)
    let s = p.0.to_string()
    return s.trim()
}
fn main() { println(f()) }
```
`nikaia run` gives `error: Cannot return value referencing local variable s` (rustc). With `let s = (3).to_string()` or `t.len().to_string()` it runs (#865).

**Why.** `formats` / `formats_a_number` in `crates/nikaia-std/src/tools/buffers.nika` (about lines 535-587) decide without types whether `to_string` makes a buffer of its own: a number literal, arithmetic, `len`, a call declaring a number result, or a local whose *written* type is not text (`formats_local`). A local with no type written (`let p = (3, 4)`) has `ty: null`, so `formats_local` says false, and a field `p.0` falls to `else => false`.

**Scope.** Inferred locals bound to a number expression, and tuple fields of a local bound to a tuple literal. A struct field, a method result, a generic call stay out: they need the checker's types (#851's area), not this walk.

**Locations.** `bind_let` around `buffers.nika:400-421` (where `a_local(..)` is built; `Local` struct line 98, `a_local` line 188); `formats_local` line 582; `formats_a_number` line 547. Test beside `tests/compiler/tests/a_view_of_a_formatted_local_is_returned_owned.nika`.

**To build.**
1. `Local` gets `number: bool` (or the tuple's per-element flags): set in the `let` binding when no type is written and `formats_a_number(value)` holds; for a tuple literal, which elements are numbers. `formats_local` answers true for such a local.
2. `formats_a_number` handles `Expr::Field { base: Variable, name }` where the name is a tuple index and that element of the local was a number.
3. Test `tests/compiler/tests/a_view_of_an_inferred_number_local_is_returned_owned.nika` (+ `.stdout`): `let n = 3` then `n.to_string()`; `let p = (3, 4)` then `p.0.to_string()`; `let q = ("a", 4)` with `q.1` (number) — each returned through `.trim()`. Must fail before the fix. `lower-std`, rebuild, commit `Fixes #866` with the regenerated `lowered/buffers.rs`.

Cases that already pass before the fix stay in the test as guards; say which under Tests, not under Deviation.

## Files you may change
`crates/nikaia-std/src/tools/buffers.nika` (+ `lowered/buffers.rs`), the new test. Anything else: stop and report under Needs.
Known facts: read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b866-inferred-number-local.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
