---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# pbc3-touch-sharing: touch write-back and sharing infer in Nikaia

Score: the borrowing core goes to Nikaia before the parser (owner, 2026-11-03: the parts a later change to borrowing touches are Nikaia first; then the parser). The decisions are mostly Nikaia already (`tools/keep.nika`, `tether.nika`, `touch.nika`, `sharing.nika`, `text_tiers.nika`); what is left in Rust is listed in `docs/selfhosting-plan.md`, *Borrowing core*. Part of #125.

**Owns:** `crates/nikaia/src/contracts/touch.rs`, `contracts/sharing.rs` `infer`, `crates/nikaia-std/src/tools/touch.nika`, `tools/sharing.nika` and their lowered files, `crates/nikaia/tests/it/touch_sets.rs` (only to move it), new tests under `tests/compiler/`.

**Not yours:** `keep.rs`, `tether.rs` (pbc1, pbc2), `keeps.rs` (another agent), `sharing.rs` `analyse_program` and `report` (they stay: callbacks on `Parsed` and `Ty::from_ast`).

## To build
1. `touch.rs` `infer`: the write-back (dedupe, sort by `order_of`, only when `!touches_known`) and the driving order move to `touch.nika` `touch_write(ledger, ...)`. What stays Rust is the conversion of the resolver's `MethodCalls` (a Rust type) into a Nikaia map, handed in. `sharing.rs` `infer` (writes `contract.sharing`) moves to `sharing.nika` `sharing_infer`. Commit.
2. The `mod tests` of `touch.rs` and `tests/it/touch_sets.rs` (with `TouchOps`/`KINDS`, used only there) become `.nika` tests under `tests/compiler/` with the same cases; delete the Rust tests and `TouchOps` after the `.nika` ones pass. Commit `Part of #125`.

## Done when
`touch.rs` keeps only the `MethodCalls` adapter and its uses (about 25 lines). `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests (the moved cases count once); `it` filtered on `touch`, `sharing`, `contracts`, `ledger` and the guard set; clippy clean; lower-std fixpoint holds. Body names BC-3 as built. No changelog line.

## Stop and report if
`nikaia.contracts`, the shipped ledger or a `.stderr` changes.

Traps of every BC slice: run `lower-std` and commit every lowered file it changed; tools files share one namespace (prefix new functions with the module name); declare functions `sync` (#1014); no mutable tuple-pattern names (NK1307); no `BTreeSet::extend`, no `BTreeMap::entry` (#950): get, then insert; no take out of a `let mut` view (#949): clone or rebuild; never compare contracts by `ptr::eq`; debug the lowering with `nikaia check --tethers`. The ledger order must stay byte-identical: `tests/compiler/nikaia.contracts` and the shipped ledger must not change.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*: absent on purpose (name the ADR): build it that way; not decided or not thought of: stop that piece, keep what is committed, report it with a probe of a few lines; a defect met on the way: fix it in the compiler with a test as its own commit (at most one), a second one goes under `Found:` with its probe.

`rs2nika` is deleted (docs/builder-facts.md): translate by hand and give the three costliest hand edits under `Rs2nika:`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/pbc3-touch-sharing.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
