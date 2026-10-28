---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P4b, a build's foreign boundary of `project.rs` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, largest Rust files first; `project.rs`, ~4,200 lines). Plan row P4, second half (P4a, the members, runs just before this brief and splits the row). S=2, C~3-5 M (comparable: P3a, S2). Prerequisite: P4a (`project_members.nika` and its manifest view); if P4a is not on `develop` (`git log --oneline -5 -- crates/nikaia-std/src/tools/project_members.nika` empty), stop with `Result: blocked`.

## Facts
* Plan row P4 (copied), second half: `Foreign`, `crates_beneath`, `Foreign::of`/`packages`, `sources_that_moved`, `proofs_digest`, `description_at` into `project_members.nika`; `Foreign::library` stays Rust (`contracts::std_library`). Draft share of the row 88%.
* Locations at 27d6d73, `crates/nikaia/src/project.rs` (P4a shifts them: re-grep): `pub struct Foreign` :1635 with `impl Default` :1673, `crates_beneath` :1688, `impl Foreign` :1708 (`of` :1714, `library` :1758, `packages` :1784), `sources_that_moved` :1803, `proofs_digest` :1827, `description_at` :1854. Callers: `project.rs` :1248 (`proofs_digest`), :1372-1373 and :3191-3192 (`Foreign::of` / `default`), :1417 (`packages`); `modules.rs` :1108 (`Foreign::of(root).descriptions`); `Around<'a>` :1876 holds `&'a Foreign`.
* `Ledger` is already Nikaia's (`tools/ty.nika` :1233). Its Rust-side methods (`LedgerOps` in `crates/nikaia/src/contracts/mod.rs`: `empty`/`blank`, `absorb` :305, `read_beside` :864) stay Rust; `crate::describe::crate_root` (`describe.rs` :453) stays Rust.
* `std` has no SHA-256 (Part III 17). The precedent is S2's: a host trait method `fn sha256_hex(ref self, read: ref Vec[u8]) -> String sync` (`SysrootHost`, `tools/sysroot_cache.nika` :50-52), implemented in Rust with `orchestrator::cache::sha256_hex`. Use the same shape: one host trait (e.g. `ForeignHost`) for `sha256_hex`, `read_beside`, `absorb`, `empty_ledger`, `crate_root`; the manifest reads go through P4a's view (`manifest_at` or what P4a named in its commit).
* `std` counterparts checked for the rest: `fs::read`, `fs::read_dir`, `fs::canonical(path, fs::Root::Anywhere)`, `Path.join`/`file_name`, `Set` and `Map` exist. If a moved line needs a `std` call that is missing, stop and report it under `Needs:`.
* Whether `Foreign` becomes a Nikaia struct (then `library` is a Rust free function over it, and `Around` holds the lowered type) or stays Rust filled from a Nikaia result, is yours: name it in `Body:`. Every printed message stays byte for byte.
* Known open lowering defects, not yours: #909 (write `.clone()` where rustc asks), #916 (`ref Bytes?` parameters, a match over a cloned option compared: write a match on the field, as P3b did). Name each use in `Body:`.
* The converter's `Rs2nika:` costs are read by the converter lane: do not change `crates/rs2nika/`.

## To build
1. Run the converter on a scratch file holding exactly the moved Rust (`rs2nika --profile check-free --file`), report `--score` under `Rs2nika:`. Move `Foreign` (all but `library`), `crates_beneath`, `sources_that_moved`, `proofs_digest`, `description_at` into `project_members.nika` with their doc comments; the callers call the lowered functions; the moved Rust is deleted. `lower-std`: only `project_members.rs` and its index lines change.
2. In `docs/selfhosting-plan.md` mark P4b built (`project_members.nika`); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`), except #909 and #916 above; if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia test tests/compiler --both-settings` (every count as before); `it` tests filtered on `foreign describe contracts proofs NK2504 rust_dep` plus the guard set of `.claude/agents/builder.md`; `nikaia --lib project::` passes; clippy clean; lower-std fixpoint. Commit `project: a build's foreign boundary into Nikaia (P4b)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the ones left above; a `.stderr` or message that would change; a missing `std` call; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p4b-project-foreign.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
