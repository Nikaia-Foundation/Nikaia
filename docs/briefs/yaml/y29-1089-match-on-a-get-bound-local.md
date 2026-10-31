---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief: a local bound by `m.get(k) ?? <jump>` or `match m.get(k) { null => <jump>  x => x }` is lent; a match on its field borrows (#1089)

Score: blocks #928 step 4's second half (`yaml`'s `load`, ADR-yaml-1 D6, D8; y27-load run 1 blocked on it, draft on local branch `wip/y27-load` at eef3a2e7). A compiler defect found by the yaml lane, fixed in this lane because only `yaml` needs it now. It changes `crates/`: name in the report what it decided (the supervisor raises the version), `Changelog:` one sentence. The commit says `Fixes #1089`.

Facts: Where things live, Traps already met

**Problem.** A local bound to an element through a read whose miss jumps - `let a = m.get(k) ?? throw ..` / `?? return ..`, or `let a = match m.get(k) { null => throw ..  x => x }` - is not taken as lent. A `match` or `??` on one of its non-copy optional fields then lowers without `&` (`match a.node { Some(x) => x }`), and rustc refuses it: *Cannot move out of `a.node` as enum variant `Some` which is behind a shared reference*. The same match on a `ref A` parameter, or on `let a = m[k]`, lowers `match &a.node` and runs. Found by the yaml lane (y27-load, `load`'s alias resolution reads an anchor table this way); probed by the architect on `target/fast/nikaia` at 6eab43c4.

**Probe** (`nikaia run p.nika`; wanted `2`):
```nika
use std::collections
struct E { msg: String }
impl Error for E { fn message(ref self) -> String { return self.msg.clone() } }
struct N { v: Vec[i64] }
struct A { node: N? }
fn get(m: ref collections::HashMap[String, A], k: String) -> N sync throws {
    let a = match m.get(k) { null => throw E { msg: "no key" }  x => x }
    let node = match a.node { null => throw E { msg: "no node" }  x => x }
    return node.clone()
}
fn main() throws {
    let mut m: collections::HashMap[String, A] = collections::HashMap()
    m.insert("a", A { node: N { v: [1, 2] } })
    let n = get(m, "a")
    print(f"{n.v.len()}")
}
```
Rows (all checked; the body of `get` changed):
* a: the probe (map, `match .. null => throw`) - rustc E0507.
* b: `let a = m.get(k) ?? throw E{..}` - rustc E0507.
* c: `let a = m.get(k) ?? return N { v: [] }` - rustc E0507.
* d: over `m: ref Vec[A]`, `let a = m.get(k) ?? throw E{..}` and the `match` form - rustc E0507.
* e: row b with `let node = a.node ?? throw E{..}` (a `??` on the field, not a `match`) - rustc E0507, lowered `match a.node { Some(__nikaia_value) => .. }`.
* f (runs, must stay): `let a = m[k]` over `ref Vec[A]`, with the field `match` or `??` - prints `2` (lowered `match &a.node`).
* g (runs, must stay): the field match in a function taking `a: ref A` - prints `2`.
* h (runs, must stay): row a with arm `x => x.clone()` - prints `2`.

**Suspected location (medium confidence).** `crates/nikaia-std/src/tools/check_core_flow.nika`, `check_core_flow_through_a_view` (:482) decides `lent_scrutinees` (the `&` the emitter writes, `emit_core_match.nika:1096`) from `local.lent || local.ty.is_a_view() || state.a_lent_let(world, root)`. For rows a-e none holds: the `let` in `crates/nikaia-std/src/tools/check_core_stmt.nika` (the let walk around :700-750) marks `check_core_stmt_lent_let` for a place (`m[k]`, row f), a seen-through value and `check_core_stmt_a_lent_list_read` (:87, a `Coalesce` whose read is in `flow.lent_list_reads` and whose fallback jumps) - which misses the map `get`, the `ref Vec` parameter's `get` (row d) and the `match .. { null => <jump>  x => x }` form entirely. The `??` on the field (row e) goes through the coalesce lowering, which reads the same facts or its own: find which.

## To build (after a `tools/*.nika` change, `lower-std` regenerates its `lowered/*.rs`; commit both)

1. Rows a-e lower with `&` (or any lowering rustc accepts) and print `2`; rows f-h unchanged. A local so bound is a view of the collection for the rest of the checker too (it may not be handed away or kept past the collection). #1057 is the neighbouring refusal of `return a.field` (bug lane, b13): do not build it, do not change its rows. Test `tests/compiler/tests/match_on_a_field_of_a_get_bound_local.nika` + `.stdout` (rows a-e and f-h in one program, each printing), refused by the unfixed binary (say so in `Tests:`).
2. `tests/compiler --both-settings` green; the view tests of the gate (`it text_views stored_views tethers views_kept`) green.
3. Proof on the real case: `git show eef3a2e7:packages/yaml/src/main.nika > <your scratchpad>/y.nika`; `target/fast/nikaia check` on it is clean, and `nikaia run` of a scratch copy calling `load` once (defaults written out) builds (y27's `Loader::expansion` has `let node = match a.node { .. }`). Say in `Tests:` that it builds. Commit nothing under `packages/yaml/`: y27 run 2 takes the draft.

Files you may change: the `tools/*.nika` under the suspected location (and the coalesce lowering's file if row e needs it) with their `lowered/*.rs`, the new test, and an existing test's expected output your change rightly changes (name it in `Body:`). If the fix needs a language decision, stop: `Result: blocked`, `Needs: architect decision`. A builder that has found no cause after about 40 turns stops and reports `partial` with what it ruled out.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y29-1089-match-on-a-get-bound-local.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
