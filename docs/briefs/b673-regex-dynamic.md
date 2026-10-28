---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: `std::regex`, the first package - run-time patterns, no positions (#673 step 1)

Owner's direction (2026-10-28): start with everything on `regex::Dynamic` (the floor of ADR-regex-1 D6), add the rest step by step; leave positions out until ADR-374 is built. Records: `docs/specification/adr/adr-regex-1.md` (D1-D3, D8, D10), Part I 2.6 (the `std::regex` paragraph).

## Facts
* Nothing of `std::regex` exists. `std` wraps crates as its own types (`hashbrown` in `src/map.rs`, ADR-296 D42); no crate type may appear in a program, the ledger or a message (ADR-regex-1 D8).
* Engine: `regex-automata`'s `meta::Regex` (default features; it brings `regex-syntax` and `aho-corasick`), in a new `crates/nikaia-std/src/regex.rs`, `pub mod regex;` in `src/lib.rs`, the dependency in `crates/nikaia-std/Cargo.toml` with a comment in the style of the others there.
* Ledger: `crates/nikaia-std/std.contracts`. Patterns to copy: a `std` type `[type."bytes::ByteSet"]` (:5592), a constructor `[fn."bytes::ByteSet"]`, a method returning a sequence of views `[fn."Bytes::split"]` (`-> Seq[ref(self) Bytes]`), a view result `[fn."Bytes::text"]` (`-> ref(self) String`), an error type `[type."io::IoError"]` (:4383) and `throws = [...]`, a trait `[trait."supervisor::Restart"]`.
* `text::Match` does not exist (ADR-374 is not built; `s.find` still gives `i64?`). This package creates `text::Match` with **only** `m.text() -> ref String` (a view of the searched text); `start`, `end` and `from:` come with ADR-374 and are not added here.

## To build
1. **The type and the engine.** `regex::Regex[E]` with `E: regex::Engine` (a trait), and exactly one engine, `regex::Dynamic`. `regex::dynamic(pattern: String) -> regex::Regex[regex::Dynamic] throws regex::Error`; `regex::Error` carries the crate's message as text (no crate type). The type parameter is there from the start so that a program's type does not change when engines are added (ADR-regex-1 D3).
2. **The calls without positions** (ADR-regex-1 D3's table, minus `start`/`end`/`from:`), each on `regex::Regex[regex::Dynamic]`:
   * `re.is_match(s: String) -> bool`
   * `re.find(s) -> text::Match?` (the leftmost-first match; `m.text()` is its text, a view of `s`)
   * `re.find_all(s) -> Seq[text::Match]` (left to right, not overlapping)
   * `re.captures(s) -> regex::Captures?`, `c.group(i: i64) -> text::Match?` (0 is the whole match; a group that did not take part is `null`; an `i` past the groups is `null`), `c.named(name: String) -> text::Match?`
   * `re.replace_all(s, with: String) -> String` (`$1`, `${name}` as the crate's interpolation does)
   * `re.split(s) -> Seq[ref String]` (the pieces between matches, views of `s`)
   `Regex` is read-only and shared between threads: say so in the ledger as the other shared `std` types do.
3. **Tests in Nikaia** in `tests/compiler/tests/` (`.nika` with `.stdout`, a refusal with `.stderr`): one test per call above; a multibyte text (`"äöü 12 €"`) for `find` and `split`; `find_all` not overlapping (`"aaaa"` with `aa`); a group that did not take part; `named`; `replace_all` with `$1` and `${name}`; a bad pattern (`"("`) caught as `regex::Error` and its message printed; a literal pattern passed to `dynamic` (allowed).
4. Add `"regex"` to the module list of `every_std_module_is_in_the_shipped_ledger` (`crates/nikaia/tests/it/contracts.rs` :814) - an existing Rust test changed to keep it true, no new one.

A compiler defect met (a generic `std` type with a bound the ledger cannot describe, a view result it cannot type, a `Seq` of a `std` struct) stops the slice: fix it in the compiler as its own commit with a test if it is one step, else report it under `Needs:` with a probe of a few lines. Do not route around it - no `i64` positions, no `String` copies instead of views, no type without its parameter.

## Done when
The new tests pass with `target/fast/nikaia test --both-settings --project tests/compiler`; the guard set of `.claude/agents/builder.md`; clippy clean; `lower-std` unchanged or at its fixpoint. Commit `std::regex: run-time patterns on regex-automata, without positions (#673 step 1)`, `Part of #673`. Changelog: one line for the supervisor's heading under `Changelog:` (what a program can now write).

## Stop and report if
An open design choice beyond this brief (the shape of a call, a name); a `.stderr` outside your tests that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; delete `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b673-regex-dynamic.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone), Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, `Rs2nika: none`. `none` where nothing, as the bare word. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
