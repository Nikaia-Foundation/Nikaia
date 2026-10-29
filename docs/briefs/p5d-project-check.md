---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P5d, `check`, `newly`, `keeps_that_moved` and `explain` of `project.rs` into `project_check.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`). Plan row P5d of `docs/selfhosting-plan.md`, *Large files 3* (draft 54 %; prerequisite P2 built). S=2, C~3-5 M (comparable: M3b, P4b). First of the batch; P5b (`lower_reading`, same file, other lines) runs after it.

Cut: crates/nikaia/src/project.rs:1193-1488
Cut: crates/nikaia/src/project.rs:1490-1674
Convert: --profile check-free --prefix project_check_
Shape: schematic host-calls (R47/R244 `[forms project_check_]` rows and R295's host parameter, learned: P4b)

The driver converts this before you start (`.draft.nika`, `.notes`, `.rs`). Ranges at 02b36d6; if `git log -- crates/nikaia/src/project.rs` shows a later change, re-grep `pub struct Around<`, `^fn newly`, `pub fn changed_lines`, `pub struct Explain`, `^pub fn explain(` and cut again. `Project::report` (2473-2578) is **not** in this row: it reads and emits a whole `Program`, which P5b's host gives; it moves with P5c.

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read `crates/nikaia-std/src/tools` as `crates/nikaia-compiler/src`, `nikaia_std::tools::` as `nikaia_compiler::`.

## Facts
* Read `docs/builder-facts.md` (new: *Host traits*, the anchored `it` filter, `let mut x: String = "…"`, #943).
* Already Nikaia (do not redo): `project_check_summary`, the three `project_check_let_*`/`lint_*` filters, the `locked_*` messages (`project_check.nika`); `Ledger`, `Contract` (`fails_with`, `keeps`, `signature: Signature?`), `Signature.params`, `Ty.text()` (`ty.nika`); `CheckFinding`/`CheckSeverity` (`check::Finding` re-exports them, `check/mod.rs:46`); `[forms]` already maps `{x}.span.at()` to `({x}.span.start as i64)`.
* **Stays Rust**: `Around`, `check::Newly`/`Kept` (read in `check/*`, lane 2), `Explain` and `asked()` (built by `main.rs:457`, `:771` from flags; Nikaia's field would be `comptime_`), `write_proofs` (`Book` is Rust), `write_ledger` and `changed_lines` (already thin over P2's messages and `diffs.nika`; the `with_context` chain must stay byte for byte), `foreign_library`. Every Rust function keeps its signature (callers: `lower_reading` `project.rs:953`, `sysroot.rs:308/582`, `specbook.rs:208`, `main.rs:473`, and `it` tests `common/mod.rs`, `sequences`, `compile_time_reads`, `grammar_at_build_time`, `prove`, `build_time`).
* **Step 1, the draft's host calls**: add a `[forms project_check_]` and `[hosts project_check_]` section to `crates/rs2nika/convert/rs2nika.names` (format: the `project_members_` sections there) for the Rust calls below, re-convert, and build from that draft; `--score` against it. Re-convert with the driver's two commands (`C=crates/rs2nika/convert/target/nikaia/bin/rs2nika; $C --cut crates/nikaia/src/project.rs:1193-1488 --cut crates/nikaia/src/project.rs:1490-1674 > D.rs; $C --profile check-free --prefix project_check_ --file D.rs > D.nika 2> D.notes`). The table is data, not converter code: the converter lane's uncommitted `crates/rs2nika/convert/nikaia.*` are not yours.
* **`pub trait ProjectCheckHost`** (Rust impl `CheckHost` in `project.rs`, built by `check` over `parsed`, `around`, `own`, `library = foreign_library(foreign)?`, `modules = foreign.packages(modules)`; `RefCell` for the facts and the violations):
  - `fn check_against(ref self) -> Vec[CheckFinding] sync` (`check::check_against(..)`; keeps `checked.facts` for `facts_out`)
  - `fn foreign_check(ref self) -> Vec[CheckFinding] sync` (`crate::foreign::check(..)`)
  - `fn sync_violations(ref self) -> i64 sync` (`sync::check(..)`, kept)
  - `fn render_sync_violation(ref self, at: i64, path: ref String, source: ref String) -> String sync`
  - `fn render_finding_in(ref self, finding: ref CheckFinding, path: ref String, lines: ref Vec[String]) -> String sync`
  - `fn in_the_wrapper(ref self) -> bool sync` (`WRAPPER_MARKER` set or `WARNED`)
  - `fn eprint(ref self, text: ref String) sync` (std has no `eprint` without a newline)
* Nikaia `project_check_check[H: ProjectCheckHost](host: ref H, path: ref String, source: ref String, user_parallelism: ref String, refuted_claims: ref String, unproved_claims: ref String, building_a_library: bool) -> ProjectCheckSaid` with `pub struct ProjectCheckSaid { warnings: String, refused: String? }`: the NK1238 retain, the sort, the three filters, the printing loop, the summary. Rust: `refused` -> `Err(diagnostics::refuse(text))`, else `Ok(warnings)`; `facts_out` filled from the host.
* Nikaia `project_check_gained(committed: ref Ledger, inferred: ref Ledger) -> collections::BTreeMap[String, Vec[String]]` and `project_check_keeps_that_moved(committed: ref Ledger, inferred: ref Ledger) -> collections::BTreeMap[String, Vec[ProjectCheckKept]]` (`ProjectCheckKept { param: String, now: bool, ty: String }`; `signature.arguments()` is `signature.params`, check `contracts/mod.rs:123`). Rust `newly` keeps reading and parsing `nikaia.contracts` and maps `ProjectCheckKept` to `check::Kept`.
* **`pub trait ProjectCheckExplainHost`** (Rust impl over `&Program`, `&Settings`, `&Reads`, `std_ledger()`): `fn units(ref self) -> i64 sync`, `fn unit_path(ref self, at: i64) -> String sync`, `fn unrolling_report(ref self) -> String sync`, `fn explain_report(ref self, kind: ref String, at: i64) -> String sync` (kinds `overlaps`, `sharing`, `tethers`, `asserts`, `bounds`, `trust`: one arm each, the `NO_COLOR`/terminal test inside `asserts`). Nikaia `project_check_explain[H: ProjectCheckExplainHost](host: ref H, comptime_asked: bool, kinds: ref Vec[String])` prints in today's order; Rust `explain` keeps `asked()` and passes the asked kinds in that order.
* Every refusal, warning and report byte for byte (`tests/compiler` `.stderr`, the `--explain` tests).
* Do not touch: `crates/rs2nika/**` except `rs2nika.names`, `contracts/*`, `check/*`, `emit/*` (lane 2), `manifest.*`, `modules.*`, `docs/briefs/*` other than your report, and the uncommitted files `git status` shows at start.
* Disk: about 3.7 GB free at 02b36d6; `agent.nika` escalates under 3 GB. Copy `target/fast/nikaia` to your scratch dir before a long test run; right before it, `git status --short` the lowered files.
* Tests: `--lib project::`; `it` filters `project check newly keeps explain asserts sharing overlap tether bounds trust compile_time_reads prove build_time sequences`; `nikaia test --project tests/compiler`; `lower-std` fixpoint (only `lowered/project_check.rs` and the index change).

## To build
1. The `rs2nika.names` rows, the two hosts, `project_check_check`, `project_check_gained`, `project_check_keeps_that_moved`, `project_check_explain` into `project_check.nika` with doc comments; `check`, `newly`, `keeps_that_moved`, `explain` forward. Commit `project: the check's tally and the explanations into Nikaia (P5d)`, `Part: #125`; plan row P5d updated (built, what stays Rust, `report` moved to P5c), self-hosting share in the commit body.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p5d-project-check.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
