---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief p9: #1053 step 9, the figure counts claims and `panic`s, and `formal-verification.md` publishes it

Facts: Where things live, Traps already met, Commands

Score: S=3 (the owner's 100 % target is only honest if the count covers every run-time check: an `assert` moved to a precondition (ADR-269 D5) is a check at its callers, and a `panic` is a run-time error claim (b) must count), O=1 (the lane's own measure), C=4 M (a flag on `lower-std`, the script, one doc section). Area: prover. Must land before the lane puts `assert`s into `tools/*.nika` (#1053 step 8).

Pre-flight: p8 (#1076, 788981d) and p10 (#1084) are committed (`git log --oneline -12 | grep -e 788981d -e "checked entry"`, and `git log -12 --format=%B | grep "#1084"`); if not, stop: `Result: blocked`, `Needs: p8 and p10 first` (without it the claim count would report unproved preconditions as proved).

## Decision (architect, 2026-10-31; a development aid, no language change)

* `nikaia lower-std --bounds --asserts` prints, for each tools file, its `--bounds` report as today and then its `--asserts` report (the block `check --asserts` prints, `check_claims_report` in `check_facts.nika` :720, the file's path as `--bounds` prints it). `--asserts` without `--bounds` on `lower-std` prints the `--asserts` blocks alone. `lower-std` without either is unchanged. The `--help` text of `lower-std` (the paragraph on `--bounds`) says it in one sentence.
* `scripts/proved.nika` sums the `asserts in` header lines too and prints, after the `proved:` line, `claims: N asserts, P proved (L through a loop), C preconditions proved at every call, U not proved` (the header's fields; a file with no `assert` adds nothing), then `panics: K in F files` (it reads the tools sources, `crates/nikaia-std/src/tools/*.nika` beside the script's checkout, and counts `panic(` outside `//` comments: 13 in 9 files today), then `left at run time: X` = unproved indexes + unproved operations + `U` + `K`. That last line is the lane's target figure: 0 is claim (b).
* A report without `asserts in` lines (today's `--bounds` alone) prints `claims: not counted (run lower-std with --asserts)` instead of the claims line, and `left at run time` without the `U` term, saying so.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia/src/main.rs` :569 `lower_std_bounds`, :756 the `Command::LowerStd` dispatch (`args.bounds` :174, `args.asserts` :165).
* `crates/nikaia/src/sysroot.rs` :541 `bounds_tools` (add the asserts block per file; how `check --asserts` reaches `check_claims_report` for one file: follow `args.asserts` from `main.rs` :471 / :806).
* `scripts/proved.nika` (102 lines; `number_before` reads the header fields).
* `docs/formal-verification.md` §2 (lines 77-91, the figure paragraph) and §4 (line 120, *Now (A)*).
* No `tools/*.nika` changes, so no `lower-std` re-lowering.

## Steps

1. The flag and the script. Test: a `cargo test -p nikaia --lib sysroot::` row only if one already covers `bounds_tools` (p1 found that a `tests/compiler` program cannot run a sysroot command); otherwise verify by hand and say so under `Tests:`: run `target/fast/nikaia lower-std --bounds --asserts > R` (about 45 s) and `target/fast/nikaia run scripts/proved.nika -- R`; quote the four lines under `Tests:`. Check a scratch copy of one tools file with one proved `assert` added is counted (in a clone or reverted by name at once; never commit it).
2. The figure, at both settings (`--optimization prove-through-loops:on` as the second run), into `formal-verification.md` §2 (replace the de2af5b2 figure paragraph: the commit, both settings, the claims and panics lines, `left at run time`) and §4 *Now (A)*'s percentages; the sentence "26 `assert` and 11 `panic`" is replaced by the counted figures. `docs/writing-guide.md` rules: no absolute claims, the measured commit named. `Part: #1053`.

Changelog: write one sentence (`lower-std --asserts` is a toolchain change), not `none`.

## Done when

Build, fmt, clippy, `pre-push-check.sh`; `cargo test -p nikaia --lib sysroot:: bounds_report::` green; committed locally. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if

The change needs a file outside *Locations*; an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p9-1053-claims-in-the-figure.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
