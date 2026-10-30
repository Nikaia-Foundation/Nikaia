---
agent: builder
model: opus
max_turns: 90
budget_usd: 20
---
# Brief: fix forward the red `develop`: `sequences::the_corpus_has_no_more_unanswered_method_calls_than_it_had`

Score: a red `develop` is fixed forward first (CLAUDE.md); `main` and `promote` stand still until it is green.

## Facts (architect, from CI annotations)

* Red since run 41136499149 (push ending 75e25ed8), on arm64 and bootstrap_compiler, every run since (ad7f118f, dbc92834, 8e67ee37). Last green: a5663bf6. The failing test is the one in the title (`crates/nikaia/tests/it/sequences.rs:306`): it sums the checker's unanswered method calls over `examples/` and `crates/nikaia-std/src/**` and asserts `<= 16`.
* Commits of the first red push (`git log a5663bf6..75e25ed8`): 43312d30 (prover: grammar actions, `crates/nikaia-std/src/tools/prover_walk.nika` +135 lines, `prover_passes.nika`), 7cde2178 (`packages/yaml/` only: the test does not read it), 75e25ed8 (a report). So the suspect is **43312d30's new code in `prover_walk.nika`** (`prover_action`, `prover_action_number`, `prover_action_locals`, `prover_bound_number`). Likely candidates, unchecked: the tuple bound from a `match` (`let (called, generics, args) = match pattern { .. }`, then `generics.len()`, `generics[0].generics.is_empty()`), `rule.ret_type ?? return null` then `ret.generics`, the lambdas `fn(name) { scope.ints.contains(name) }`.
* The counter: `crates/nikaia-std/src/tools/check_core_call_on.nika:107` (`calls.unanswered += 1`).
* (An unrelated single failure of `examples::every_example_package_passes_its_own_tests` on arm64 at e89554fd passed again at a5663bf6: not yours; mention it under `Tests:` if you see it again.)

## To build

1. **Find the calls.** Run the test (`cargo test --profile fast -p nikaia --test it -- sequences::the_corpus_has_no_more_unanswered`) to get today's count. To name the calls, add a temporary `eprintln!` of the call's written text in the lowered `crates/nikaia-std/src/tools/lowered/check_core_call_on.rs` where `unanswered` is incremented, run once, and restore that file by name (`git checkout -- <that file>`, your own file only). Compare with the same list on a5663bf6's `prover_walk.nika` if needed (the count there was within 16).
2. **Fix the cause in the checker**, where the receiver's type is plainly given by the program (a tuple `let` from a `match` whose arms state the types, a `??` with `return`/`continue`, a lambda parameter typed by the `fn` type it fits): the checker's typing in `crates/nikaia-std/src/tools/check_*.nika` (plus their lowered `.rs`, re-lowered as `docs/builder-facts.md` says). Do **not** rewrite `prover_walk.nika` to dodge the gap (CLAUDE.md, *Fix, do not work around*). Add a Nikaia test in `tests/compiler/tests/` only if the fix changes what a program means (a call refused or lowered differently); the corpus test itself covers the count. If the count falls below 16, lower the ceiling to the new figure with a doc sentence above the test saying what answered the calls (the test's own convention).
3. If step 1 shows the calls come from a checker gap that cannot be fixed within this run (a change over several checker files, or one that needs a design choice): do not commit a ceiling raise. Report `Result: blocked`, `Found:` the list of calls and the receiver form that leaves them untyped (a five-line probe), `Needs: architect` — the architect then decides between a fix brief and a revert of 43312d30.

Stop about 15 turns before `max_turns`: commit what is verified and report `partial`.

## Tests

* `cargo test --profile fast -p nikaia --test it -- sequences::` (the corpus test and its neighbours) must pass.
* The gate of `.claude/agents/builder.md` (guard set; a `tools/check_*.nika` change also runs `target/fast/nikaia test --both-settings --project tests/compiler`).
* Commit line: `check: <what is now typed> (fix forward: corpus unanswered calls)`.

## Files

May change: `crates/nikaia-std/src/tools/check_*.nika`, their `lowered/*.rs`, the lowered files a re-lower changes, `crates/nikaia/tests/it/sequences.rs` (ceiling and its doc only), new files in `tests/compiler/tests/`. Not yours: `crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`, any other uncommitted file in the checkout (`git status` at start).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed. The disk has about 3.5 GB free: no second clone.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-corpus-unanswered.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
