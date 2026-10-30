---
agent: builder
model: opus
max_turns: 100
budget_usd: 25
---
# Brief: #1012 step 2, `NK1264` refuses a foreign impl that keeps more than the column, `NK1265` warns on a column change (ADR-277 D17)

Score: self-hosting lane; the build of an owner decision (#1012, option E). Step 2 of the *To build* list of #1012. Precondition: `b1012a-trait-column-fn.md` (step 1, the column computed in the fixpoint) is on `origin/develop`; if not, stop with `Result: blocked`, `Needs: b1012a`. **This brief may take `NK1264` and `NK1265`** (reserved by the supervisor for this decision; Part III's code table already states them); no other new code.

## Problem

After step 1, an impl of a trait from another package takes the library column. D17: an impl that keeps a position the column runs is refused with `NK1264` (message: the trait method, the parameter, the statement that keeps it, and that only the trait's package widens the column); an impl under the trait package's own `tests/` is such a client too. A change to the column of a `pub` trait method warns with `NK1265`, in either direction, naming the parameter and the impl that changed it, until the ledger is committed (the mechanism of `NK1208`). A caller refused for lending into a position the column keeps is told which impl keeps it.

## Locations
* The foreign-impl order in `crates/nikaia/src/contracts/keeps.rs`: the impl's own evidence without the back edge, compared with the library column; excess is `NK1264`; then the library column is published as the impl's entry. Raise it the way the contract pass raises its other refusals (grep an `NK12` raised from `crates/nikaia/src/contracts/`).
* `NK1265`: beside `NK1208` in `crates/nikaia-std/src/tools/contract_changes.nika` (`changed_contracts`), called from `crates/nikaia/src/prove.rs` ~416. `changed_contracts` sees only ledgers; the causing impl must reach it from the inference (an in-memory map trait method -> impl that widened or narrowed it). No new ledger field: if one seems needed, stop (`Needs: owner decision`; the ledger's form is Part III 13.5).
* The lending refusal: find where a lambda borrowing a local is refused at a kept position (the checker's `kept_args` path); where the position is kept only through the trait column, the message names the impl and its keeping statement.
* Part III's code table carries the wording of both codes; ADR-277 D17 the rule.

## To build
1. `NK1264`, and the lending message naming the impl. Test (g), toolchain-driving in the pattern of `tests/compiler/tests/commit_dry_run.nika` (`toolchain()`, a package into `mktemp -d`, `process::run`, print the stderr lines that matter): a package with `pub trait H` and no keeping impl; a dependent package (or the package's own `tests/` file) with an impl that stores the callback: refused with `NK1264`, and the message names `H::ask`, the parameter and the line. Commit.
2. `NK1265`. Test (h), same pattern: build a package and commit its ledger; add a local impl that keeps; build again: the warning `NK1265` naming the impl, the widened `keeps` in the ledger diff, and a lending caller's refusal naming the impl; remove the impl again: `NK1265` in the narrowing direction. `lower-std` fixpoint clean if a tools file moved. Commit `contracts: NK1264 refuses a foreign keeper, NK1265 warns on a trait column change (#1012)`, `Part of #1012`.

## Done when
(g) and (h) pass through `nikaia test`, the step-1 tests and `a_trait_method_callback_is_handed_on.nika` still pass, the `it` filter `contracts` and the guard set of `.claude/agents/builder.md` are green; pushed. Changelog: one line (`NK1264`, `NK1265`).

## Stop and report if
An edit outside `crates/nikaia/src/contracts/`, `crates/nikaia/src/prove.rs`, `crates/nikaia-std/src/tools/*.nika` and their lowering, the checker's diagnostics, and the new tests; the causing impl cannot reach `changed_contracts` without a ledger field; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1012b-trait-column-refusals.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
