---
agent: builder
model: opus
max_turns: 70
budget_usd: 14
---
# Brief: #1097, `flat_map` with a function that hands back a `Seq` leaves the element untyped

Score: compiler bug, a wrong refusal (NK1245) of a plainly meant form. Found by b18 (#1078, 46824f2b), which built the `U?` arm of `bind` this one sits next to. One function, cheap and certain.

Facts: Tests and the gate, Probes, Checker and views, Lowering and lower-std

## Issue #1097 (probe checked by the architect on `target/fast/nikaia` at 0510bf6f)
```nika
fn main() {
    let lines = ["a;b", "c"]
    let parts = lines.iter().flat_map(fn(l) { l.split(";") }).map(fn(p) { p.len() }).collect()
    print(parts.len())
}
```
Expected `3`. Got NK1245 on `p`.

Cause (by reading, sure): `crates/nikaia-std/std.contracts` `[fn."Seq::flat_map"]` declares `f: fn(T) -> Vec[U]`. In `crates/nikaia-std/src/tools/ty.nika`, `bind` sends the `Vec[U]` pattern to `bind_named`, which binds only against a `Ty::Named` "Vec". The lambda hands back `Seq[ref String]` (`str::split`), so `U` stays unbound.

Decision (architect, on the issue): `flat_map`'s function may hand back anything a `for` walks, a `Vec`, an `Array` or a `Seq`, and `U` is its element. Rust's `flat_map` takes `IntoIterator`, so the lowering does not change.

## To build
1. Reproduce the probe on the unfixed binary, plus a `Vec` result row (`fn(l) { [l.len(), 1] }`) and a named-function row. Keep the results under `Tests:`.
2. Bind `U` from a `Seq[X]`, `Vec[X]` or `Array[X]` result. Where the rule goes is your choice: say which in `Body:`. It can sit in `bind`/`bind_named` as an element binding of a sequence-like pattern against `Ty::Seq`, or the contract can say `Seq[U]` with a `Vec` result fitting it. Do not widen the binding beyond the element of a walkable sequence. The text kinds of the parts must survive into the next lambda, as #1078 made them for `filter_map` (`text_tiers.nika`, ADR-293 D26). If `flat_map` needs the `mapped` rule too, give it, and say so in `Body:`.
3. `lower-std` and its fixpoint.
4. Test `tests/compiler/tests/flat_map_seq_result.nika` + `.stdout` with the three rows, failing on the unfixed binary.
Commit `check: flat_map binds its element from a Seq, Vec or Array result (Fixes #1097)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`, along with the `it` modules that mention `flat_map`/`filter_map` (b18's list: nullable seq_reductions views_kept map_trust templates). Also: `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `ty.nika`, `check_core_args.nika`, `check_core_method.nika`, `text_tiers.nika`, `std.contracts`, their lowered twins and the new test. Or you hit an error in another agent's file. #1098 (named fn and `filter_map` over `iter()` of copy elements) is a separate issue: do not fix it here.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Your prompt holds the facts you need from `docs/builder-facts.md`. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `<full brief path>.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Report path: `/home/user/Nikaia/docs/briefs/bugs/b21-1097-flat-map-seq-result.md.report.md`
