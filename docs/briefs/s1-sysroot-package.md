---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 10
effort: medium
---
# Brief: S1, the tools package's assembly into `sysroot_package.nika` (Part #125)

Score: self-hosting lane, plan row S1 of `docs/selfhosting-plan.md` (its ranges are stale; the ones below are checked against the tree). About 330 lines of Rust move; the converter it waited on is gone, the move is by hand.

Cut: crates/nikaia/src/sysroot.rs:55-64
Cut: crates/nikaia/src/sysroot.rs:240-247
Cut: crates/nikaia/src/sysroot.rs:326-346
Cut: crates/nikaia/src/sysroot.rs:405-455
Cut: crates/nikaia/src/sysroot.rs:730-816
Cut: crates/nikaia/src/sysroot.rs:879-1035
Cut: crates/nikaia/src/sysroot.rs:1172-1220
Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **What moves**, into a new `crates/nikaia-std/src/tools/sysroot_package.nika` (text only, every function `sysroot_package_`-prefixed, doc comments carried over): `HAND_WRITTEN` (:55-64) as a constant; `Codegen::render` (:240-247) over a `ref BTreeMap[String, String]`; `every_refusal` (:326-346) with a host trait whose body-less `render_finding(ref self, finding: ref CheckFinding, path: ref String, source: ref String) -> String sync` Rust answers with `crate::diagnostics::render_finding` (pattern `project_check.nika:302`; sort by `span.start`, which is `Span::at()`); `without_bodies` with its nested `bodies` (:405-455) over `ref Vec[Spanned[Item]]` (`crate::ast` is `nikaia_std::tools::ast`); `shared` and `raw_string` (:879-1035); `facade` (:1172-1220) over a `ref ParsedView` (`parsed_view.nika:11`, built in Rust by `crate::check::parsed_view_of`); and the assembly of `lower_tools_files_cached` (:730-816: imports hoisted, the grammar flag, `mod {stem}_lowered`, the file header, the sorts, the index preamble) as one function from the file names, the lowered texts and the views to the package's `Vec[(String, String)]`, index first. Reuse `SYSROOT_TOOLS_PACKAGE` / `SYSROOT_TOOLS_FILES` (`sysroot_cache.nika`) and `emit_misc::is_reserved` for `crate::emit::escaped`.
* **Probe** (supervisor, this round, `target/fast/nikaia run`, all run with the right output): `s1_shared.nika` (byte scan with `text.bytes[a..<b].text() catch { ... }`; bind it with `let` before comparing, a `catch` cannot be followed by `==`), `s1_facade.nika` (or-pattern of `Item::Fn{..}`/`Struct{..}` with `receiver: null, is_public: true`, `(name ?? continue).clone()`, `{{` in an f-string), `s1_refusals.nika` (generic host, `filter(fn(f) { ... })`, `sort_by_key(fn(f) { f.span.start })` on a `Vec[ref T]`), `s1_bodies.nika` (`Vec[(i64, i64)].sort()`, a cut inside a `ü` returns the whole source).
* **Rust keeps** one-line forwards with their signatures: `fn render(&self) -> String`, `fn every_refusal(path: &Path, source: &str, refusals: &[&crate::check::Finding]) -> String`, `fn without_bodies(source: &str, parsed: &crate::parser::Parsed) -> String`, `fn facade(file: &str, unit: &crate::parser::Parsed) -> String` (or drop `facade` and `shared` once the assembly is the only caller); `Codegen::new` (:231-238, `toml::Value`); `TOOLS_PACKAGE` (:48, read at :1230); and in the loop of `lower_tools_files_cached` only `timer.file`, `lowered?`, `*reused += ...` before the call to the Nikaia assembly. Untouched callers (`grep -n` in `sysroot.rs`): `render` at :161 and the tests :1574-1575, `every_refusal` at :318 and :708, `without_bodies` at :650.
* **New tools file**: one `lower-std` with the old binary before the `cargo build`, and a row in `scripts/self_hosting.py` beside `sysroot_cache.nika`.
* **#1060 guard**: after `lower-std`, only `lowered/sysroot_package.rs` and `lowered.rs` are yours. `lowered/emit_core_leaves.rs` differs in the tree from another agent's uncommitted `emit_core_leaves.nika`: neither stage nor restore it.
* **Acceptance**: `lowered.rs` byte-identical after `lower-std` with the new binary, but for the new module's own lines (`mod sysroot_package_lowered`, its `pub use`, its facade); every other `lowered/*.rs` unchanged (`git diff --stat`).
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start.

## To build
1. `sysroot_package.nika`, the Rust forwards, `lower-std` and its fixpoint, the acceptance diff. Tests: `cargo test -p nikaia --lib sysroot::`, `cargo test -p nikaia --test it -- sysroot:: selfhosting_defects:: self_hosting::` (anchored `grep -E '^test result:|FAILED|panicked'`). Plan row S1: built, row closed (what stays Rust: `Codegen::new`, the parallel lowering, the cache, `Phases`); self-hosting share in `Body:`. Commit `sysroot: the tools package is assembled in sysroot_package.nika (S1)`, `Part: #125`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s1-sysroot-package.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
