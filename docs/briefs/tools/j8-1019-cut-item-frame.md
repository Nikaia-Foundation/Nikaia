---
agent: builder
model: opus
max_turns: 70
budget_usd: 18
---
# Brief: winnow-grammar numbers a Cut error in a repetition as it numbers a Backtrack one (#1019 step 3d, backend half)

Facts: Traps already met

Score: unblocks j6 (`at $.contacts[1].mail`, #1019 step 3d, ADR-344 D3's error form of the typed read); S=A, O=~0.4M tokens, C=~0.3M tokens; five match arms in the backend `winnow-grammar`, nothing in Nikaia.

## Problem
j6 run 1 (report `j6-1019-error-path.md.run1.report.md`) found that winnow-grammar rev 4f8e6691 pushes the `item N` frame (`Diagnostics::item`, `src/error.rs` 693/769/833) onto an error leaving a repetition only when it is `ErrMode::Backtrack`; an `ErrMode::Cut` takes `Err(e) => return Err(e)` and loses the index. Every failure inside `members[S]` is a Cut (`k:Key => ...`, `fail`), so a typed read cannot say which list element failed. A Cut error loses what a Backtrack error keeps: the backend is inconsistent with itself.

## Decision (architect, 2026-10-31, `docs/architect.md` *Language decisions*: obvious; no language change)
* Wherever a repetition numbers a Backtrack error with `e.item(k)`, it numbers a Cut error leaving the same element with the same `k` and returns it as a Cut (`Err(ErrMode::Cut(e.item(k)))`), and likewise for the streamed folds' `Fail` (their Cut arm, if they keep one). Nothing else changes: no new pattern, no change to `record`, priorities or the rendered form.
* Cost: on the failure path only; a parse that succeeds runs the same code. A grammar without a cut inside a repetition prints exactly as before.
* Rejected: a pattern pushing a frame computed while running (`frame(expr, p)`, j6's option b): a new grammar form for one user, where the existing frame needs only to stop vanishing. Telling which repetition a frame came from is Nikaia's emitter's (a marker rule around a list element, j6 run 2), not the backend's.

## Locations
Work in your own clone of the backend, never in `~/.cargo/git/checkouts/` (read-only) and never through a `[patch]` in the Nikaia checkout (shared; other lanes build there):
`git clone https://github.com/keywan-ghadami/winnow-grammar /home/user/winnow-grammar && git -C /home/user/winnow-grammar checkout -B main 4f8e6691` (HEAD of `main` is 4f8e6691; if `origin/main` has moved, build on it and say so in `Body:`).
* `src/rt.rs`: `repeat_recording_bounded` (958, the arm at 1005), `repeat_counting_bounded` (1043, arm at 1082), the repetition macro around 1248 (`e.item(base + seen + 1)`, calls `stopped(..)` first: keep that order for a Cut too). `repeat_recording` (935), `repeat_counting` (1032): check whether they delegate or have arms of their own.
* `src/rt/stream.rs`: 515 (`Fail::Backtrack(e)` in the item loop, `e.item(*n + 1)`), 868 (`e.item(item.unwrap_or(0))`): read the `Fail` enum first; number a Cut there only where an index is known.
* Tests: `tests/cut_test.rs`, `tests/diagnostics.rs` (existing `item N` expectations), `tests/frames_test.rs`; their `CHANGELOG.md` `[0.1.0] - Unreleased` takes one entry.

## To build
1. The five arms above (and any other `e.item(` site `grep -n "\.item(" src` finds) number a Cut error. Test, a new `#[test]` in `tests/cut_test.rs` (winnow-grammar is a Rust crate; Nikaia's *tests in Nikaia only* rule is for this repository): a grammar `list = "[" item ("," item)* "]"; item = "{" => digit+ "}"` (in the crate's own syntax, `SYNTAX.md`) on `[{1},{2},{x}]` renders a stack holding `item 2` above `item`, and on `[{x}]` holds no `item` frame from that repetition; a repetition `("a" => "b")*` on `ababac` numbers its Cut `item 3`. Each fails on 4f8e6691.
2. `cargo test` in the clone passes (fix an existing expectation only where it now rightly gains an `item N` line, and name it in `Body:`); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` there.
3. Commit in the clone (`git -C /home/user/winnow-grammar commit`, message in its CHANGELOG's style, the `Attribution:` lines below as trailer). Do not push (no GitHub here): the main session pushes it. Write `git -C /home/user/winnow-grammar format-patch -1 -o /home/user/Nikaia/docs/briefs/tools/` and name the file under `Files:`, so the commit survives the clone.
4. Nothing changes in the Nikaia checkout but that patch file and your report: do not move the pin in `Cargo.toml` (j6 run 2 does, after the push).

## Done when
`cargo test` green in the clone with the new test failing on 4f8e6691 (say so under `Tests:`); the patch file in `docs/briefs/tools/`; the report says `Pushed: winnow-grammar <sha>` (local, unpushed) and `Needs: help from main session: push winnow-grammar <sha> to keywan-ghadami/winnow-grammar main`. Commit the patch file and the report in Nikaia with `target/fast/nikaia run scripts/commit.nika -- REPORT` (its `Pushed:` rewrite is fine; keep the winnow-grammar sha in `Body:`). Changelog: none (no Nikaia change yet).

## Stop and report if
a Cut arm cannot be numbered without changing what `record` or the error ranking does (say which function, `Needs: architect decision`); an edit outside the clone, the patch and the report. Delete `/home/user/winnow-grammar/target` at the end (the clone itself stays for the main session) and report GB freed. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j8-1019-cut-item-frame.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
