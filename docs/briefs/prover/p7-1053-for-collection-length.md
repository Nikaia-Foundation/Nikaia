---
agent: builder
model: opus
max_turns: 70
budget_usd: 14
---
# Brief p7: #1053 step 4, finished: the checker records the length of what a `for` walks, so D13's turn count reaches a `for` over a collection

Facts: Where things live, Traps already met, Commands

Score: S=3 (claim (b), the owner's 100 % target: the counter reason holds 541 of the unproved sites under `on`, most of them `at += 1` in a `for x in xs`), O=3 M (every user's `for x in xs { n += 1 }` keeps its overflow check today), C=3 M (one fact in the checker; p6's walk code already reads it). Area: prover.

## Decision (architect, 2026-10-31, under *Language decisions*: no language change, a fact the checker already records for `.len()`)

p6 (28acd94) built ADR-361 D13 in the bounds walk, but the walk learns a collection's length only where the checker recorded it in `facts.std_lengths` / `facts.sized_lengths` (`check_core_method.nika` :916-921, `.len()` receivers only). `check_core_stmt_for` records the iterable the same way: when `check_core_method_counted(over)` (make it `pub`), insert `id_of(iter) as i64` into `std_lengths`, and into `sized_lengths` where `state.elements_take_space(over)`. A map or set walks one turn per entry, a list one per element. (`for c in text` over a `String` passes check and fails in rustc today, #1075; leave it to the bug lane.) Nothing else changes: `bounding_length` (`bounds_walk.nika` :1715) reads the fact by `node_key(receiver)`.

## Problem (probed 2026-10-31 on the binary of 28acd94)

`target/fast/nikaia check --bounds tests/compiler/tests/prove_for_counts_its_turns.nika` (copy it outside `tests/compiler`): line 11 `every`'s `at += 1`, line 20 `some`'s `n += 1`, line 30/33 `same`'s `xs[at]` and `at += 1` are `checked: a counter a loop changes, with no bound tied to the loop's turns`; only line 44 (`ranged`) is proved. After this brief all four are proved at both settings (`off` and `prove-through-loops:on`); the soundness rows (52 unbounded start, 60 `u8` `n += 2`, 71 reset, 80 inner-loop growth, 89 `n * 2`, 97 `u8` over an unknown range) stay checked. If a soundness row turns `proved`, stop at once: that is a wrong proof.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/check_core_stmt.nika` :1558 `check_core_stmt_for` (record after `let over = check_core_expr(...)`).
* `crates/nikaia-std/src/tools/check_core_method.nika` :187 `check_core_method_counted` (to `pub`), :916-921 the `.len()` recording to mirror.
* `crates/nikaia-std/src/tools/check_decls.nika` :76 `elements_take_space`.
* Read only: `bounds_walk.nika` :1715 `bounding_length`, and p6's `bounding_turn_bounds`. If the walk does not pick the fact up unchanged, the fix in `bounds_walk.nika` is allowed (say what in `Body:`).
* Re-lower each changed `.nika` (`lower-std`), commit its `lowered/*.rs`. None of these files is in `docs/briefs/batch.txt` or `docs/briefs/bugs/batch.txt`'s current briefs.

## Steps

1. Record the length; `prove_for_counts_its_turns.bounds` and `.loops.bounds` re-blessed (`nikaia test --project tests/compiler --bless prove_for_counts_its_turns` or the way p5 built) and read through line by line against the list above. Add two rows to `prove_for_counts_its_turns.nika` (`use std::collections`): `for (k, v) in m` over `m: collections::HashMap[String, i64]` with `n += 1` (proved after), and `for c in text.chars()` (`text: String`) with `n += 1` (stays checked: a produced sequence has no length the checker records; probed, it runs and prints today). Both `.bounds` files changed accordingly.
2. Keep passing: every `tests/compiler` test named `prove_*` or `bounds_*` (`nikaia test --project tests/compiler --both-settings`), `cargo test -p nikaia --test it -- prove bounds`. A test whose `.bounds` rightly gains a proof is re-blessed and named in `Body:`.
3. The figure: `target/fast/nikaia lower-std --bounds > R` and `... --optimization prove-through-loops:on > R2`, `target/fast/nikaia run scripts/proved.nika -- R` / `-- R2`: both `proved:` lines and the counter reason's count under `Tests:` and in `Body:` (28acd94: off 1,520/2,646 indexes, 733/1,620 operations; on 1,552/2,646, 740/1,620; counter 541 on). `Part: #1053`.

Changelog: write one sentence (a prover change raises the version), not `none`.

## Done when

The tests above pass; `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if

An edit is needed outside the files under *Locations*, their lowered twins and the tests; a soundness row turns `proved`; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (`crates/nikaia-std/src/tools/lowered/project_members.rs` is residue, #1060), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p7-1053-for-collection-length.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
