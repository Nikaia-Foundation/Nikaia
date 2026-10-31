---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: C1a, the declared-items loop of `infer_package_noted`, `computed_defaults` and `attribute_values` into `ledger_items.nika` (Part #125)

Score: self-hosting lane, plan row C1 of `docs/selfhosting-plan.md` (first half; the converter blocker the row names, `.entry(k).or_default().insert(v)`, is gone: the draft of these ranges carries one TODO). S=2, C~2-4 M (comparable: P5d, K3a part 2). Touches no file of the bug lane's queue (b13-b18) nor P5c's.

Cut: crates/nikaia/src/contracts/mod.rs:377-763
Cut: crates/nikaia/src/contracts/mod.rs:1073-1191
Convert: --profile check
Shape: schematic callbacks (Rust-held data reached through `fn(..) -> T sync` parameters, as `ledger_defaults.nika`'s `write_result_sources` and `attribute_value`)

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

The driver converts both ranges (checked on 4520d400: 505 Rust lines give a 318-line draft with one TODO, `unwrap_or_default`). The first range is all of `infer_package_noted`; only its declared-items loop (the `for parsed in units` at :398 through the `_ => {}` arm's end, :636, with `declared` :391 and `defaults` :395) moves. The rest of the function (checking each unit, `sync`/`throws`/`touch`/`keeps` inference, `evaluate_defaults`, `attribute_values`' call) stays Rust as it is: delete it from the draft.

## Facts
* **What moves**, byte for byte in what it writes into the `Ledger`:
  - `pub fn declare_items(units: ref Vec[ParsedView], borrowing: ref Vec[collections::BTreeSet[String]], mut ledger: Ledger) -> Vec[ComputedAt]`: the loop. `declared` from `types_declared_in(view.items, view.words)` per unit (already in `ledger_items.nika` :26); `borrowing[u]` is unit `u`'s `borrowing_names`, computed in Rust (`emit::borrowing_structs_beside` takes a `Parsed`, it stays Rust); `ledger.function(..)` is `declared_function` (`declared.nika` :110), `trait_method` is `declared_trait_method` (:250); `impl_parameters`, `declared_extern`, `declared_grammar`, `a_literal` are this file's; `declared_enum`, `tethered_fields`, `declared_fields`, `default_text` are `declared.nika`'s. The mark: an attribute whose `view.unaliased(name)` is `"meta::Attribute"` (`check_attributes.nika` :25 `check_attributes_mark_name`, private: make it `pub`) gives `check_attributes_mark_text(view, attr)` (:93).
  - `pub struct ComputedAt { pub key: String, pub at: i64, pub unit: i64, pub item: i64, pub method: i64?, pub option: String, pub field: bool }` replaces Rust's `Computed<'a>` (plan row: a unit index and the item's position instead of `&'a Expr`; reason: the `Expr` and `Type` live in Rust's `Parsed` arena). `method` is the index in an `impl`'s methods, `null` for a free `fn` and a struct field; `at` is the option's index in `config` or the field's index.
  - `computed_defaults(into, key, unit, item, method, view_item)` as the Rust one, over `computed_options` (`ledger_defaults.nika` :101).
  - `pub fn attribute_values(mut ledger: Ledger, units: ref Vec[ParsedView], library: ref Ledger, computed: fn(i64, ref Ty, ref Expr) -> String? sync)`: the walk (the `At` enum may become the `(kind, place)` pair `apply_attribute` takes); `computed(unit, ty, argument)` is Rust's `computed_literal(units, units[unit], "an attribute's argument", argument, ty, library)`.
* **Stays Rust**: `infer_package_noted`'s signature and everything after the loop; it builds `Vec<ParsedView>` with `check::parsed_view_of` (`check/mod.rs` :902) and the `borrowing` sets, then calls `declare_items`. `evaluate_defaults` and `set_default` (:1205-1250) read `ComputedAt` and find the default and its type by index (`units[unit].program.items[item]`, then the method's or the struct field's or the `config` entry); `computed_literal`, `Ledger::function`, `trait_method` (delete the last two if nothing else calls them; `scripts/where.nika` tells). `lookup` and `candidates` (:809-844) are C1b, not this brief.
* **Callers** (signatures unchanged): `infer_package` (`contracts/mod.rs` :373), `modules.rs` :315. No test in `crates/nikaia/tests/it/` names a moved function.
* **The ledgers must not move**: every `nikaia.contracts`, `nikaia.derived`, `nikaia.proofs` a build writes stays byte for byte; `git diff --stat tests/compiler/nikaia.*` after the test run below shows nothing.
* **#1060 guard**: after `lower-std`, `git status --short crates/nikaia-std/src/tools/lowered/` must show only `ledger_items.rs`, `check_attributes.rs` (if `pub` changed it) and `lowered.rs` as yours; any other lowered file differing from `HEAD` is `Found: #1060` and stops the brief.
* Do not touch: `crates/rs2nika/**`, `project.rs`, `comptime_run.rs`, `sysroot.rs`, `text_tiers.nika`, `docs/briefs/*` but your report, the uncommitted files `git status` shows at start.

## To build
1. `ledger_items.nika` gains the four items above; `contracts/mod.rs` calls them and loses the moved Rust; `lower-std` and its fixpoint; `cargo check -p nikaia`.
2. Tests: `cargo test -p nikaia --test it -- ledger contract default attribute trait impl extern grammar` (anchored `grep -E '^test result:|FAILED|panicked'`), `cargo test -p nikaia --lib contracts::`, and `nikaia test --project tests/compiler --both-settings` on the tests whose names contain `default`, `attribute`, `ledger`, `trait`, `extern`, `mark` (by name); then the ledger check above. Commit `contracts: the declared-items loop, computed defaults and attribute values move into ledger_items.nika (C1a)`, `Part: #125`; plan row C1 updated (C1a built, what stays Rust, C1b = `lookup`, `candidates`, the `stored` fold), self-hosting share in the body (`scripts/self_hosting.py`). Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Done when
Build, `lower-std` fixpoint, the tests of step 2 and the ledger check pass; fmt, clippy (1.97 and 1.99.1), `pre-push-check.sh`; committed locally; `Rs2nika:` with the score.

## Stop and report if
A form Nikaia lacks (`Result: blocked`, `Found:` with a probe); an edit outside `contracts/mod.rs`, `ledger_items.nika`, `check_attributes.nika` (the `pub`), their lowered files, `docs/selfhosting-plan.md`; an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/c1a-ledger-items.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
