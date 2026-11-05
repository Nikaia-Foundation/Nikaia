---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# v1b: ADR-views-3 D11-D13 and the codes into Parts I-III (#1213 step 1, second half)

Score: v1 (39cd60c0, 8a3be238) wrote D1-D10 into the specification and found four points the record left open. The supervisor decided them as D11-D13 of `docs/specification/adr/adr-views-3.md` (read them first) and assigned the codes (`NK1333` a `ref` written in a declaration outside `extern "C"`, `NK1334` an unneeded `keep` (warning), `NK1335` a `from` that does not name a way out's source, `NK1336` a missing `from` where there is no body; `NK2302` stays the unmarked keep). Text only; no compiler change. S=1, C~2 M.

**Owns:**
* `docs/specification/10-nikaia-light.md`, `20-nikaia-advance.md`, `30-nikaia-tooling.md`, `docs/specification/adr/adr-184.md` (a *Changed by ADR-views-3 D11* note on D3 only), `docs/specification/adr/README.md` (index rows), `tests/specification` (`EXPECTED`, `COMPILES.txt`).

**Not yours:**
* Compiler, `std`, packages, examples and `guide/`.
* `adr-views-3.md` and `adr-283.md`: records; a contradiction is reported under `Found:`.
* The async ADRs and Part I 8.x async text (another lane).

## To build
1. D11 and D12 in the text: every `ref Array[T]` of Parts I-III (Part I 2.2 and the `Array` rules near line 317-332, Part II 10.2 and the grammar inputs, Part III `windows`/`chunks` and the byte buffer) becomes `Array[T]` with the view rule of D11 (lent or `mut` parameter a view of a run; `keep` parameter generic over its length; field, element, result a view where a piece of a buffer flows in, else `NK1182` with `Vec[T]` and `Array[T, N]` as ways out; a `comptime` `Array[T]` a view of data the build fixed). Part I 6.5's list of what keeps drops "returns it by value"; a returned parameter is a view `from` it (D12). Commit `Part of #1213`.
2. D13 and the codes: the reserved-words paragraph of Part I says `keep` and `from` are not reserved and mean the mark and the source only in their one position; the noun *keep* for the place a given-up buffer moves to becomes *anchor* throughout Parts I-III (`the caller's anchor`, the `Tethered` row of the table near Part I 6.6), the verb *keep* and the mark stay. Part III's `NK1xxx` row gains `NK1333`-`NK1336`; its `NK2302` entry says the body keeps a parameter not marked `keep` (Part I 6.5); `NK1257` and `NK1261` stay retired. Commit `Part of #1213`.

Write by `docs/writing-guide.md`, last section (compact, one rule in one place, one meaning per word). No new decision in the text; what D11-D13 leave open is reported under `Found:`.

## Done when
No `ref Array` outside `extern "C"` in the specification; no `NK????`; `nikaia run scripts/pre-push.nika` (ADR citations) clean; `tests/specification` counts as after v1 (lowered 58, floor 57), a block that now uses `keep`, `from` or a view `Array[T]` field and does not yet compile listed in `COMPILES.txt` as such. No changelog line. `Gate:` is `green` or `red` (run the gate of `.claude/agents/builder.md` on the text change), never `none`.

## Stop and report if
D11-D13 contradict another live ADR on a point the text must state.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v1b-views3-arrays.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
