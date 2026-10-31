---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: `fs::Metadata::modified`, `links`, `file_id` and `time::since_epoch` in `std` (#922 step 4, Part #125; unblocks K3b, O1)

Score: self-hosting lane. #922 step 4 is what K3b (`comptime_run.rs` `run_fingerprint`, `stamp_of`: `meta.modified()` as nanoseconds since the epoch) and O1 (`cache.rs` `used_at`, `first_link`) wait for. The signatures are decided in #922 (owner, 2026-10-28) and Part III 17.1 already lists `modified` among `Metadata`'s readers (`30-nikaia-tooling.md` :1392); nothing here goes to the owner. First of the batch; K3b runs after it and uses `modified`.

Facts: Where things live, Traps already met, Commands

## Decided (#922; architect, 2026-10-31)
* `fs::Metadata::modified(self: ref fs::Metadata) -> time::Duration sync`, `touches = []`: the last modification since the Unix epoch, following a link as `fs::metadata` does; `time::Duration` zero where the platform does not say or the time is before the epoch (Rust: `.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default()`, the exact chain `comptime_run.rs` :840 and :918 use). `d.in_nanos()` then gives what K3b hashes.
* `fs::Metadata::links(self: ref fs::Metadata) -> u64 sync`, `touches = []`: the number of hard links (`MetadataExt::nlink`); `1` off Unix.
* `fs::Metadata::file_id(self: ref fs::Metadata) -> (u64, u64)? sync`, `touches = []`: device and inode (`MetadataExt::dev`, `ino`); null off Unix.
* `time::since_epoch() -> time::Duration sync`, **no `touches` line** (as `time::now` :4977: a clock read orders against everything): the wall clock, `SystemTime::now().duration_since(UNIX_EPOCH)`, zero before the epoch. ADR-368 amendment, one paragraph: the monotonic clock (`time::now`, an `Instant`) measures spans within a run and never goes back; the wall clock is what a file's time is stamped in, so only it can be compared with `modified`; it can go back, and a program that measures a span with it is wrong. Both exist because neither does the other's job.
* `Metadata` gains three fields (`modified: std::time::Duration`, `links: u64`, `file_id: Option<(u64, u64)>`), filled in `fs::metadata`; its derives stay. ADR-319 amendment, one paragraph: these are readings of the name, no capability beyond the `Root` the `metadata` call already took.

## Locations
* `crates/nikaia-std/src/fs.rs`: `struct Metadata` :420, `impl Metadata` :427-446, `fn metadata` :448-456 (add the readers after `is_file`, `#[cfg(unix)]` for `MetadataExt`, the off-Unix values as above).
* `crates/nikaia-std/src/time.rs`: `fn now` :25 (put `since_epoch` after it); `in_nanos` :121/:140 exists.
* `crates/nikaia-std/std.contracts`: `fs::Metadata::is_file` block :4530-4534 (model the three on it, a `#` comment line above each); `time::now` :4974-4981 (model `since_epoch` on it, no `touches`).
* Part III `docs/specification/30-nikaia-tooling.md`: :1392 (the comment already says `modified`; make it `len, is_dir, is_file, modified, links, file_id`) and a sentence after the code block naming the three readers' off-Unix answers; the `std::time` bullet :1571-1577 (one sentence: `time::since_epoch()` is the wall clock, comparable with `modified`, not monotonic, ADR-368).
* `docs/specification/adr/adr-319.md`, `adr-368.md`: the amendments above, each citing #922.

## To build
1. The four entries in `fs.rs`, `time.rs`, `std.contracts`, Part III, both ADR amendments. Test `tests/compiler/tests/fs_metadata_times.nika` (+ `.stdout`): the program writes a file under `fs::scratch` (or the root the other `fs_*` tests use: `tests/compiler/tests/fs_writes_bytes_and_reads_them_back.nika`), reads `fs::metadata`, and prints only booleans: `modified` is above zero, `modified` is not after `time::since_epoch()` plus one second, `since_epoch` is above 1.7e18 ns, `links() == 1`, `file_id() != null` (the suite runs on Linux), and a second `metadata` of the same file gives the same `file_id`. Commit `std: fs::Metadata::modified, links, file_id and time::since_epoch`, `Part: #922` (step 3, `fs::lock`, stays open).

## Done when
The new test passes with `nikaia test tests/compiler --both-settings` on it; `cargo test -p nikaia-std --lib fs:: time::` and the `it` filters `fs metadata time` pass; clippy (1.97 and 1.99.1, `-p nikaia-std`) clean; `lower-std` leaves every `lowered/*.rs` as committed. The report gives a Changelog line (it changes `std`), `Rs2nika: none`.

## Stop and report if
`std.contracts` refuses a `(u64, u64)?` result or a `u64` reader (`Found:` with the message, `Needs: architect decision`), or a `.stderr` of an existing test changes.

## Not yours
`crates/rs2nika/**`, `crates/nikaia-std/src/tools/lowered/project_members.rs` (uncommitted in the tree, residue of the reverted GR, #1060), every `docs/briefs/*` but your report, and any file `git status` shows modified at your start. If `30-nikaia-tooling.md` holds another agent's hunk, commit only yours (builder-facts *Traps already met*, `git apply --cached`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s922b-metadata-times.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
