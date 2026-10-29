---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: K3a part 1, `lowered_units`, `without_foreign`, `dependency_modules`, `by_package` of `comptime_run.rs` into `comptime_tree.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`, K rows of `comptime_run.rs`). Plan row K3a (`docs/selfhosting-plan.md`, *Large files 3*) split in two: this part (~150 Rust lines: the assembly of a build-time program of several files) now; part 2 (`compute`, `computed`, `worked_out_ahead`, `undeclared_callee`, `culprit`, 284-595) next, over this part's host. S=1.

Cut: crates/nikaia/src/comptime_run.rs:119-271
Convert: --profile check-free --prefix comptime_
Shape: schematic host-calls (the `[forms comptime_]`/`[hosts comptime_]` rows of `crates/rs2nika/convert/rs2nika.names`, learned P4b, P5b, P5d): every call into what stays Rust (`emit_module_body_at`, `Needs`, `Ledger::infer_package`, `Parsed::keeping`) becomes a host call; bodies, order and texts stay as they are.

The driver converts this before you start and names the draft (`.draft.nika`, `.notes`, `.rs`): start from it. Ranges at 7b36e23 (re-grep `fn lowered_units`, `fn by_package` if `git log -- crates/nikaia/src/comptime_run.rs` shows a later change). **If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read `crates/nikaia-std/src/tools` as `crates/nikaia-compiler/src`.

## Facts
* `Parsed`, `Lowered`, `SourceMap`, `Build`, `Provenance` (trust), `Needs` and `emit_module_body_at` (`emit/mod.rs` :463, lane 2) stay Rust. `Ledger` is Nikaia's (`ty.nika`, lowered as `crate::contracts::Ledger`). The units cross as indices into a Rust slice held by the host (the arena pattern of M3a/M3b, `ModulesEmitHost`, `modules.nika` :1037, Rust impl `modules.rs` :741).
* New trait `ComptimeLowerHost` in `comptime_tree.nika`, Rust impl `LowerHost<'a> { units: &'a [&'a Parsed], build: Build, provenance: Provenance, map: RefCell<SourceMap> }` in `comptime_run.rs`. Signatures:
  `fn unit_count(ref self) -> i64 sync`;
  `fn package_of(ref self, at: i64) -> String? sync` (`units[at].package`);
  `fn preamble(ref self) -> String sync` (every unit's `needs_of` joined, then `needs.preamble()`);
  `fn emit_unit(ref self, at: i64, beside: ref Vec[i64], contracts: ref Ledger, entry: bool, placed_at: i64?) -> String? sync` (Rust: `without_foreign(units[at], contracts)`, then `emit_module_body_at(unit, &beside_as_refs, build, provenance, contracts, &Ledger::blank(), entry, &Reads::none(), None).ok()?`; where `placed_at` is given, `map.borrow_mut().extend(body.map.placed(placed_at as usize, at))`; returns `body.rust`);
  `fn infer_package(ref self, at: ref Vec[i64]) -> Ledger sync` (`Ledger::infer_package(&those, std_ledger())`).
  `placed_at` is the generated text's length in **bytes** (`rust.len()` in Rust); check Part III that `String.len()` is bytes before relying on it.
* Nikaia functions (prefix `comptime_`): `comptime_lowered_units(host, entry: i64, contracts: ref Ledger, packages: ref collections::BTreeMap[String, Ledger]) -> String?` (the text; the Rust adapter builds `Lowered { rust, map: host.map.into_inner(), published: Default::default() }`); `comptime_dependency_modules(host, packages) -> String?`; `comptime_by_package(host) -> collections::BTreeMap[String, Ledger]`; `comptime_without_foreign(mut items: Vec[Spanned[Item]], words: ref winnow_grammar::InternerContext, runs: fn(ref String) -> bool sync)` (drops `extern` items, functions and impl methods whose key `runs` refuses; key `name` or `Target::name`; the pattern of `comptime_drop_readers` :153). `runs` stays a Rust closure over `ledger.functions.get(key).is_none_or(|c| build_time::may_not_run(c).is_none())` (`may_not_run` is row BT's).
* `trust::analyse_package(&beside, library).provenance` stays in the Rust adapter of `lowered_units`, before the host is built.
* Rust adapters keep their signatures, so the callers do not change: `grammar_run.rs` :361 (`without_foreign`), :389 (`by_package`), :393 (`dependency_modules`); `comptime_run.rs` :145, :163, :229, :418, :443. No caller in `crates/nikaia/tests/it/`.
* Texts byte for byte: `"// Generated: a \`comptime\`'s program (ADR-321).\n\n"`, the `pub mod {name} {` header with its `#[allow(unused_imports)]` and prelude `use`, `"\nconst __NIKAIA_SITES: &[nikaia_std::abort::Site] = &[];\n"`.
* Do not touch: `crates/rs2nika/**` except `rs2nika.names`, `emit/*`, `check/*`, `contracts/*`, `grammar_run.rs`, `project*`, and the uncommitted files `git status` shows at start (`text_tiers` `.nika`/`.rs`, `crates/rs2nika/convert/nikaia.*`, `docs/briefs/k1-comptime-tree.*`, other reports): another session's.
* Tests: `it` filters `comptime grammar_at_build_time` (the `comptime_compiled` file is long: run it once, at the end, beside the guard set of `.claude/agents/builder.md`), `tests/compiler` through `nikaia test` once at the end (output to a file).

## To build
1. Step 1, the draft's host calls: add `[forms comptime_]` and `[hosts comptime_]` sections to `crates/rs2nika/convert/rs2nika.names` for the calls above, re-convert (`C=crates/rs2nika/convert/target/nikaia/bin/rs2nika; $C --cut crates/nikaia/src/comptime_run.rs:119-271 > D.rs; $C --profile check-free --prefix comptime_ --file D.rs > D.nika 2> D.notes`), build from that draft; `--score` against it.
2. The host trait, the four functions with doc comments into `comptime_tree.nika`, the Rust host impl and adapters; `lower-std` (only `comptime_tree.rs` and the index change). Commit `comptime: a build-time program of several files assembled in Nikaia (K3a part 1)`, `Part: #125`, with `rs2nika.names` in the same commit.
3. `docs/selfhosting-plan.md` row K3a: part 1 built, part 2 (`compute` .. `culprit`) left, over `ComptimeLowerHost`; self-hosting % (`python3 scripts/self_hosting.py`) in the commit body.

A lowering or checker defect met is fixed in the compiler as its own commit with a test; more than one: stop, `Needs:`.

## Done when
The `it` filters and the guard set once at the end, beside clippy; `tests/compiler`; lower-std fixpoint. Changelog: none (a move).

## Stop and report if
A message or generated text that would change; an edit needed in a lane 2 file; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/k3a1-comptime-lower.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
