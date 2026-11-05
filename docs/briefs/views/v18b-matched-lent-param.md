---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v18b: a lent parameter a `match` takes apart stays lent; a lent `String` argument passes as `&str` (#1213, split from v18, after v18a)

Score: ADR-views-3 (D1: a plain parameter is lent, D4: `ref` leaves the language outside `extern`). Read `v18-matched-lent-param.debrief.md` and `v17-migrate-docs-packages.md.report.md` (`Found:` (1) and (4)) beside this brief. v18a has fixed two rustc error classes the keep change exposed (a lent parameter at a method's parameter; a `ref` pattern over a lent subject).

Facts: Checker and views, Lowering and lower-std, Tests and the gate

**Start from** `v18-matched-lent-param.partial.patch` beside this brief: v18's uncommitted work. Its `keeps.nika` part (a matched parameter stays lent; names an arm binds alias the parameter; `MatchPattern::Named` and `RestNamed` in `names_a_pattern_binds`) and its three tests are the base of step 1; its `check_references.nika` part (`check_reference_a_library_text_view`, never compiled) is the base of step 2. Its 42 lowered files are not: regenerate them. Apply with `git apply --include=<path>` per file you take.

**(1) Probes.** Each must check and run without `ref`:
```nika
enum Json {
    Text(String),
    Array(Vec[Json]),
    Num(i64),
}

fn longest(value: Json) -> i64 {
    match value {
        Json::Array(items) => return items.len() as i64
        else => return 0
    }
}

fn main() {
    let j = Json::Array([Json::Text("ab"), Json::Num(3)])
    println(longest(j))
    println(longest(j))
}
```
prints `2` twice. An arm that moves the payload out keeps the parameter (so this is refused at the second call, as with a by-value parameter):
```nika
fn items_of(value: Json) -> Vec[Json] {
    match value {
        Json::Array(items) => return items
        else => return []
    }
}
```
A named-field pattern and a rest pattern read only, so they stay lent (`5` twice, then `5` twice):
```nika
enum Shape {
    Named { label: String, size: i64 },
    Plain(i64),
}

fn size_of(shape: Shape) -> i64 {
    match shape {
        Shape::Named { label, size } => return size + label.len() as i64
        Shape::Plain(n) => return n
    }
}

fn first(xs: Vec[i64]) -> i64 {
    match xs {
        [h, ..rest] => return h + rest.len() as i64
        else => return 0
    }
}

fn main() {
    let s = Shape::Named { label: "abc", size: 2 }
    println(size_of(s))
    println(size_of(s))
    let v = [4, 5]
    println(first(v))
    println(first(v))
}
```
v18 also met a move out of a lent parameter in the lowered `prove_run.rs` once the rule flipped: the tool's `.nika` that moves out of a now-lent parameter is fixed in that `.nika` (it keeps, or it clones where the language says so), never in the lowered file.

**(4) Probe** (`built.push_str(html::escape(name))` fails in rustc with *Mismatched types*; `html::escape` is std's only `-> ref String` and returns a `Cow` in Rust; the no-`&` decision is in `the_compiler_writes_the_reference`, `check_references.nika`):
```nika
use std::html

fn emphasised(name: String) -> String {
    let mut built = String()
    built.push_str("<em>")
    built.push_str(html::escape(name))
    built.push_str(name)
    built.push_str("</em>")
    return built
}

fn main() {
    println(emphasised("a<b"))
    println(emphasised("Ada"))
}
```

**Owns:** `keeps.nika`, `check_references.nika` and the checker files the rule needs, the tools `.nika` a flipped signature breaks, the lowered files `lower-std` changes, the new tests in `tests/compiler/`, `examples/escaping` (drop its `ref` once it works), the partial patch (delete it in your last commit).

**Not yours:** the migration tool (v19), the rest of the examples and tests (v20).

## To build
1. **(1)**: the keep rule. Before any build: one `lower-std`, then `git diff --stat crates/nikaia-std/src/tools/lowered/` and a first rustc cycle; sort the errors into classes. A class v18a fixed that shows again, or a new class: stop and report with a probe. Then the fix in the tools `.nika`, rebuild to the fixpoint, commit every lowered file with the tests. Commit `Part of #1213`.
2. **(4)**: the lent `String` argument at `html::escape`, with a test; then drop the `ref` in `examples/escaping` and its tests pass. Commit `Part of #1213`.

`Rs2nika: none`.

## Done when
All probes check and run as stated without `ref`; the touched tests and `examples/escaping` pass; the fixpoint holds.

## Stop and report if
the keep change makes a std or tools signature refuse a caller the rule cannot tell apart (a real move through a match), or the first rustc cycle shows an error class this brief does not name: stop with `partial` and the probe.
**Note (supervisor, after the first v18a run):** this brief also takes the class v18a could not reach: `Some(ref x)` (a `ref` binding in the lowered pattern) against a subject that is already a borrow (`&Option[...]`, a lent parameter) is a rustc error. Cause: `check_core_flow.nika` `by_reference`. Build a probe of a few lines first (a lent `Option[String]` parameter matched with `Some(x) => x.len()`) and pin it as a test. A lent subject binds by view without `ref`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v18b-matched-lent-param.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq

**Note (supervisor, after v19):** when the rule for a `match` over a plain parameter lands, re-bless `tests/compiler/tests/migrate_views3_reads_package_tests_with_the_package.stdout`: its five `show(keep v: json::Value)` lines go.
