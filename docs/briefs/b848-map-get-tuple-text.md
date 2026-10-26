---
agent: builder
model: sonnet
max_turns: 80
budget_usd: 12
---
# Brief: `m.get(k) ?? (tuple)` for every tuple that holds text (#848)

Score: S=3 (a correct program reaches rustc and fails there), O=0.3 M, C=5 M (#846's fix 6aafa677 was one std file and one test), bug lane 1 (`docs/priorities.md`).

## Issue #848 (copied)
Follow-up to #846. Fix 6aafa677 covers only `(String, Copy)` and `(String, String)` pairs. Probes (both: `use std::collections`, `m: collections::HashMap[String, T]`, `m.insert("a", ...)`, `let v = m.get("b") ?? (...)`):
* `T = (i64, String)` with `?? (0, "none")`: rustc "Mismatched types. Expected reference `&(i64, String)`, found tuple `({integer}, _)`".
* `T = (i64, String, String)` with `?? (0, "none", "n")`: the same for `&(i64, String, String)`.

To build: 1. Generalise the fix to every tuple that holds text. 2. One `tests/compiler/` test per form.

## Locations
* `crates/nikaia-std/src/index.rs`, trait `Or` (about line 679) and the two #846 impls (`impl<'a, B: Copy> Or<(&'a str, B)> for Option<&'a (String, B)>` and the `(String, String)` one, about lines 737-754). Doc of the rule: ADR-293 D28 (the read is lent; text becomes a view, a copy value is copied).
* Existing test: `tests/compiler/tests/map_get_tuple_fallback.nika` + `.stdout`.

## To build
1. Probe matrix first, each as a `tests/compiler/tests/map_get_tuple_fallback_<form>.nika` + `.stdout` printing every element of the found and the fallback tuple: pairs `(i64, String)`, `(bool, String)`; triples `(String, i64, String)`, `(i64, String, String)`, `(String, String, String)`, `(i64, i64, String)`. Run each with `target/fast/nikaia run` against today's binary to see the failure.
2. Fix at the root, the same way as #846: one `Or` impl per arity and text position, written by a `macro_rules!` in `index.rs` (arity 2 and 3; every position either `String` → `&'a str` or a `Copy` parameter). Check coherence with the existing generic impls (`cargo check -p nikaia-std`). If the emitter, not `std`, turns out to be the root (it chooses the tuple of views), stop and report under Needs.
3. `cargo build --profile fast -p nikaia`; run the new tests and the old one at both settings (`target/fast/nikaia test --both-settings --project tests/compiler`, or the single files); gate; commit `std: m.get(k) ?? (tuple) answers views for every tuple that holds text` with `Fixes #848`.

## Files you may change
`crates/nikaia-std/src/index.rs`, the new tests. Nothing in `crates/nikaia/src` unless step 2's stop rule says so.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b848-map-get-tuple-text.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
