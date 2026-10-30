---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #995, `a ?? m.get(k)` with an owned `a: T?` reaches rustc

Score: bug lane; blocks K3a part 2 (`k3a2-comptime-compute.md`, next in the batch: `known(name) ?? ahead.get(name)` in `computed`). S=1, a lowering defect on the self-hosting path. Builders have no GitHub: the issue text is copied below.

## Problem

`a ?? m.get(k)`, where `a: T?` is owned and `m.get(k)` is a map's element, passes the checker and reaches rustc: the fallback lowers as `Option<&T>` beside the owned `Option<T>` (E0308). `return m.get(k)` alone as `i64?` runs (the lowering copies it), so only the `??` fallback position misses the copy. K3a part 2 (#125, `comptime_run.rs::computed`) needs exactly this form: `known(name) ?? ahead.get(name)` over `BuildValue`.

## Probe (target/fast at about 0.0.1305, 2026-10-30)

```nika
use std::collections

fn first(a: i64?, m: collections::BTreeMap[String, i64], k: String) -> i64? {
    return a ?? m.get(k)
}

fn word(a: String?, m: collections::BTreeMap[String, String], k: String) -> String? {
    return a ?? m.get(k)
}

fn main() {
    let mut m: collections::BTreeMap[String, i64] = collections::BTreeMap()
    m.insert("b", 2)
    let mut w: collections::BTreeMap[String, String] = collections::BTreeMap()
    w.insert("b", "two")
    print((first(null, m, "b") ?? 0).to_string())
    print(word(null, w, "b") ?? "none")
}
```

rustc: `Expected Option<_>, found Option<&_>` at :4 (help: `copied`) and `Expected Option<String>, found Option<&String>` at :8 (help: `cloned`). Expected: `2`, `two` - or, for the text and a struct, a refusal by the checker with a `.clone()` help if ADR-282 D7 says that copy is the program's to write. Never rustc.

A plain `return m.get(k)` as `String?` from a lent map is a defect of its own (#996), not this one's.

## Locations

* Checker: `check_core_ops_coalesce`, `crates/nikaia-std/src/tools/check_core_ops.nika:752`.
* Lowering: `emit_core_coalesce`, `crates/nikaia-std/src/tools/emit_core_flow.nika:651`; how a plain `return m.get(k)` gets its copy: follow `emit_core_*` for `Expr::MethodCall` `get` on a map (`nikaia run scripts/where.nika -- emit_core_coalesce`).
* Tests: new `tests/compiler/coalesce_map_get_fallback.nika` with `.stdout`.

## To build

1. Re-run the probe on a fresh `cargo build --profile fast -p nikaia`. Passes: `Result: noop`, the probe as a test anyway (pins it).
2. Check ADR-282 D7/D9 for whether the copy of a text or struct element is written by the compiler at this meet or refused; numbers are copied either way (as `return m.get(k)` already is). Build that in the lowering (or the checker's refusal), for numbers, text and a struct value (`BuildValue`-like enum with a `String` part).
3. Test (the three element kinds, also inside a lambda: `let wider = fn(n) { known(n) ?? more.get(n) }` over a local map, handed on as `fn(String) -> T? sync`); `lower-std` fixpoint; the `it` guard set. Commit `Fixes #995`.

## Notes

Read `docs/builder-facts.md`; `CLAUDE.md` rules for builders; report form of `docs/architect.md`, *Briefs and reports*.
* Do not touch: the uncommitted files `git status` shows at start (`crates/rs2nika/convert/nikaia.*`, other reports): another session's. Changelog: one sentence (a lowering change).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b995-coalesce-map-get.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
