---
agent: builder
model: opus
max_turns: 100
budget_usd: 26
---
# Brief: #1003 step 3: json decodes `\u` with `scalar::from`, then an integer `as scalar` is refused (`Fixes #1003`)

Score: the last step of #1003, which blocks #928 step 2c (yaml escapes) and leaves `cp as scalar` in json, a cast the owner decided to refuse (2026-10-30).

**Read first:** `docs/builder-facts.md`, #1003's decision (copied below), ADR-359 D3 (a function promises its result with an `assert` before `return`), the wip commit `git show dfe7bac6` (branch `wip/y4b-1003-as-scalar-refused`; read it with `git show`, never check the branch out in the shared tree).

## Issue #1003 (decided, owner 2026-10-30)

`scalar::from(n) -> scalar` with a precondition, not `scalar?`: `n` is not a surrogate and not above `0x10FFFF`, held as a precondition its callers prove (ADR-269 D5/D20, ADR-380); where not proved, refused under `unproved-claims = "error"` or checked at run time under `"warn"`. `n as scalar` from an integer other than `u8` is refused by the checker with a help naming `scalar::from`. Steps 1-2 are on develop (52effec5, 1443b829). Step 3: the checker refuses an integer `as scalar` (variable and literal), help naming `scalar::from`; the `ToScalar` run-time path goes; test `tests/compiler/tests/an_integer_as_scalar_is_refused.nika` + `.stderr`.

## To build (each step a commit)

1. **Pre-flight (first ten turns):** y7b (#1009, `Fixes #1009`) must be on develop: `git log --oneline origin/develop | grep -i 1009`, and `tests/compiler/tests/prove_a_surrogate_pair_decodes_to_a_scalar.nika` must exist and pass (`target/fast/nikaia run` on a scratch copy). If not: `Result: blocked`, `Needs: y7b`.
2. **json**: in `packages/json/src/main.nika`, `hex4` (115-139) promises its range with `assert(n >= 0 && n <= 0xFFFF)` before `return n` (ADR-359 D3; the architect checked a near copy of `hex4` with that line (`throw Bad::No` for `throw fail(..)`): proved through the loop by ADR-361 D12's turn-indexed intervals), and line 196 becomes `out.push(scalar::from(cp))`. The call stands inside `string`'s `while true` loop: if it is refused there while y7b's probes pass, that is a defect: `Result: blocked`, `Found:` with a probe of the loop shape. `target/fast/nikaia check --asserts` on the package lists the call as proved; `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project packages/json` green. Regenerate json's `nikaia.contracts`/`nikaia.proofs`/`nikaia.derived` if the build rewrites them, and commit them.
3. **The checker refuses an integer `as scalar`**: take dfe7bac6 over (`git cherry-pick -n dfe7bac6`, then resolve; its `packages/json/src/main.nika` hunk is replaced by step 2's line). It touches `num.rs` (the run-time `ToScalar` path goes), `check_core_places.nika`, `check_facts.nika`, `check_reads.nika`, `emit_core_places.nika`, Part I (`10-nikaia-light.md`, the `as` paragraph) and Part III (`30-nikaia-tooling.md`, one line), adds `an_integer_as_scalar_is_refused.nika` + `.stderr`, deletes `an_integer_cast_as_scalar_is_checked.*`. Run `lower-std` (its lowered files were not regenerated). `NK1241` exists already (Part I 2.2): no new code. Look for any other integer `as scalar` the refusal now hits: `git grep -n "as scalar" -- packages scripts examples tests crates/nikaia-std` (a `u8` operand is allowed; `crates/rs2nika/convert/src/tr_expr.nika:459` writes `as scalar` for a Rust `as char`, whose operand Rust already restricts to `u8`: leave it). The last commit says `Fixes #1003`.

## Gate and facts

* Gate of `.claude/agents/builder.md` (#852) (a `tools/check_*.nika` change: also `target/fast/nikaia test --both-settings --project tests/compiler`); json's own tests; the package gate below for `packages/yaml` (yaml uses `scalar::from`; make sure it still passes). The `sequences` guard is red on develop (18/16, *Red on develop now*): your change must not raise it.
* Declare changed check/prover functions `sync` (#1014).
* `Changelog:` one sentence (Part I and `crates/` change); the version raise is the supervisor's.

Files this brief may touch (beyond its report): `packages/json/**`, the files of dfe7bac6, their lowered files and `lowered.rs`, the files `lower-std` rewrites, another integer `as scalar` the grep finds outside `crates/rs2nika/`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y4c-1003-finish.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
