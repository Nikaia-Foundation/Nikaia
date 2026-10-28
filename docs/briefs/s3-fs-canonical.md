---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: `fs::canonical` and `Path.strip_prefix` in `std` (Part #125, unblocks P3b and P4)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first). A `std` gap that blocks two planned moves: P3b (`shown_path` in `project.rs`, ~390 lines) and P4 (`canonical`, `rooted` in `project.rs`, ~660 lines), rows in `docs/selfhosting-plan.md`, *Large files*. P3a's report (b8547a6) left `shown_path` in Rust for exactly this. *Fix, do not work around* (`CLAUDE.md`): the gap is closed in `std`, not routed around in the tool.

## Facts
* What is missing (P3a, b8547a6): `shown_path` (`crates/nikaia/src/project.rs` :3856) is `path.canonicalize()`, `root.canonicalize()`, `path.strip_prefix(root)`. Part III 17.1 has neither a canonical name nor a prefix cut on `fs::Path`.
* Decided by the architect (ADR-319 D3 amended, Part III 17.1 text below), no owner question:
  * `p.strip_prefix(base) -> Path?`: pure, on whole parts, no file system. The rest of `p` after `base`'s parts, or `null` where `base` is not its leading parts. `"a/b/c".strip_prefix("a")` is `b/c`; `"a/b".strip_prefix("a/b")` is the empty path; `"ab/c".strip_prefix("a")` is `null`. The name is Rust's, so the converter maps it one to one.
  * `fs::canonical(path: Path, root: Root) -> Path throws`: the absolute name with every link, `.` and `..` resolved; the name must exist (`NotFound` otherwise). Under `Dir(store)` it throws `Outside` where the resolved name leaves the store's own canonical name. Not `async`, not on the reactor (one system call, as `exists`).
* Locations: `fs.rs` path operations `crates/nikaia-std/src/fs.rs` :26-60 (`starts_with`, `ends_with`, `file_name`, `parent`, `with_extension`, `join`: add `strip_prefix` beside them, same shape); `exists` :307 (model `canonical` on it: `resolve(asked, root)?`, then `std::fs::canonicalize`, then the `Outside` check against the store canonicalized); `walk` :1061. How a method on `fs::Path` reaches these functions in the checker and the emitter: find it with `nikaia run scripts/where.nika -- with_extension file_name` and follow `file_name` (it returns `Path?` as `strip_prefix` will). Existing tests of the path operations: `tests/language/src/file_paths.nika`.
* Part III text to add, `docs/specification/30-nikaia-tooling.md` 17.1:
  * in the *Whole-file access* block after `walk`: `pub fn canonical(path: Path, root: Root) -> Path throws         // the absolute name, links and . and .. resolved`
  * in *A path is an `fs::Path`*: add `strip_prefix` to the list of operations, and after that sentence: "`p.strip_prefix(base)` is the rest of `p` after the whole parts of `base`, or `null` where they do not lead it; it reads no file system. `fs::canonical(path, root)` is the absolute name with every link, `.` and `..` resolved; it throws `NotFound` for a name that does not exist and, under `Dir(store)`, `Outside` where the resolved name leaves the store."
  * ADR-319 D3 (`docs/specification/adr/adr-319.md` :74): add `strip_prefix` (handing back `Path?`) to its list, and one line: "Amended 2026-10-28 (P3a): `strip_prefix`, and `fs::canonical` in Part III 17.1, for a name shown relative to a root."

## To build
1. `Path.strip_prefix` and `fs::canonical` in `fs.rs`, reachable from a program and from a tools file; the spec and ADR text above. Test: a new `tests/compiler/fs_canonical.nika` with its `.stdout` (no absolute path in the output: print `strip_prefix` results, `canonical(x).strip_prefix(canonical("."))`, a `null` case, `NotFound`, and `Outside` for a `..` and for a symlink leaving an `fs::scratch()` root; make the link with what `std::fs` offers, else leave that one row out and say so in `Body:`).
2. Move `shown_path` into `crates/nikaia-std/src/tools/project_tests.nika` (prefix `project_tests_` if `where.nika` finds a clash), `project.rs` calls the lowered one through `rt::exec::block_on` only if it is async (it should not be); fall back to the given name where `canonical` throws, as the Rust does. Update P3a's and P3b's rows in `docs/selfhosting-plan.md`: `shown_path` built, the `std::fs` prerequisite of P3b and P4 met. `lower-std`: only `project_tests`' lowered file and its index lines change.

## Done when
`nikaia test tests/compiler --both-settings`; `nikaia test tests/language` filtered on `file_paths` if the runner filters, else the package; `it` tests filtered on `file_paths fs_ test_runner output_test program_test` plus the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. Commits: `std: fs::canonical and Path.strip_prefix` (step 1), `project: shown_path into Nikaia` (step 2), both `Part: #125`. `Changelog:` one sentence for step 1 (a `std` change raises the version: the supervisor's). `Rs2nika:` for step 2.

## Stop and report if
`fs::Root`'s resolve cannot express the `Outside` check without a change to `resolve` that alters another call's errors; a `.stderr` that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s3-fs-canonical.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
