---
agent: builder
model: sonnet
max_turns: 40
budget_usd: 6
---
# Brief: fix-forward, clippy 1.99's `chunks_exact_to_as_chunks` on `std`'s SHA-256 (CI red since c80e63ac)

Score: a red `develop` is fixed forward first (`CLAUDE.md`); `main` stands still until then (last green 0a5ea814). One small commit before the moves.

## Facts (traced by the architect, 2026-10-31)
* Commits of the first red clippy run's push (`git log 0a5ea814..c80e63ac`): the cheap suspect is 3ff1505b (s922, `std: hash::sha256_hex, env::var and env::temp_dir`); the rest are briefs, docs, BT, b8 and the grammar guard, all clippy-clean on 1.99.1 once the three lines below are fixed (checked).
* CI's job `bootstrap_compiler`, step *No clippy warnings*, fails since c80e63ac (runs on c80e63ac, 65dda5c6, f0ed954a; 0fc63b20 and 640ab414 stopped earlier at the badges). The cause: `rust-toolchain.toml` says `channel = "stable"`, CI installs **1.99.1**, this machine's `stable` is **1.97.0**. Clippy 1.99 has a new lint, `clippy::chunks_exact_to_as_chunks`, and it fires three times in `crates/nikaia-std/src/hash.rs` (`sha256_hex`, `sha256_block`):
  * :316 `let mut blocks = data.chunks_exact(64);` (then `for block in &mut blocks` and `blocks.remainder()`)
  * :329 `for block in tail[..end].chunks_exact(64)`
  * :370 `for (t, word) in block.chunks_exact(4).enumerate()`
* `slice::as_chunks` is stable since Rust 1.88, which is `std`'s floor (`crates/nikaia-std/Cargo.toml` `rust-version = "1.88"`, `RUST_FLOOR` in `crates/nikaia/src/emit/mod.rs:679`), so the lint's suggestion is usable; no `#[allow]`.
* Toolchain 1.99.1 is installed beside `stable` on this machine: `cargo +1.99.1 clippy ...` reproduces CI. With `-A clippy::chunks_exact_to_as_chunks` the whole workspace is clean on 1.99.1 (checked on 5b97cbe3), so these three lines are all.

## To build
1. In `hash.rs`: `let (blocks, rest) = data.as_chunks::<64>();` and loop over `blocks`; the tail loop over `tail[..end].as_chunks::<64>().0`; `sha256_block` may take `&[u8; 64]` and read its words through `block.as_chunks::<4>().0` (then `u32::from_be_bytes(*word)`). Same digests.
2. Checks: `CARGO_TARGET_DIR=/home/user/nikaia-clippy199 cargo +1.99.1 clippy --all-targets -- -D warnings` (its own target dir, so the shared `target/` is not rebuilt for another rustc; **delete `/home/user/nikaia-clippy199` at the end** and report the GB), and `cargo clippy --all-targets -- -D warnings` on the local stable; `cargo fmt`; `scripts/pre-push-check.sh`; `cargo build --profile fast -p nikaia`; `tests/compiler/tests/hash_sha256.nika` run with `target/fast/nikaia run` from `tests/compiler`, its output `diff`ed byte for byte against `hash_sha256.stdout` (the digest of `"one\n"`; `nikaia test` has no name filter, and the driver runs the whole suite) and `cargo test --profile fast -p nikaia --test it -- cache` (the Rust twin's digests).

## Done when
Both clippy runs are clean, the two tests pass, committed. Commit: `std: sha256 reads its blocks with as_chunks (fix-forward: clippy 1.99, CI red since c80e63ac)`, no `Fixes`. Changelog: none (no meaning moves).

## Stop and report if
1.99.1 shows another warning (name it under `Found:` with file and line), or a digest changes.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-clippy199.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
