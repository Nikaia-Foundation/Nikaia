---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 20
---
# Brief: #655 step 2a, a view of an element as a `Vec` element or a tuple part under `ref E?`

Score: bugs lane (`docs/priorities.md`): the checker passes code rustc refuses (E0308) or refuses what ADR-354 D1 allows (NK1103); one area (element views). Steps 1, 3, 4 of #655 are done (2e1ba5d, 2003b4c, deed9ad, f4dfb8c); 2b (`null` arm) and 5 (lifetimes) are later briefs.

## Issue text (#655, item 1, step 2)
- `w.push((2, given[1]))` into `Vec[(i64, ref E?)]` lowers without `Some`, and rustc refuses it (E0308).
- `let t: (i64, ref E?) = (1, given[0])` is refused with NK1103.
- `v.push(given[1])` into `Vec[ref E?]` lowers to `Some(*get(..))`.
To build 2: a view of an element as a tuple part, a `Vec` element or an `if` branch under `ref E?`: it lowers to `Some(&..)` and is accepted. One test per probe.
The rule is decided: ADR-354 D1, *an index or a field read off a place is a view where the slot declares `ref T` or `ref T?`*; a `Vec` element type and a tuple part are such slots. No new decision, no spec text.

## Probes (all with `struct E { n: i64 }`, `fn f(given: ref Vec[E]) -> i64`, main `let g = [E { n: 1 }, E { n: 2 }]`, `println(f"{f(g)}")`; architect ran them on 2e1ba5d's binary)
* (a) `let mut v: Vec[ref E?] = []` / `v.push(given[1])` / `return v.len()`: rustc *Mismatched types* (`Some(*..)`). Expected `1`.
* (b) `let mut w: Vec[(i64, ref E?)] = []` / `w.push((2, given[1]))` / `return w.len()`: rustc *Mismatched types* (expected `Option<&E>`, found `E`). Expected `1`.
* (c) `let t: (i64, ref E?) = (1, given[0])` / `return t.0`: NK1103 *This value is `(?, E)`*. Expected `1`.
* (d) control, already fine: `let y: ref E? = if c { t } else { given[1] }` prints the element; keep it so.
* (e) the `Vec` of (a) returned from `f` (result `Vec[ref E?]`, two `ref` parameters not involved): refused as a kept view is now, or lowered right; say which in `Tests:`.

## Locations
* The rule as built for a `let` and an arm: `crates/nikaia-std/src/tools/check_core_stmt.nika` `check_core_stmt_element_view` 330 (called at ~503 for a `let`), and `crates/nikaia-std/src/tools/check_arm_leaves.nika` ~616 (an arm under `let`); it marks `facts.lent_reads` (lowered as `&` in `emit_expr.nika` ~221).
* An argument against a parameter's type (the `push` argument, want = the element type): `crates/nikaia-std/src/tools/check_arguments.nika` `check_arguments` ~123 (loop `for argument in expanded`).
* The `Some(..)` wrap of a value into a `T?` slot: `check_overflow.nika` `check_widened_slot` 211 → `facts.nullable_sites`, read by `emit_stmt.nika` (`emit_around_value`, 462-740) and `emit_core_leaves.nika` 230. A tuple literal against a declared tuple type: find it with `nikaia run scripts/where.nika -- NAME` from these.
* Generated files change with each `.nika` (`crates/nikaia-std/src/tools/lowered/*.rs`): `docs/builder-facts.md` says how to re-lower.
* Tests: new `tests/compiler/tests/a_view_of_an_element_pushed_into_a_vec_of_views.nika` (a), `..._as_a_tuple_part.nika` (b and c), each with `.stdout`.

## Done when
Probes (a)-(c) print `1`, (d) unchanged. `nikaia test tests/compiler --both-settings` passes, plus the `it` filters `element_views`, `views_kept`, `stored_views`, `tethers`. Commit `A view of an element as a Vec element or a tuple part is lent (Part of #655)`; no `Fixes`. Changelog line: yes.

## Stop and report if
A choice the ADR does not make (e.g. a view into a `Vec` that is later returned): `Result: blocked`, the question under `Owner:`.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-element-view-slots.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
