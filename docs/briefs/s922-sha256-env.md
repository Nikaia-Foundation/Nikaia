---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: `hash::sha256_hex`, `env::var`, `env::temp_dir` in `std` (#922 steps 1-2, Part #125; unblocks GR, K3b, O1)

Score: self-hosting lane. #922 steps 1-2 block GR (`grammar_run.rs`: `crate::assets::digest`, `std::env::var("CARGO")`), K3b and O1 (`docs/selfhosting-plan.md`), and four tools files carry a host `sha256_hex` method only because `std` has none. The signatures are decided in #922 (owner, 2026-10-28); what this brief adds is decided by the architect, no owner question.

Facts: Where things live, Traps already met, Commands

## Decided (architect, 2026-10-31)
* **SHA-256 is written out in `crates/nikaia-std/src/hash.rs`: no `sha2` dependency.** #922's table says "`sha2` (already a dependency of the toolchain)": it is a dependency of `nikaia` and `orchestrator`, not of `nikaia-std`, and `std` is built into every user program (`crates/nikaia-std/Cargo.toml` :116, ADR-002 D4 keeps that graph small). `hash.rs` already copies rustc-hash's byte hash for the same reason. That is FIPS 180-4's compression function: about 80 lines, no `unsafe`, no table but the 64 round constants. The NIST test vectors are the oracle, in a Nikaia test.
* **`env::var` declares no `touches` line.** Part III's contract table (`30-nikaia-tooling.md` :332) says an absent `touches` means *touches everything*, so a read of the environment keeps its order against every other effect. That is correct (another call can set the variable), and it needs no new vocabulary. `env::temp_dir` is the same. Both are `sync`.
* `env::var(name: ref String) -> String? sync`: null when the variable is unset or not Unicode, and also when it is empty (`cache.rs:742` and `project.rs:562` treat empty as unset). `env::temp_dir() -> fs::Path sync`: `std::env::temp_dir()`.
* `hash::sha256_hex(data: ref Vec[u8]) -> String sync`: 64 lowercase hex characters. It equals `orchestrator::cache::sha256_hex` byte for byte.

## Locations
* `crates/nikaia-std/src/hash.rs` (`crc32c` :247, the pattern for a `pub fn` that a program reaches); `crates/nikaia-std/src/lib.rs` :33-76 (`pub mod`: add `env` in order); `crates/nikaia-std/std.contracts` :1855-1866 (the `hash::crc32c` block: model both on it, with a `#` comment line above each `[fn."…"]`, no `touches` line for the `env` pair); `cli::args` :131 for a `sync` note.
* Part III `docs/specification/30-nikaia-tooling.md`: :871 (`hash::crc32c`'s paragraph: put `sha256_hex` after it); 17.1 :935 (Universal Modules: add an `std::env` entry beside `std::cli` :1477, saying null for unset, empty or not Unicode, and that it orders against everything).
* Where the host methods are: `nikaia run scripts/where.nika -- sha256_hex`, which gives `sysroot_cache.nika:52`, `project_settings.nika:255`, `project_members.nika:351` and `modules.nika:903`, their Rust impls, and `orchestrator/src/cache.rs:46`.

## To build
1. `hash::sha256_hex` in `hash.rs`, `std.contracts`, Part III. Test `tests/compiler/tests/hash_sha256.nika` (+ `.stdout`) printing the digests of `""`, `"abc"`, the 448-bit NIST message `"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"`, `"one\n"` (as `crates/nikaia/tests/it/cache.rs:197` has it) and a 1000-byte message that crosses block boundaries. Take the expected values from `sha256sum`, not from your code. Commit `std: hash::sha256_hex`.
2. `env::var`, `env::temp_dir` in a new `env.rs`, `std.contracts`, Part III. Test `tests/compiler/tests/env_var.nika` (+ `.stdout`): a child started through `process::run(…, env: [("NIKAIA_PROBE", "x"), ("NIKAIA_EMPTY", "")])` that prints `env::var` of both and of an unset name (x, null, null). If the test runner cannot start the test's own binary as a child, use `nikaia test`'s environment instead (read a variable the runner sets, `where.nika -- NIKAIA_` in `crates/nikaia/src/test_runner*`). Print no absolute path; for `temp_dir` print only `env::temp_dir().is_dir()`. Commit `std: env::var and env::temp_dir`.
3. **Retire the four host `sha256_hex` methods.** Delete the trait method in each tools file and call `hash::sha256_hex` (for `ref String`, its `.bytes()`/`as_bytes` form, whichever `std` has; `where.nika -- as_bytes`). Delete the Rust impls. Run `lower-std` and the fixpoint: only the four `lowered/*.rs` and their index lines change. `orchestrator::cache::sha256_hex` stays (O1 moves it). Commit `tools: the host sha256_hex goes, std's is called`.

## Done when
The two new tests pass with `nikaia test tests/compiler --both-settings` (the gate's `gate_test_some` on them). The `it` filters `cache sysroot project_settings modules members` pass, plus the guard set of `.claude/agents/builder.md`. clippy is clean, and the lower-std fixpoint holds. In the report: a Changelog line for steps 1-2 (both change `std`, so the supervisor raises the version), and `Fixes:` none (steps 3-4 of #922 stay open). The report says `Rs2nika: none` (no Rust moved by conversion).

## Stop and report if
A tools file cannot call a `std` function from `hash`/`env` (lower-std refuses it, or the lowered call does not resolve): `Found:` with the lowered line. Also stop if `std.contracts` refuses an entry without `touches`, or a `.stderr` of an existing test changes.

## Not yours
`crates/rs2nika/convert/nikaia.{contracts,derived,proofs}`, `crates/nikaia-std/src/tools/check_core_{flow,leaves,stmt}.nika`, `check_takes.nika` and their lowered twins (the bug lane's b8), the other briefs and reports in `docs/briefs/`, and any file `git status` shows modified at your start. `docs/specification/30-nikaia-tooling.md` is modified at the time of writing (another agent's hunk near :1746): commit only your hunks, as builder-facts *Traps already met* says (`git apply --cached`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s922-sha256-env.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
