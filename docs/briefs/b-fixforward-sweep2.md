---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: fix-forward 2, arm64's *Accepted-programs sweep* red since the c7e5b66 run

Score: a red `develop` is fixed forward first (`CLAUDE.md`); `main` stands still until then (last green 590cd7c). Lane 1, before P5a.

## Facts (the first fix-forward, 60b1154, ruled the converter out)
* The first red run's commit is c7e5b66, but that **push carried two commits**: eeaefb1 (`Keep a struct of views a call makes, across a package's files; either-text map keys (0.0.1258)`) had no CI run of its own. c7e5b66 touches only `crates/rs2nika/convert/src` (not a workspace member, never dumped: 60b1154's body). **So the cause is eeaefb1**, a compiler and `std` change.
* eeaefb1 changed: `crates/nikaia-std/src/index.rs:469` (a new `impl Get<&EitherText> for HashMap<String, V, S>` beside the generic `impl<K, Q, V, S> Get<&Q> for HashMap<K, V, S>` at :438), `tools/buffers.nika` (holders, `held_text_field`, `opaque_puts`), `tools/emit_stmt.nika`, `tools/text_tiers.nika`, `contracts/keep.rs` (`buffer_plans` takes `beside`), `contracts/tether.rs`, `check/mod.rs`, `emit/mod.rs`; the lowered files follow.
* **Suspect 1** (cheapest): the second `Get` impl for `HashMap` makes `m.get(&k)` / `m[k]` ambiguous where `k`'s type is inferred later: `rustc` E0282/E0283 in lowered programs that compiled before. **Suspect 2**: the keep changes lower a one-file program to Rust that `rustc` refuses (E0502/E0505/E0597). **Suspect 3**: an entry of `KNOWN` (`crates/nikaia/tests/it/accepted_sweep.rs:42`, e.g. `a10522b5e28c4937`, #730 item 5) compiles now: the sweep fails on that too.
* How the sweep works: `accepted_sweep.rs` header. `check::check_against` (in-process, the `it` tests) dumps every accepted one-file program to `$NIKAIA_DUMP_ACCEPTED/<binary>-<hash>.nika`; the ignored test `accepted_sweep::` lowers each and runs `rustc --emit=metadata` in batches of 40. CI: `.github/workflows/nikaia.yml` :166 and :385. The CI log is not readable here; from 60b1154 on a failing sweep writes annotations, but do not wait for them.

## To build
1. **Reproduce** (the `it` binary only, never the full suite): `d=$(mktemp -d)`, `NIKAIA_DUMP_ACCEPTED=$d cargo test --release -p nikaia --test it` (failures of other tests do not matter here), then `NIKAIA_DUMP_ACCEPTED=$d cargo test --release -p nikaia --test it accepted_sweep:: -- --ignored`. Note the failing hash(es), the tests that dumped them and `rustc`'s first error.
2. **Confirm the cause**: revert eeaefb1's hunk that the error points at in the working tree (suspect 1: the `index.rs` impl; suspect 2: the `buffers.nika` change with `lower-std` for that file), re-run only the sweep (step 1's second command over the same dump; for suspect 2 re-dump with the failing test's filter only). Restore the hunk afterwards.
3. **Fix at the source**, one commit, with a `tests/compiler` program that runs the failing shape (*Fix, do not work around*): suspect 1, make the either-text lookup not compete in inference (e.g. the emitter writes `.as_str()` for an `EitherText` key, or the impl moves behind a distinct trait method), keeping eeaefb1's own tests in `tests/compiler/tests` passing; suspect 2, the keep lowering; suspect 3, remove the `KNOWN` entry that compiles now (that is the only `KNOWN` edit allowed: never add one) and say which commit fixed it. Run the sweep again over a fresh dump of the failing test's filter: it passes.

## Done when
The sweep passes over step 1's dump (or a fresh dump, if the fix changed the dumped sources), eeaefb1's tests and the new test pass (`nikaia test` over `tests/compiler`, filter by name), clippy clean, `scripts/pre-push-check.sh` passes. Commit: `<what was wrong> (fix-forward, sweep red since eeaefb1)`, `Fixes` none. Changelog: one sentence if the compiler or `std` changed, else none.

## Stop and report if
The fix is more than one step, or would undo what eeaefb1 decided (then report under `Found:` with the program and the error, `partial`); the cause is in another lane's uncommitted file. Not yours: `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}`, `docs/briefs/k1-comptime-tree.*` and `lane-bugs-557a-span-here.md.run.json` (uncommitted in the checkout).

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-sweep2.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
