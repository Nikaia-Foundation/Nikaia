---
agent: builder
model: opus
max_turns: 100
budget_usd: 25
---
# Brief: #905, a variant's `Vec[String]` part taken in a `match` arm, and no unused `let mut` for a drained part

Score: bugs lane (`docs/priorities.md`): a user-visible checker defect (correct programs refused, one passes to a rustc error) and a workaround in our own `node_numbers.nika` (owner: no technical debt). Found by n903b on 2026-10-28.

## Issue #905 text, copied (builders have no GitHub)

Found by n903b (`tools/node_numbers.nika`, 2026-10-28), widened by the architect's probes on 42c0246.

### Problem
Part I 3.4 (*A `match` reads what it binds*): over a name the function owns, an arm that keeps the part (returns it, hands it on, changes it) takes it. For a variant whose part is `Vec[String]` the checker types the binding `Vec[ref String]` in every such arm, so taking it is refused:

| Probe (`enum E { A(Vec[String]), N }`, `fn g(e: E)`) | Today |
| :--- | :--- |
| `match e { E::A(xs) => { return xs }, else => { return [] } }` (also with `E::N =>` instead of `else`) | NK1104 `Vec[ref String]` returned |
| `E::A(xs) => { return h(xs) }`, `fn h(xs: Vec[String])` | NK1102 `Vec[ref String]` passed |
| `E::A(xs) => { ...; for x in xs { out.push(x) } ... }` and `for x in xs.drain()` | NK1102 `push` gets `ref String` |
| `let r = match e { E::A(xs) => xs, else => [] }; return r` | passes the checker, rustc: Mismatched types |
| same with `A(String)` or `A(Vec[i64])`, `return s` / `return xs` | runs |
| `struct S { xs: Vec[String] }`, `for x in s.xs { out.push(x) }` | runs |

In `tools/*.nika` (lowering) the drained form passes, but `check_core_flow_changed_copies` records the drain as a change (`changed_bindings`), `emit_core_match.nika` :530 writes `let mut xs = xs;` while the loop lowers to `into_iter()`: rustc `unused_mut`, clippy `-D warnings` red. `node_numbers.nika` routes around it (loops in functions taking the list); that workaround goes once this is fixed.

### Locations
* `crates/nikaia-std/src/tools/check_core_flow.nika`: `check_core_flow_arm_pattern_parts` :944, `check_core_flow_parts` :964 (`view_of_type` on parts when `!lendable && typed.is_a_view()`), `check_core_flow_changed_copies` :1104, `check_core_flow_lent_parts` :1060.
* `crates/nikaia-std/src/tools/check_patterns.nika` :14 `pattern_parts`; `check_types.nika` :39 `view_of_type`.
* `crates/nikaia-std/src/tools/emit_core_match.nika` :530 (`changed_bindings` -> `let mut`).
* `crates/nikaia-std/src/tools/node_numbers.nika`: the helper functions that take the list only because of this.

### To build
1. Find where the part's type becomes `Vec[ref String]` (a view of the element instead of the part taken whole); the arm that keeps the part binds `Vec[String]`. Tests in `tests/compiler/` (Nikaia): the six rows above, each with `.stdout`; plus one row where the arm only reads `xs` and `e` is used after the `match` (stays a view, still runs).
2. A drained (or consumed by `for`) part is not a change that needs `let mut`: no `let mut x = x;` where the body never needs `mut`. Test: the drained row, lowered in a tools-like program, clippy clean (`lower-std` shows no unused_mut). Then fold `node_numbers.nika`'s loops back into the arms; `lower-std` changes only the files this touches.

## Gate
Build, clippy, `scripts/pre-push-check.sh`; `nikaia test --both-settings --project tests/compiler`; the `it` binary's `errors_lowering::`, `samples::`, `diagnostics::`, `lower::` (if present); `lower-std` changes only the files this touches (name them in `Body:`). Not the full suite. The last commit says `Fixes #905`.

## Stop and report if
The binding type comes from a rule Part I states otherwise (quote it under `Owner:`); more than 20 existing tests change output; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b905-variant-list-part.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
