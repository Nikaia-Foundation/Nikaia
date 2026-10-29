---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 15
---
# Brief #943: a mutable local set from a text literal and changed in place is `String?` in `tools/*.nika` (NK1125, NK1104)

Score: bugs before moving (`CLAUDE.md`, `docs/priorities.md`); a wrong refusal every move into `tools/*.nika` meets (M3b found it), and the message names a type the author never wrote. One file area (the check), apart from the fix-forward's `tools/fmt.nika`.

## Issue text (#943, copied; builders have no GitHub)
Found by M3b (093923c, 2026-10-29). In a `tools/*.nika` file (compiled by `nikaia lower-std`), a mutable local initialised from a text literal and then changed in place is refused:

```nika
pub fn modules_probe_a() -> String {
    let mut out = "a"
    out.push_str("b")
    return out
}
```

`nikaia lower-std` refuses it with NK1125 (*A `String?` may be missing*) and NK1104 (*returning `String?`*). The same lines in a user program run with `nikaia run` are accepted. Writing the type (`let mut out: String = "a"`) gets past it; that is the house spelling (445 uses), but the refusal is wrong: nothing here can be missing.

Probe: (1) add the function to any `crates/nikaia-std/src/tools/*.nika` and run `target/fast/nikaia lower-std`: NK1125 + NK1104; (2) the same function plus `fn main() { println(modules_probe_a()) }` run with `nikaia run`: prints `ab`.

## Locations (resolved 2026-10-29)
* `lower-std`'s check of one module: `crates/nikaia/src/sysroot.rs:281` `lower_std_module`, which calls `crate::check::check_against(..)` with its own settings (about :302) and `refuse_assumes_in_std`; compare those settings with what `nikaia run` passes (the std/tools mode, the text tiers: `tools/text_tiers.nika`, a literal's tier under the tools profile).
* NK1125 is raised in `crates/nikaia-std/src/tools/check_refusals.nika:154` (doc at :114, `reaches_through_a_plain_value`); NK1104 beside it.
* Tests of NK1125: `tests/compiler/tests/a_struct_field_without_its_null_arm_stays_nullable.*`, `crates/nikaia/tests/it/nullable.rs` (where.nika).

## To build
1. Find why the literal's local is typed `String?` under `lower-std` and not under `nikaia run`, and fix it in the checker (or the tools-profile typing it uses) so both accept the probe. Test: `tests/compiler/tests/a_text_literal_local_changed_in_place_is_text.nika` with the probe (`.stdout` `ab`). Also put the probe into a scratch copy of a `tools/*.nika` file, run `lower-std`, see it pass, and restore the file by name (`git checkout -- <that file>` and its lowered `.rs`); say so under `Tests:`. Run `lower-std` once more on the clean tree: no lowered file but those your fix changes.
2. Commit `Fixes #943`.

## Done when
The new test passes (`target/fast/nikaia test --project tests/compiler` filtered to it), the NK1125/NK1104 tests above pass (`it -- nullable` and their `tests/compiler` programs), the gate of builder.md (a change to `check/` or `tools/check_*.nika` runs `nikaia test --both-settings --project tests/compiler`), clippy, `cargo fmt`, `scripts/pre-push-check.sh`. Changelog: one sentence.

## Stop and report if
The difference is a deliberate rule of the tools profile written in Part I-III or an ADR (then `Owner:` names it and the choice), or the fix needs a file another agent has uncommitted (`git status` first: `tools/view_changes.nika` and `crates/rs2nika/convert/nikaia.*` were another session's at the time of writing).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b943-literal-local-push.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
