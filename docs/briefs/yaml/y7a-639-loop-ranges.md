---
agent: builder
model: opus
max_turns: 120
budget_usd: 32
---
# Brief: #639, per-turn interval candidates for a loop of at most 16 turns (ADR-361 D12, owner's decision 2026-10-30)

**Read first: ADR-361 D4, D5, D9 and D12 (new)**, Part III 14.2 (`docs/specification/30-nikaia-tooling.md` line 454), then the spike `spike/prover-loop-ranges` (09a7d79f): `git show 09a7d79f -- crates/nikaia-std/src/tools/prover_loops.nika` (+219 lines, a prototype) and its probes `git show 09a7d79f:spike-probes/<name>.nika` (a_const, b2_typed, c2_typed, c_acc_for, d_throw, e2_post, e_caller). Read them with `git show`; never check the branch out in the shared tree.

## What the spike found (architect's summary)

* A counter from a known start by a constant to a constant limit (at most 16 turns): every other state name with a known start is evaluated over intervals turn by turn; turn `j` gives the linear candidate `counter > start + j*k || lo_j <= x <= hi_j`, checked by D4's three queries like any other. Proves json's `hex4` shape (`n <= 0xFFFF` after four turns of `n = n*16 + v`).
* A `throw` in a loop body is a way out like a `return` (before, such a loop was not stated at all).
* A whole-number `let` (typed, or a cast) that nothing reads binds a fresh name, so the conditions on it constrain it.
* The spike raised the Houdini rounds 4 -> 4+16; keep the extra rounds to the loops that get D12 candidates (ADR-258 D2: only the claim that needs it pays).
* Estimate for the proper build: 350-500 lines in 3-4 files, Part III 14.2, about six tests. **Not in this brief** (y7b, next): #1009 (`let mut cp = call()` never read, `prover_walk.nika:499`'s early return; facts of a `let` inside a branch lost at the join) and the untyped `let d = ds[k]` read (probe b vs b2). A probe that needs those stays out of this brief's tests.

## To build (each step a commit)

1. **Pre-flight (first ten turns):** build the spike's `prover_loops.nika` diff onto develop in your head, not in the tree: list what of it is prototype (fixed limits, names) and what stays. Run probes a_const and b2_typed against develop's compiler (`check --asserts`): both open today, say so under `Tests:`.
2. **Throw as an exit and the fresh name for an unread whole-number `let`** (small, separable): with tests `prove_a_throw_in_a_loop_is_an_exit.nika` (from d_throw) and one for the fresh name.
3. **D12 candidates**: per-turn intervals for a loop with a constant trip count of at most 16 turns, a `for` over a literal range and a `while` counter against a literal limit; nothing proposed beyond 16 or for a trip count the prover does not read; `--asserts` names what carried the proof (D7). Tests from a_const, b2_typed, c2_typed, e2_post, e_caller, and one with 17 turns that stays open (warning under `unproved-claims = "warn"`, as `unproved_claims` tests do).
4. **Part III 14.2**: a paragraph saying D12 (constant trip count, at most 16 turns, each candidate checked, a `throw` an exit). `Changelog:` one sentence; the version raise is the supervisor's.

`lower-std` after each `tools/*.nika` change; the gate of `.claude/agents/builder.md` (#852), plus `cargo test --profile fast -p nikaia --test it -- prove unproved_claims`. The last commit says `Fixes #639` and `Part of #1003`.

Files this brief may touch (beyond `packages/yaml/`): `crates/nikaia-std/src/tools/prover_*.nika` and their lowered files, `tests/compiler/tests/prove_*`, Part III 14.2 of `docs/specification/30-nikaia-tooling.md`; not `packages/json/` (y4c's).

## Facts the last builders paid for

* The lowering cache misses an inferred `sync` (#1014): declare new or changed prover functions `sync` (y5 lost 8 turns to stale `prover_passes.rs`).
* To probe an unproved claim, put the call in a loop body or behind an input the prover cannot pin: a pure call with literal arguments is run at build time and proved. `--asserts` lists only the checker's claims; NK1300 for a precondition is raised at the callee's `assert` (`check_claims.nika`).
* `cargo test --lib prove` runs 0 tests; the prover's tests are `tests/compiler` and `it -- prove unproved_claims`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).
* **Package gate** (this lane): `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project packages/yaml` green, and `cargo test --profile fast -p nikaia --test it -- fmt::every_nika_file` (it formats every `.nika` under `packages/`). For a change only under `packages/yaml/` this is the gate the line above asks for; a change under `crates/` runs the gate of `.claude/agents/builder.md` (#852) as well. `target/fast/nikaia` is built by the session hook; if it is missing, `cargo build --profile fast -p nikaia` once.
* A package change raises no toolchain version (CLAUDE.md table; ADR-yaml-1 D1): `Changelog: none`, unless the brief changes `crates/`.
* Files not to touch: anything outside `packages/yaml/` and the brief's own report, unless the brief names it; never `docs/briefs/batch.txt` or a brief of another lane.

* **Stop about 15 turns before `max_turns`**: commit and push every finished step, then report `Result: partial` with what is left under `Needs:`. Uncommitted work at the end is lost work (y4 ran out at 110).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y7a-639-loop-ranges.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
