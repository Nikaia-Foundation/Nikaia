---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: M1, the program assembly of `modules.rs` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, largest Rust files first; #125, `docs/selfhosting-plan.md` *Large files 2*, row M1, the first slice it names). S=2 (~380 Rust lines, converter draft 71%), C~4-6 M (comparable: P2, e51e3e2, 341 lines in one run).

## Facts
* Plan row M1 (copied): `modules.rs`: `Program::of` (the dependency ledgers merged into `library`, `contracts`, `as_its_own`), `shown_paths`, `could_promise`, `lost_sync`, `contracts_as_promised`, `package_names`, `is_single_file`; `declared_once`, `declared_names`, `groups`, `as_promised`. Target `crates/nikaia-std/src/tools/modules.nika` (exists: `declared_names`, `could_promise`, `pause_path`, `lost_sync_unplaced` are there; the Rust halves of these are adapters to finish, not new code). Host added: `package_ledger(group) -> Ledger` (contract inference and the shipped ledger stay Rust), `render_finding(finding, path, source) -> String` (`diagnostics`); the own groups' loop stays Rust (it holds `Vec<Checked>`), Nikaia merges the ledgers it returns; `Program`/`Unit` stay Rust structs (they hold `Parsed`) and pass `package`, `renames`, paths.
* Locations at fa57fcc, `crates/nikaia/src/modules.rs`: `declared_once` :304, `declared_names` :337, `groups` :414, `as_promised` :510, `struct Program` :535, `Program::of` :675-837, `shown_paths` :838, `could_promise` :859, `lost_sync` :876-941, `contracts_as_promised` :942, `package_names` :955, `is_single_file` :963. `Ledger` is `ty.nika`'s (`crate::contracts::Ledger` re-exports it).
* Known risks (plan): `library` grows group by group (a dependency's ledger is absorbed before the next is inferred against it), so the inference callback is called inside the Nikaia loop, not before it. The converter marks every `Ledger` parameter `// ADAPTER: bleibt in Rust` (its rule R58) although `Ledger` is Nikaia's: delete those markers by hand and name it under `Rs2nika:`; do not change the converter (another lane's file).
* What stays Rust (plan, *Stays Rust*): `collect*`, `package_at`, `collect_one`, `read_unit`, `check_imports`, `one_namespace`, `package_ledger*`, `shipped_ledger`, `sources_of`, `emit`, `emit_reading`, `foreign_beside`, `described_beside`; `Unit`, `Program`, `Dependency`, `TestCase`. The `Item::Test` retain and `type_rules::across_files` at the top of `of` stay Rust if they need `&mut Parsed`.

## To build
1. Run the converter on a scratch file holding exactly the moved Rust (`rs2nika --profile check --file`), report `--score` under `Rs2nika:`. Move the listed functions into `modules.nika` (prefix new public names `modules_` where `where.nika` finds a clash), with their doc comments (they carry the ADR references). `modules.rs` calls the lowered functions; the moved Rust is deleted. `lower-std`: only `modules`' lowered file and its index lines change.
2. Mark M1 built in the plan's table (one line, as P2's row), update `modules.nika`'s line in `scripts/self_hosting.py` only if its text needs it; report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`); if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia test tests/compiler --both-settings`; `it` tests filtered on `package depend promise sync use_ declared test`; the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. Commit `modules: the program assembly into Nikaia (M1)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice; a `.stderr` that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m1-modules-program.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
