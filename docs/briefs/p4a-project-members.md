---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P4a, a package's members of `project.rs` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, largest Rust files first; `project.rs`, 4,310 lines). Plan row P4 (`docs/selfhosting-plan.md`, *Large files*) is ~660 lines, too large for one run (M1's first run was cut by the driver's 30-minute limit), so it is split: P4a now (the members, ~400 lines), P4b later (`Foreign`, `crates_beneath`, `Foreign::of`/`packages`, `sources_that_moved`, `proofs_digest`, `description_at`). S=2, C~4-6 M (comparable: P3a). Prerequisite met: `fs::canonical`, `Path.strip_prefix` (f861426).

## Facts
* Plan row P4 (copied): `packages_of`, `renames_in`, `Member`, `dependencies_first`, `members_of`, `rooted`, `canonical`, `crate_root_of` into new `crates/nikaia-std/src/tools/project_members.nika`; added: `manifest_at(root) -> ManifestView?` (name, path dependencies, foreign crates; `Manifest` and `toml` stay Rust), `canonical(path) -> String`; `Member.manifest` becomes the view, `drive_to` reads the Rust `Manifest` again where it needs `codegen_for`. Draft share 88%.
* Locations at 33f9df4, `crates/nikaia/src/project.rs`: `packages_of` :713-806, `renames_in` :807, `pub struct Member` :836, `dependencies_first` :869, `members_of` :921-991, `rooted` :992 (takes and returns `toml::Value`: stays Rust), `canonical` :1011 (`fs::canonical(path, fs::Root::Anywhere) catch { path.clone() }`, as `shown_path` in `project_tests.nika` :62), `crate_root_of` :1022. Re-grep before cutting; `where.nika -- packages_of members_of Member` for the callers.
* `std` counterparts checked (Part III 17): `fs::read_dir`, `fs::canonical`, `Path.join`/`extension`/`display`/`parent`/`file_name` exist; `is_file` is `project_tests_is_file` (`project_tests.nika` :81, via `metadata`). `Path.is_absolute` and `with_file_name` have none, but they are only in `rooted` and `packages_of_file`, which stay Rust. If a moved line needs a `std` call that is missing, stop and report it under `Needs:` (it is its own `std` brief first).
* The `Manifest` view or callbacks: `Manifest::read`, `.dependencies()`, `.package_name()`, `.foreign_crates()`, `.has_build_section()` stay Rust; Nikaia gets them as a view struct built by Rust or as callbacks (`fn(..) -> .. sync`, as M2's `intern`). Which, and whether `Member` becomes a Nikaia struct or stays Rust with Nikaia returning its parts, is yours: name it in `Body:`.
* The converter's `Rs2nika:` costs are read by the converter lane: do not change `crates/rs2nika/`.

## To build
1. Run the converter on a scratch file holding exactly the moved Rust (`rs2nika --profile check-free --file`: the code holds no `Checker`, `Emitter` or `self`), report `--score` under `Rs2nika:`. Move the functions (all but `rooted`) into the new `project_members.nika` (prefix `project_members_` where `where.nika` finds a clash) with their doc comments; `project.rs` calls the lowered functions; the moved Rust is deleted. `lower-std`: only the new lowered file and its index lines change; add the file to `scripts/self_hosting.py` as P3a did.
2. In `docs/selfhosting-plan.md` split row P4 into P4a (built, `project_members.nika`) and P4b (the rest, as named above, prerequisite P4a); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`); if it is more than one step, stop and report it under `Needs:`. Known open checker defect, not yours: #909; write `.clone()` where rustc asks for it and name it in `Body:`.

## Done when
`nikaia test tests/compiler --both-settings` (every count as before); `it` tests filtered on `package dependenc member manifest workspace path_dep rename` plus the guard set of `.claude/agents/builder.md`; `nikaia --lib project::` passes; clippy clean; lower-std fixpoint. Commit `project: a package's members into Nikaia (P4a)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the ones left above; a `.stderr` or message that would change; a missing `std` call; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p4a-project-members.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
