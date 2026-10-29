---
agent: builder
model: sonnet
max_turns: 120
budget_usd: 30
---
# Brief b942: a viewed text moved into a struct, variant, tuple or list is kept; NK2305 where the place must own (#944, #942 steps 1, 3, 4)

Score: bugs before moving (`CLAUDE.md`, `docs/priorities.md`); every shape here reaches rustc today (E0505, Part III C.1), and it is the `yaml` pull parser's shape (#928). Decided by the owner, 2026-10-29: ADR-353 D8, D9 (d84a15f). Starts after b945 (`docs/briefs/b945-struct-result-lifetimes.md`) is pushed: check `git log --oneline -- tests/compiler/tests/a_struct_result_viewing_a_local_lives_in_the_callers_keep.nika` shows a commit; if not, report `blocked`.

## Issue text (#942, #944, copied; builders have no GitHub)
**#942 probe** (`nikaia run`, `in.txt` holds `a: 1`):
```nika
use std::fs
struct View { head: String }
struct Cursor { input: String, pos: i64 }
fn parse(t: String) -> View { return View { head: t } }
fn main() throws {
    let text = fs::read_to_string("in.txt", fs::Root::Anywhere)
    let v = parse(text)                         // v views text
    let c = Cursor { input: text, pos: 0 }      // text moves into the Cursor
    println(f"{c.pos}")
    println(v.head)                             // v still used
    println(c.input)
}
```
Today: *Cannot move out of `text` because it is borrowed* (rustc E0505). The same with a tuple `(text, 0)`, a list `[text]`, a variant `E::A(text)`, and inside a function returning `c`: E0505 each. `let moved = text; Cursor { input: moved }` is rustc E0308 (`&String` into `String`): `__keep_frame.put(..)` gives a `&String`. Already built (1b1d740): a text moved into a field, tuple or list is followed there, a change in place of it while `v` lives is `NK2305`.

**Decision** (ADR-353 D8, D9; Part I 6.6, Part III `NK23xx` row): the literal is a variable of the same function (D3). The move counts as a view flowing into the position (ADR-282 D13): the field/element/payload becomes a view of the kept buffer (`EitherText` where text of its own also flows in), program-wide; `text` stays in the frame or the keep as for `let moved = text`. Where the position must hold text of its own (changed in place, or handed to a callee that keeps it, in **any** function) while `v` is still used: **`NK2305` at the move**, D6's wording: headline names the value and the view; labels: the call that made `v`, the move (*`Cursor.input` must hold text of its own*), **the place that forces it** (*`c.input.push_str(..)` in `advance` changes it in place*), the later use; help: hand over a copy (`Cursor { input: text.clone(), pos: 0 }`) or build the value first and take the view from its field (`let c = Cursor { input: text, pos: 0 }` then `parse(c.input)`). `NK1106` and `NK2105` keep their cases where no view lives.

**#944** (prerequisite: the help's second way out is refused today):
```nika
    let mut c = Cursor { input: text, pos: 0 }
    let v = parse(c.input)          // v views c.input only
    c.pos = c.pos + 1               // a different field
```
gives `NK2305` (*`c` is changed here, but `v` may point into its data*): a false refusal; `v` points into `c.input`, not `c.pos` (ADR-283 D13: a refusal stands only on a named buffer).

## Locations (resolved 2026-10-29)
* `crates/nikaia-std/src/tools/view_changes.nika`: `vc_moved_parts` (:913, the parts a literal takes), `vc_through` (:214), the two `NK2305` findings (:968 hand-over, :992 change); its Rust host `crates/nikaia/src/view_changes.rs`, wired in `crates/nikaia/src/check/mod.rs:472`.
* The keep path of `let moved = text`: `crates/nikaia-std/src/tools/buffers.nika` (:401-410, `given_up_inside` :877, `ask.given_up`); emission `tools/emit_keeps.nika`, `tools/emit_stmt.nika:454` (`given_up_unmuted`), `tools/emit_expr.nika:121-165`.
* The tiers (what a position is by what flows in): `crates/nikaia-std/src/tools/text_tiers.nika` (`Expr::StructLit` arms :2330, :3124, :3314); NK1106's code: `tools/check_numbers4.nika:447`, `tools/check_expect.nika:98`.
* Existing tests: `tests/compiler/tests/text_moved_into_a_field_is_watched_while_a_view_is_used.*`; NK2305/NK1106 tests via `nikaia run scripts/where.nika -- NK2305 NK1106`.
* Read `docs/builder-facts.md`; `where.nika` for anything else.

## To build
In the tests, the text is text of its own made without a file (`let text = f"a: {n}"`), not `fs::read_to_string` and not `.clone()` on a literal (`NK1301`).

1. **#944**: a view whose argument is a field path (`c.input`, `c.a.b`) records that path as its source, and a change is refused only where its path and the source path overlap (one a prefix of the other). Tests: `tests/compiler/tests/a_view_of_one_field_lets_another_field_change.nika` + `.stdout` (the #944 probe, completed as in #942's probe); `tests/compiler/tests/a_view_of_one_field_refuses_a_change_of_that_field.nika` + `.stderr` (`c.input.push_str("x")` while `v` lives). Commit `Fixes #944`.
2. **Moved text kept (#942 step 1)**: the probe runs; so do a tuple, a list, a variant `E::A(text)` and `let moved = text; Cursor { input: moved }`. Test: `tests/compiler/tests/a_viewed_text_moved_into_a_literal_is_kept.nika` + `.stdout`, each printing `v.head` and the moved text. Also the moved form of the struct result (b945 built the view form): `fn make() -> Cursor` that reads a text, views it, moves it into the `Cursor` and returns it, `v` used before the return: `tests/compiler/tests/a_struct_result_with_a_moved_text_lives_in_the_callers_keep.nika` + `.stdout`. Commit `Part of #942`.
3. **Refusal (#942 step 3)**: the probe plus `fn advance(mut c: Cursor) { c.input.push_str("!") }` called on another `Cursor` elsewhere in the program: `NK2305` at the move, with the labels and help above, no rustc message. Test: `tests/compiler/tests/a_viewed_text_moved_into_a_field_that_owns_is_refused.nika` + `.stderr`. The wording follows D6's existing messages (:968, :992); no new NK code.
4. **Nothing else changes (#942 step 4)**: the step 3 program with `v` not used after the move runs (the field owned); F8 (`let c = Cursor { input: v.head, pos: 0 }` then `c.input.push_str("!")`) stays `NK1106` (if no existing `NK1106` test has this shape, add `tests/compiler/tests/a_view_stored_in_a_field_changed_in_place_is_refused.nika` + `.stderr`). Test: `tests/compiler/tests/a_text_moved_into_a_field_that_owns_with_no_view_left_runs.nika` + `.stdout`. Commit `Fixes #942`.

## Done when
Every new test passes and fails on the compiler before your change (say so under `Tests:`), the existing `NK2305`, `NK1106`, keep and `text_moved_into_a_field_*` tests pass, the gate of builder.md (`tools/check_*`/`tools/emit_*` changed: `nikaia test --both-settings --project tests/compiler`), `lower-std` changes only the lowered files of the `.nika` you changed, clippy, `cargo fmt`, `scripts/pre-push-check.sh`. Changelog: one sentence per step.

## Stop and report if
A case needs a rule ADR-353 D8/D9 or Part I 6.6 does not state (`Owner:` names it), the forcing place cannot be found from the check (then build the rest, and say what is missing under `Needs:`), or a file another agent has uncommitted (`git status` first).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b942-moved-text-tiers.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
