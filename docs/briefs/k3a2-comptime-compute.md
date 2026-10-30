---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: K3a part 2 - `computed` and `worked_out_ahead` move into Nikaia (Part #125)

Score: self-hosting lane, bootstrap week lane 1, row K3a of `docs/selfhosting-plan.md`. Precondition: `k3a1-commit.md` (K3a part 1, the `ComptimeLowerHost`) `b995-coalesce-map-get.md` (#995, `a ?? m.get(k)`, 0cf0b173) and `b997-let-lambda-string-param.md` (#997, the let-bound `known` lambda of :393) are on `origin/develop`; if one is not, stop with `Result: blocked`, `Needs:` the missing one.

Cut: crates/nikaia/src/comptime_run.rs:276-536
Cut: crates/nikaia/src/comptime_run.rs:544-555
Convert: --profile check-free --prefix comptime_
Shape: schematic host-calls

The line numbers are those before k3a1-commit; re-grep `fn computed`, `fn worked_out_ahead` and `const NOT_LOWERED` if they shifted, and cut the same functions.

## What moves
* `computed` (the body of `compute`, :308-475) and `worked_out_ahead` (:477-536) become `comptime_computed` and `comptime_worked_out_ahead` in `crates/nikaia-std/src/tools/comptime_tree.nika`, faithful in their order of steps and in every text they produce (byte for byte: the tests compare them). They keep the closures `known`/`typed` as parameters (`fn(String) -> BuildValue? sync`, `fn(String) -> Ty? sync`), and take a `host: ref ComptimeComputeHost`.
* The consts `NOT_LOWERED`, `NOT_READ`, `NOT_CARRIED`, `NOWHERE` (:544-555) become `pub comptime` in `comptime_tree.nika`; Rust keeps `pub(crate) NOWHERE` as a re-export (it is used only in `comptime_run.rs`).
* What stays Rust: `compute` itself (the adapter: `defaults_known_from` / `defaults_compiled_in` thread_locals), `thread_local AHEAD`, the `Parsed` arena, everything outside the cut. Callers keep their signatures: `check/host.rs:276` (`compute_comptime`) and `contracts/mod.rs:1277`.

## The host (a sketch; build what the faithful body really needs, no more)
A trait `ComptimeComputeHost` in `comptime_tree.nika`, all methods `sync`, units by index into a Rust arena (the pattern of `ComptimeLowerHost`, k3a1): a Rust `ComputeHost<'a>` holds the files `&[&Parsed]`, a `RefCell<Vec<Parsed>>` of units it made, the library `&Ledger`, the workshop and the `Bounds`. Likely methods:
`sub_program(here, bound, value, ty, known, typed) -> i64?`, `unit_without_its_constants(at, bound, known, typed) -> i64?`, `undeclared_callee(here, value) -> String?`, `constants_of_files() / constants_of_units(units) -> BTreeSet[String]`, `without_readers(units, absent)`, `value_fn_names(unit)`, `unit_package(unit) -> String?`, `by_package(units) -> BTreeMap[String, Ledger]` (or call `comptime_by_package` with the lower host), `derived(units, packages) -> Ledger` (library clone, `LedgerOps::absorb`, `infer_package`), `forbidden_because(derived) -> String?` (`may_not_run`), `culprit(here, value, derived) -> String?`, `driven(...) -> ComptimeDriven` (an enum: `Program(String)`, `NotLowered`, `Uncrossed{ty, because}`, `NoHandBack`; over `emit_program`, `dumper`, `with_driver`), `run_comptime(program, bound) -> ComptimeRan` (a Nikaia enum mirroring `Wall` of `grammar_run.rs:43` plus `Answer(String)`), `decode(text) -> BuildValue?` (or call `gr_decode` of `grammar_decode.nika` directly with a `char_of` host closure), `enter_ahead(bound) -> bool`, `is_ahead(name) -> bool`, `leave_ahead(bound)`, `comptimes_of(at) -> Seq[(String, Expr, Ty?)]`, `folded(at, value, known) -> BuildValue?`.
Where a step is already Nikaia (`comptime_sub_program`, `comptime_constants_of`, `comptime_drop_readers`, `comptime_undeclared_callee`, `comptime_culprit`, `comptime_lowered_units`, ...), call it from Nikaia instead of through the host.

## Forms to watch (CLAUDE.md, *A missing form stops the work*)
* `known(name) ?? ahead.get(name)` needs #995 (done, 0cf0b173): a text or `BuildValue` element is written `ahead.get(name)?.clone()` (ADR-293 D15). The `let`-bound lambda of :393 needs #997 (b997 in this batch). A `match` handing back text refused NK2302 is #998 (b998 in this batch). Another `??`/`get` mismatch: stop, `Found:` with a probe.
* Sets have no `iter`/`filter` in `std.contracts` (not decided by any ADR). Write `absent` with `difference` (it exists and returns a sequence): `files_constants.difference(carried).map(fn(s) { s.clone() }).collect()`. Do not write a `for` loop over a set to stand in for a missing `filter`; if a step needs a set form that does not exist, stop: `Result: blocked`, `Found:` + probe, `Needs: owner decision`.
* A defect of the lowering (a wrong Rust type, a missing clone): stop and report it under `Found:` with a probe; do not write around it.

## To build
1. Read the cut range and `comptime_tree.nika` (K1, K2, K3a part 1). `rs2nika --profile check-free --prefix comptime_` on the cut, `--score` for the report.
2. The trait, the two functions and the consts in `comptime_tree.nika`; `lower-std` (`docs/builder-facts.md`), the fixpoint changes only `lowered/comptime_tree.rs` and its index line in `lowered.rs`.
3. `ComputeHost` and its impl in `comptime_run.rs`; `compute` calls `comptime_computed`; delete the Rust bodies. Commit.
4. Tests: `cargo build --profile fast -p nikaia`, the `it` filters `comptime grammar_at_build_time` plus the guard set of `.claude/agents/builder.md`; `tests/compiler` programs with `comptime` (`grep -l comptime tests/compiler/*.nika`) through `nikaia test` on those files. No new test is needed for a faithful move.
5. Row K3a of `docs/selfhosting-plan.md`: part 2 built, the self-hosting share as the plan's script counts it. Commit and push.

## Done when
`computed` and `worked_out_ahead` are Nikaia; the Rust left in the cut range is the adapter and the host; the tests above pass; pushed. Changelog: none (a move). Commit subject: `comptime: computing a build-time constant moves into Nikaia (K3a part 2)`, `Part: #125`.

## Stop and report if
A text a test compares would change; a missing form (above); an edit needed outside `comptime_run.rs`, `comptime_tree.nika`, its lowering, `lowered.rs`'s index line, `rs2nika.names` (`[forms comptime_]`/`[hosts comptime_]` rows) and the plan row; an error in another agent's file.

**Not yours, never stage**: `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}` and every `docs/briefs/*.report.md` that `git status` shows modified. If `lowered.rs` shows another session's hunk, stage only yours (`git diff ... > p; edit p; git apply --cached p`).

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/k3a2-comptime-compute.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, nor a step another commit on develop already built: name the commit in `Body:`, nor a test's expected output taken from what the program really prints: say so in `Body:`); how you verified goes under `Tests:`. A `noop` says why under `Tests:`, not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
