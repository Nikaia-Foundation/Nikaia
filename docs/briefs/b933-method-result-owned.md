---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief #933: a `String` returned by a method call, then viewed, lowers as a view of the parameters (rustc: cannot return value referencing local)

Score: bugs before moving (`CLAUDE.md`, `docs/priorities.md`); a wrong lowering that reaches rustc, and it blocks the rest of P5a (`root_tag`, `program_name`, `own_copy` in `project.rs`, plan row P5a of `docs/selfhosting-plan.md`). Lowering lane only (`views.nika` / `text_tiers.nika`); the second brief of this batch is in the checker (`check_*`).

## Issue text (#933, copied; builders have no GitHub; probe corrected 2026-10-29)
Found by P5a (2e13458). A function `-> String` that returns a view (`trim()`) of a local `String` it got from a method call lowers its result as a view of its parameters. Probe (re-run on 6ccf6c8, both functions fail in rustc with *Cannot return value referencing local variable `hex`*):

```nika
trait Hasher { fn hex(ref self, seed: ref String) -> String }
struct Plain {}
impl Hasher for Plain { fn hex(ref self, seed: ref String) -> String { return f"  {seed}  " } }
fn tag[H: Hasher](seed: String, host: ref H) -> String {
    let hex = host.hex(seed)
    return hex.trim()
}
fn tag2(seed: String, host: ref Plain) -> String {
    let hex = host.hex(seed)
    return hex.trim()
}
fn main() {
    let p = Plain {}
    println(tag("ab", p))
    println(tag2("cd", p))
}
```

Expected output `ab` and `cd`. The issue saw `fn tag<'r, H>(seed: &'r str, host: &'r H) -> &'r str` for the generic host and `-> &'static str` for the concrete one. The result is a view of a local owned `String`, not of a parameter: the function must lower `-> String` (owned), which is the checker's own reading of `-> String`. (The issue's first probe had `fn hex(self, seed: String)`; the checker refuses that with NK1102 for an unrelated, correct reason, so the probe above uses `ref self` and `ref String`.)

## Locations (resolved 2026-10-29)
* The result's sources (which parameters a returned value reaches): `crates/nikaia-std/src/tools/views.nika`, the `rsrc_*` family, `rsrc_eval` :1922, `rsrc_eval_node` :1961, `rsrc_bind` :1812. A method call's result is what to look at: an owned `String` result reaches no parameter, even when its receiver (`host`) or argument (`seed`) is one.
* The result tier: `crates/nikaia-std/src/tools/text_tiers.nika`, `called_result` :1704, `method_result` :1711, `flow_result` :2149, `result_kinds` :3903.
* Lowered twins: `crates/nikaia-std/src/tools/lowered/views.rs`, `lowered/text_tiers.rs` (regenerate with `target/fast/nikaia lower-std`, never edit by hand).
* Other names: `nikaia run scripts/where.nika -- NAME`.

## To build
1. Test `tests/compiler/tests/a_text_from_a_method_call_viewed_and_returned_is_owned.nika` with the probe and its `.stdout` (`ab\ncd\n`; `println` ends each line). Run it once on the unfixed compiler and see it fail in rustc; say so under `Tests:`.
2. Fix in the lowering so the result reaches no parameter through a method call whose result is an owned `String`; both functions lower `-> String`. Then `target/fast/nikaia lower-std` twice: the second run changes nothing (fixpoint), and only the lowered files of the `.nika` you changed differ. Commit `Fixes #933`.

## Done when
The new test passes run through `target/fast/nikaia test --project tests/compiler` filtered to its name (not only with `nikaia run`: the test's `.stdout` is compared byte for byte, 7c1643d went red on a newline), `cargo test -p nikaia --test it -- views text_tiers lent`, the gate of `.claude/agents/builder.md` (a change to the lowering runs `nikaia test --both-settings --project tests/compiler`), clippy, `cargo fmt`, `scripts/pre-push-check.sh`. Changelog: one sentence.

## Stop and report if
The fix would make a result that really is a view of a parameter (`fn first(s: String) -> String { return s.trim() }`) owned; or it needs a file another agent has uncommitted (`git status` first: `crates/rs2nika/convert/nikaia.*` and `docs/briefs/k1-comptime-tree.*` were another session's at the time of writing).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b933-method-result-owned.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
