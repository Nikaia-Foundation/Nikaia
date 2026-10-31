---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief: a by-value parameter whose field a `match` moves out is consumed, not lent (#1092)

Score: y29's `Found:` (the fix for #1089), filed as #1092. Nothing in `yaml` needs it yet; built in this lane because the lane found it and the view-checker facts are fresh from y29. It changes `crates/`: name in the report what it decided (the supervisor raises the version), `Changelog:` one sentence. The commit says `Fixes #1092`.

Facts: Where things live, Traps already met

**Problem.** A by-value parameter nobody consumes is inferred lent (ADR-277 D17) and lowered `a: &A`. A `match` on one of its non-copy optional fields whose arm hands the payload on by value (`x => x`) moves the field out, so it consumes the parameter, as `a.node ?? throw ..` already does; but the inference does not count it, and rustc refuses the move (E0507, *Cannot move out of `a.node` ... behind a shared reference*). Checked by the architect on `target/fast/nikaia` at 445f6064.

**Probe** (`nikaia run p.nika`; wanted `3`):
```nika
struct E { msg: String }
impl Error for E { fn message(ref self) -> String { return self.msg.clone() } }
struct N { v: Vec[i64] }
struct A { node: N? }
fn owned(a: A) -> N sync throws {
    let node = match a.node { null => throw E { msg: "no node" }  x => x }
    return node
}
fn main() throws {
    let n = owned(A { node: N { v: [1, 2, 3] } })
    print(f"{n.v.len()}")
}
```
Rows: a the probe (E0507); b `return a.node ?? throw E{..}` prints `3` (runs, must stay); c the arm `x => x.clone()` (must stay; the parameter may stay lent); d an enum payload, `struct B { v: V }` `enum V { Seq(Vec[i64])  Nothing }`, `fn f(b: B) -> Vec[i64] { return match b.v { V::Seq(items) => items  else => [] } }` (probe it first; same cause expected); e the match on the parameter itself, `fn g(o: N?) -> N sync throws { return match o { null => throw ..  x => x } }` (probe; must print).

**Suspected location (low confidence).** Find where the inference of ADR-277 D17 records a consume for `??` on a parameter's field (`rg -n "lent" crates/nikaia-std/src/tools/check_flow.nika crates/nikaia-std/src/tools/check_references.nika crates/nikaia-std/src/tools/check_boundary.nika`) and make a `match` scrutinee rooted at the parameter count the same way when an arm binds a non-copy payload and that binding is moved (returned, stored, passed by value). y29 (445f6064) touched the neighbouring `check_core_flow_through_a_view` / `got_past_a_jump`: read its diff first, do not undo it.

## To build (after a `tools/*.nika` change, `lower-std` regenerates its `lowered/*.rs`; commit both)

1. Rows a, d, e print; b, c unchanged. Test `tests/compiler/tests/match_moves_a_field_of_an_owned_parameter.nika` + `.stdout` (all rows in one program, each printing), refused by the unfixed binary (say so in `Tests:`). #1057 (`return s.inner` of an element read) is a neighbour: do not build it.
2. `tests/compiler --both-settings` green; the view tests of the gate (`it text_views stored_views tethers views_kept`) green; `tests/compiler/tests/match_on_a_field_of_a_get_bound_local` (y29) still green.

Files you may change: the `tools/*.nika` the cause is in, with their `lowered/*.rs`, the new test, and an existing test's expected output your change rightly changes (name it in `Body:`). If the fix needs a language decision, stop: `Result: blocked`, `Needs: architect decision`. A builder that has found no cause after about 40 turns stops and reports `partial` with what it ruled out.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step. Do not push: the driver runs the suite on your commit and pushes it (CLAUDE.md, *Builders commit, the driver tests and pushes*).
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit before you write the report; the report's `Pushed:` line is `local <sha>` of your last commit. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).
* **Package gate** (this lane): `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project packages/yaml` green, and `cargo test --profile fast -p nikaia --test it -- fmt::every_nika_file` (it formats every `.nika` under `packages/`). For a change only under `packages/yaml/` this is the gate the line above asks for; a change under `crates/` runs the gate of `.claude/agents/builder.md` (#852) as well. `target/fast/nikaia` is built by the session hook; if it is missing, `cargo build --profile fast -p nikaia` once.
* A package change raises no toolchain version (CLAUDE.md table; ADR-yaml-1 D1): `Changelog: none`, unless the brief changes `crates/`.
* Files not to touch: anything outside `packages/yaml/` and the brief's own report, unless the brief names it; never `docs/briefs/batch.txt` or a brief of another lane.

* **Stop about 15 turns before `max_turns`**: commit every finished step, then report `Result: partial` with what is left under `Needs:`. Uncommitted work at the end is lost work (y4 ran out at 110).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y30-1092-match-moves-owned-field.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

