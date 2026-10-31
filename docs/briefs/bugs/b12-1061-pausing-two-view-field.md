---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1061, a struct field holding pausing code with two views in and a view out fails in rustc

Score: compiler bug, false acceptance (`nikaia check` passes, rustc *Lifetime may not live long enough*); found by this lane's b11 (#1048 row d), cause traced by b11, two places to change.

Facts: Where things live, Traps already met, Commands

## Issue #1061 (copied; probed by the architect on the suite binary at 2831276b)
```nika
fn longer(a: ref String, b: ref String) -> ref String {
    if a.len() >= b.len() { return a }
    return b
}
struct Holder { f: fn(ref String, ref String) -> ref String }
fn main() {
    let h = Holder { f: longer }
    println(f"{h.f("abcd", "abcd").len()}")
}
```
Rows:
* a: the probe - rustc *Lifetime may not live long enough* at line 7 (the NK1261 warnings are expected; `String` without `ref` is refused NK1106 at the field today, not yours). Must print `4`.
* b (prints today, must stay): a struct with such a field that it builds and never calls, printing `1`.
* c (runs today, must stay): row a with the field written `... -> ref String sync`.
* d (must stay): `tests/compiler/tests/a_function_type_with_two_views.nika` (b11's rows a-c and the sync list row).

Syntax: a field holding code is called `h.f(a, b)` (`(h.f)(..)` does not parse); struct fields are separated by commas. An unfixed binary for before/after probes: `/home/user/nikaia-suite/target/fast/nikaia`.

## Suspected location (traced by b11, sure)
Two places repeat the #703 rule *a function type handed two views or more that gives back one hands back the call's own view*, and both say `sync` only. Change both, in one commit:
* `crates/nikaia-std/src/tools/views.nika:425` `views_the_call_s`: drop `if !code.is_sync { return false }`, so `holds_view` stops giving the struct an `'a` for such a field. Its callers: `holds_view` (`views.nika:411`) and `borrowing` in `emit_tables.nika` (`nikaia run scripts/where.nika -- views_the_call_s`); check each still means what it says for pausing code.
* `crates/nikaia-std/src/tools/emit_core_types.nika`, the kept branch (`if !runs`, the `!code.is_sync && lent` block, about :213-221): today `Kept<dyn for<'call> Fn(&'call str, &'call str) -> BoxedIn<'call, &'a str>>`; when the result is one of the lent views (`views_the_call_s` holds, or the same count the `runs` branch uses since b11), write the outcome with `EmitLifetimes::Call`: `BoxedIn<'call, &'call str>`. Lists through `a_list_read_as_a_slice`, as b11 did.
Changing only the emitter gives *Lifetime parameter `'a` is never used* for row b (b11 tried it).

## To build
1. Before any edit, run rows a-c on the current binary; keep the results under `Tests:`.
2. The fix above. Test `tests/compiler/tests/a_pausing_two_view_field.nika` + `.stdout` (rows a and b, also row a over `ref Vec[i64]` if it fails the same way today; say which under `Tests:`). Commit `emit: a struct field holding pausing code with two views in and a view out names the call's lifetime (Fixes #1061)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings` (by name); the `tests/compiler` tests whose names contain `function_type`, `two_view`, `code_param`, `kept`, `struct_field` or `lambda` pass (run by name); `lower-std` fixpoint clean (`lowered/views.rs`, `lowered/emit_core_types.rs`, and `lowered/emit_tables.rs` if touched); build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `views.nika`, `emit_core_types.nika`, `emit_tables.nika`, their lowered twins and the test; a `tools/*.nika` of the toolchain changes its lowering through the `views_the_call_s` change in a way rustc refuses (report the file and line); an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b12-1061-pausing-two-view-field.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
