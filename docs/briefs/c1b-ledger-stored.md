---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 12
---
# Brief: C1b, the `stored` fold of `infer_package_noted` into `ledger_items.nika`; the dead `LedgerOps::candidates` goes (Part #125)

Score: self-hosting lane, plan row C1 of `docs/selfhosting-plan.md` (second part; C1a built, 5bd603f3). Small: S=1, C~1 M. `lookup` is split off as C1c: it waits on #1107 (bug lane b26), a `ref T?` unwrapped by a `match` or `??` loses its `ref`, so `(String, ref FnContract)?` cannot be said yet. Do not move `lookup`. Touches no file of P5c (`project.rs`, `project_drive.nika`) nor of the bug lane's queue.

Cut: crates/nikaia/src/contracts/mod.rs:444-455
Convert: --profile check
Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **What moves**: the fold at `contracts/mod.rs:444-455` (in `infer_package_noted`) that merges every `Checked`'s `stored_code` into one `BTreeMap<String, StoredCode>` (`crate::check::StoredCode` is the lowered `CheckStoredCode`, `tools/check_state.nika:551`, with `empty()`). New `pub fn ledger_items_stored(codes: Vec[collections::BTreeMap[String, CheckStoredCode]]) -> collections::BTreeMap[String, CheckStoredCode]` in `ledger_items.nika` (or the prefix that file uses for its public names; follow it), the same merge: `resolved` and `free` extended, `unresolved` or-ed, keyed entries made with `CheckStoredCode::empty()` (`or_insert_with`, or ADR-384's `or_default` only if `CheckStoredCode` has a `Default`; say which in `Body:`). Rust keeps `let stored = ledger_items_stored(checked.iter().map(|c| c.stored_code.clone()).collect())` (the Rust already clones each pair). The architect probed the merge shape (a `for (key, code) in codes` with `entry(key.clone()).or_insert_with(..)` and `extend(code.resolved.iter().cloned())`) on 7ad2ea08: it runs.
* **Dead code**: `LedgerOps::candidates` (trait `contracts/mod.rs:170`, impl :554-584 with its doc comment) has no caller (`scripts/where.nika -- candidates`: only `surface.nika`'s own `candidates` and `check_values.nika`'s `ledger_candidates`, which is the Nikaia one the checker uses). Delete both.
* **Stays Rust**: everything else of `infer_package_noted`; `lookup` (C1c).
* **The ledgers must not move**: every `nikaia.contracts`, `nikaia.derived`, `nikaia.proofs` a build writes stays byte for byte; `git diff --stat tests/compiler/nikaia.*` after the tests shows nothing.
* **#1060 guard**: after `lower-std`, only `ledger_items.rs` and `lowered.rs` differ from `HEAD` among the lowered files you produced; any other is `Found: #1060` and stops the brief.
* Do not touch: `crates/rs2nika/**`, `project.rs`, `comptime_run.rs`, `sysroot.rs`, `docs/briefs/*` but your report, the uncommitted files `git status` shows at start (now `tools/emit_core_leaves.nika` and its lowered twin, the `crates/rs2nika/{convert,facts}/nikaia.*` files).

## To build
1. `ledger_items_stored` in `ledger_items.nika`, the call in `contracts/mod.rs`, `candidates` deleted; `lower-std` and its fixpoint; `cargo check -p nikaia`.
2. Tests: `cargo test -p nikaia --test it -- throws locks touch ledger contract` (anchored `grep -E '^test result:|FAILED|panicked'`), `cargo test -p nikaia --lib contracts::`, and `nikaia test --project tests/compiler --both-settings` on the tests whose names contain `throws`, `stored`, `lock`, `ledger` (by name); then the ledger check above. Commit `contracts: the stored-code fold moves into ledger_items.nika; the dead LedgerOps::candidates goes (C1b)`, `Part: #125`; plan row C1 updated (C1b built; C1c = `lookup`, waits on #1107), self-hosting share in the body (`scripts/self_hosting.py`).

## Done when
Build, `lower-std` fixpoint, the tests of step 2 and the ledger check pass; fmt, clippy (1.97 and 1.99.1), `pre-push-check.sh`; committed locally; `Rs2nika:` with the score.

## Stop and report if
A form Nikaia lacks (`Result: blocked`, `Found:` with a probe); an edit outside `contracts/mod.rs`, `ledger_items.nika`, its lowered file, `lowered.rs`, `docs/selfhosting-plan.md`; an error in another agent's file.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/c1b-ledger-stored.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
