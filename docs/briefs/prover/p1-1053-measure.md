---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief p1: #1053 step 1, count proved checks over the tools package as it lowers (`nikaia lower-std --bounds`, `scripts/proved.nika`)

Facts: Where things live, Traps already met, Commands

Score: S=2 (the owner's target, 100 % proved, needs a figure that is true first; today's figure is not, see below), O=2 M (every later prover brief chooses its work by this tool's table instead of an architect's hand count), C=6 M (a flag through an existing path plus a small Nikaia script; like the `--bounds` reason move, #435), area prover.

## Problem (probed on d206915, `target/fast/nikaia`)

`docs/formal-verification.md` §2 and §5 count the proved indexes and arithmetic steps of `crates/nikaia-std/src/tools/*.nika` with `nikaia check --bounds FILE`, one file at a time. Today that gives 1,821 indexes, 965 proved, and 1,381 operations, 560 proved, over 246 files. But `check` refuses 227 of the 246 files: the types and functions from the package's other files are unknown to it (NK1135 *no type called `Expr`*, NK1117...). Sites in code it cannot type drop out of the count. For example, `indexed.atoms[atom]` at `logic_search.nika:63` is not listed. `nikaia check --bounds crates/nikaia-std/src/tools` stops with *Is a directory*.

The tools package is a package (#723): `lower-std` lowers each file with the bodiless text of the others beside it (`without_bodies`) and the package's ledger. The count has to be taken the same way.

## Locations

* `crates/nikaia/src/main.rs`: `Command::LowerStd` (about line 356), its dispatch (about line 730), `fn lower_std` (line 540). `check`'s `--bounds` flag reaches `project::Explain { bounds, .. }` at about line 464.
* `crates/nikaia/src/sysroot.rs`: `pub fn lower_std` (line 1127), `lower_tools_files_cached` (line 462; per-file check and lowering at about line 555, `lower_one`), `tools_build` (line 367).
* `crates/nikaia/src/bounds.rs`: the report `check --bounds` prints (`walk::bounds_aggressive`, about line 293). `crates/nikaia-std/src/tools/bounds_walk.nika` and `bounds_reasons.nika` produce the lines.
* `scripts/tokens.nika` and `scripts/where.nika` show how a `scripts/*.nika` reads its arguments and files.

## Steps

1. **`nikaia lower-std --bounds [--optimization NAME:LEVEL]...`**: lowers the tools package as `lower-std` does, and prints for each file the report `check --bounds` prints (the same `bounds in <path>: N indexes, M proved; ...` header and the same site lines), with each file seeing the package. With `--bounds`, nothing is written and no cache entry is read or written: the report needs the walk. The `runtime` package and `std` modules may be left out, but say so in `--help`. Show in `--help` that it is a development aid. A file the package check refuses is reported as `refused: <path>` and counted, never skipped silently. Test: run it on the checkout and compare with the per-file figure above. No file may be refused, and the totals have to be at least the per-file ones (state both under `Tests:`). A `.nika` test in `tests/compiler/` is not possible for a sysroot command, so name that under `Tests:`. That is no deviation.
2. **`scripts/proved.nika`** (run as `nikaia run scripts/proved.nika -- REPORT`, where REPORT is the saved output of step 1): it prints one line `proved: I/IT indexes (P %), O/OT operations (Q %), F files, R refused`, then a table of unproved sites by reason (count, reason), then the 15 files with the most unproved sites (unproved, file). The CLAUDE.md helper-tool rule applies: a form the script lacks is a finding, not a way around.
3. **The figure into `docs/formal-verification.md`** §2 and §4. Replace the 54 % / 45 % sentence with today's measured figures, at the default walk and at `--optimization prove-through-loops:on`, with the commit and the command. Add one line to §5 saying the counting tool exists and how to run it. Use honest wording only (`docs/writing-guide.md`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `docs/briefs/prover/p1-1053-measure.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
