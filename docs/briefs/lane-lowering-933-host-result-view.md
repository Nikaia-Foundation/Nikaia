---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---

Owns: `text_tiers.rs`, `views.rs`, `tools/{text_tiers,views,tiers,lends}.nika`, their `lowered/*.rs`, new `tests/compiler` files. Issue #933 (copied below). Score: compiler-defect lane; unblocks P5a rest.
# Lowering: a `String` returned by a trait method on a generic `ref H`, then viewed, lowers as a view of the parameters

Found by P5a (2e13458, `project.rs` settings into Nikaia). Blocks the move of `root_tag`, `program_name`, `own_copy` (plan row P5a, `docs/selfhosting-plan.md`).

## Probe
```
trait Hasher { fn hex(self, seed: String) -> String }
fn tag[H: Hasher](seed: String, host: ref H) -> String {
    let hex = host.hex(seed)
    return hex.trim()
}
```
Lowers to `fn tag<'r, H>(seed: &'r str, host: &'r H) -> &'r str`; rustc: *cannot return value referencing local `hex`*. With `host: ref Plain` (a concrete type) the result is `-> &'static str`, also wrong. The result is a view of a local owned `String`, not of a parameter: the function must return an owned `String` (the checker's own reading of `-> String`).

## Locations
Lowering lane (`docs/bootstrap-week.md` section 3): the result tier is decided in `text_tiers.nika` / `views.nika` (Rust `text_tiers.rs`, `views.rs`); resolve with `nikaia run scripts/where.nika -- NAME`.

## To build
1. A `tests/compiler` program with the probe (generic and concrete host) and its `.stdout`; it fails today in rustc.
2. Fix: a value returned from a method call's result (an owned `String` local) is not a view of the parameters; the function lowers `-> String`. Run `tests/compiler`, the `it` filters `views text_tiers lent`, `lower-std` fixpoint.

Report form: `docs/briefs/common.part`.

Work in checkout C3 (`/home/user/nikaia-fix`, `docs/bootstrap-week.md` section 3); before pushing, `git fetch origin develop && git rebase origin/develop`. The commit says `Fixes #933`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/lane-lowering-933-host-result-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
