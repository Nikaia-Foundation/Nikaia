---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a list or struct of views handed back through an `if` or `match` is handed back (#1085)

Score: blocks #928 step 4 (`yaml`'s `events`, ADR-yaml-1 D7; y21-events run 3 blocked on it, draft in local branch `wip/y21-events` at 1c1199d6). A compiler defect found by the yaml lane; fixed in this lane because only `yaml` needs it now. It changes `crates/`: name in the report what it decided (the supervisor raises the version), `Changelog:` one sentence. The commit says `Fixes #1085`.

Facts: Where things live, Traps already met

## Problem

A value holding views of a `ref` parameter handed back through an `if` or `match` expression is refused `NK2302` ("`many` keeps `n` after this call, but `n: ref N` doesn't say whose data it points into"); handed back directly (`return [E { name: n.name }]`, `return E { .. }`) it passes since #1071 (79ee176c). It shows only where the struct's field is a view: below, `parse` builds `N` from a `trim()`. A struct `N` built in `main` from a literal does not show it (the field is then owned), which is why y21's own probe passed for the architect.

## Probe (`target/fast/nikaia run a.nika`; today NK2302; wanted `1 hello`; checked by the architect at 66f7750f)

```nika
struct N {
	name: String,
}

struct E {
	name: String,
}

fn many(n: ref N) -> Vec[E] sync {
	return if true { [E { name: n.name }] } else { [] }
}

fn all(ns: ref Vec[N]) -> Vec[E] sync {
	let mut out: Vec[E] = []
	for n in ns {
		out.extend(many(n))
	}
	return out
}

fn parse(input: String) -> Vec[N] {
	return [N { name: input.trim() }]
}

fn main() {
	let ns = parse(" hello ")
	let es = all(ns)
	println(f"{es.len()} {es[0].name}")
}
```

Variants, each wanted `1 hello` with the same `all`/`parse`/`main` (all refused today):

* `many`: `return match 1 {` / `1 => [E { name: n.name }],` / `else => [],` / `}`
* `many`: `let r: Vec[E] = if true { [E { name: n.name }] } else { [] }` then `return r`
* `many`: an arm that is a block, `return if true { let mut out: Vec[E] = [E { name: n.name }]` / `out } else { [] }` (yaml's `walk` arms are such blocks, with `for`/`extend`/`push` inside)
* `fn one(n: ref N) -> E sync { return if true { E { name: n.name } } else { E { name: n.name } } }`, with `all` doing `out.push(one(n))`

## Locations (high confidence)

* `crates/nikaia-std/src/tools/views.nika`, `handed_back` (~1076): a `StructLit` of the result and a `ListLit` (#1071) are handed back, everything else is `stores`. An `Expr::If`, `Expr::Match`, `Expr::Block` is to be followed to the value each branch or arm ends in (a block's tail, an arm's expression), each asked as `handed_back`; the condition and the matched value are asked as today (`descend`/`stores` on the parts that are not handed back). Look at how `rsrc_if`/`rsrc_match`/`rsrc_block` (~2185) find the tails: reuse their shape.
* The `let r = <if/match>; return r` shape: `local_vec_handed_back` / `struct_handed_back` (~880, ~983) look at the `let`'s value; they need the same descent.
* The block-arm shape: the `let mut out = [..]` inside an arm and its `out` tail is #1071's local-list case one level down; check that `local_vec_handed_back` sees the arm's block, not only the function's body.

## To build (after a `tools/*.nika` change, `lower-std` regenerates its `lowered/*.rs`; commit both)

1. `handed_back` follows `if`/`match`/block tails: the probe and its `match`, block-arm and struct-result variants print `1 hello`. Test `tests/compiler/tests/views_handed_back_through_if_and_match.nika` with `.stdout` (all four shapes in one program).
2. The `let r = <if/match>; return r` shape the same, in that test. A branch that stores the view elsewhere stays refused: `tests/compiler/tests/view_stored_in_a_branch_refused.nika` with `.stderr` (e.g. an arm that assigns `n.name` to a field of a `self` whose struct does not carry `n`'s buffer). `tests/compiler --both-settings` green; the #1071 tests (`rg -l 1071 tests/compiler/tests`) unchanged.
3. Proof on the real case: `git show 1c1199d6:packages/yaml/src/main.nika > /tmp/y.nika` (scratchpad path of your own), `target/fast/nikaia check` on it shows no error (the architect saw one NK2302 at `walk`'s `return match n.value`; with `walk` rewritten to push into a local the rest of the draft builds and its events test runs). Say in `Tests:` that it checks. Do not commit anything under `packages/yaml/`: y21 run 4 takes the draft.

Files you may change: the `tools/*.nika` under Locations with their `lowered/*.rs`, the new tests named under To build, and an existing test's expected output your change rightly changes (name it in `Body:`). If the fix needs a language decision, stop: `Result: blocked`, `Needs: architect decision`. Stop about 15 turns before `max_turns`, commit what is finished, report `partial`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y28-1085-views-back-through-if-match.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

