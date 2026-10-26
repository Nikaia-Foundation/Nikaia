---
agent: builder
model: sonnet
max_turns: 80
budget_usd: 12
---
# Brief: a view of a `+` local returned as an owned `String` (#864)

Score: S=3 (a correct program reaches rustc and fails there; met in our own tool `drive.nika`), O=small (one tier rule in `text_tiers.nika`), bug lane 1 (`docs/priorities.md`).

## Problem
Found by the drive.nika builder (`docs/briefs/b-drive-loop.md.report.md`, `Found:`). A function with an owned `String` result that returns a view of a local built with `+` reaches rustc and fails there ("Cannot return value referencing local variable `s`"): the result is lowered as `&'static str` (ADR-282 D29's *lasting* tier), though the local is owned. A correct program must compile (the result is copied out, ADR-282 D2).

## Probe (checked on develop 8335430, `target/fast/nikaia run p.nika`)
```
fn first(t: String) -> String {
    let s = t + "!"
    return s.trim()
}
fn main() { print(first(" hello")) }
```
Lowered: `fn first(t: &str) -> &'static str { let s = nikaia_std::concat::plus(t, "!"); s.trim() }`.

Matrix (same function, other bodies):
| body | today |
| :--- | :--- |
| `let s = t + "!"; return s.trim()` | rustc error |
| `let s = t + "!"; return s.bytes[0..n].text()` (fn `throws`) | rustc error |
| `let s = t + "!"; let r = s.trim(); return r` | rustc error |
| `let s = t + "!"; return s` | runs |
| `let s = t.clone(); return s.trim()` | runs |
| `let s = f"{t}!"; return s.trim()` | runs |
| `let s = t + "!"; let r = s.trim(); return r.clone()` | runs |

So the `+` local is the trigger: its kinds reach the result as *lasting* (probably the `"!"` literal operand), though `tier_expr`'s `Expr::Binary` arm returns `owned_kinds()`.

## Locations
* `crates/nikaia-std/src/tools/text_tiers.nika`: `tier_expr`, arm `Expr::Binary` (about line 3349); `flow_result` (about line 2045); the `let` binding of a local's kinds (`bind_tier_local`, about line 653).
* `crates/nikaia/src/text_tiers.rs`: `mark` (`Tier::Lasting`, about line 261), the adapter.
* `crates/nikaia-std/src/tools/emit_core_types.nika:268`: writes `&'static str` for a lasting view.
* Tests: `tests/compiler/tests/` (program + `.stdout`).

## To build
1. Find where the `+` local's tier becomes *lasting* (`nikaia lower p.nika` after each change; `lower-std` and rebuild when a `tools/*.nika` changes). Fix so a view of an owned local makes the result owned.
2. Test `tests/compiler/tests/a_view_of_a_concatenated_local_is_returned_owned.nika` + `.stdout` with the three failing rows of the matrix, and one row that must stay a static view (`let s = "lit"; return s.trim()` in a function whose result is lasting today - check its lowering stays `&'static str`).
3. Commit `Fixes #864`.

## Files you may change
`crates/nikaia-std/src/tools/text_tiers.nika` (and its `lowered/text_tiers.rs` from `lower-std`), `crates/nikaia/src/text_tiers.rs` only if the adapter is the root, the new test. If the root is in `emit_*.nika` or the checker, stop and report under Needs.
Known facts: read `docs/builder-facts.md`. A `tools/*.nika` change needs `target/fast/nikaia lower-std`, then `cargo build --profile fast -p nikaia`, and the `lowered/*.rs` diff committed.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b864-plus-local-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
