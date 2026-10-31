---
agent: builder
model: opus
max_turns: 90
budget_usd: 22
---
# Brief: #1037, a dependency's struct with two view fields is named with one lifetime (ADR-353 D7)

Score: blocks #928 (yaml properties, `y2c-properties.md` run 2 is next in this batch); a defect of something specified (ADR-353 D7), so a compiler fix, not a package change.

## Issue (#1037, copied)

A `struct` of another package with two or more fields that hold a view has one lifetime per field (ADR-353 D7: "the ledger's `tethered` lists these fields in that order, and a package that reads the type takes its places from that list"). Named as a type in the reading package, e.g. a parameter `n: ref two::Node`, it is lowered with **one** lifetime argument, and rustc refuses it. The same program in one file runs. Not naming the type (a `for n in two::make(..)` with a `match` inline) runs.

Probe (package `two`, `nikaia.toml` `[package] name = "two"`, `src/main.nika`):

```
pub struct Node {
	pub a: String?,
	pub b: String?,
	pub value: Value,
}

pub enum Value {
	Bool(bool),
	Str(String),
}

pub fn make(s: String) -> Vec[Node] {
	return [Node { a: s.strip_prefix("l"), b: null, value: Value::Bool(true) }, Node { a: null, b: s.trim(), value: Value::Str(s.trim()) }]
}

fn main() {
}
```

`tests/named.nika`, expected `bool(true)` then `str(left)`:

```
use two

fn show(n: ref two::Node) -> String {
	return match n.value {
		two::Value::Bool(b) => f"bool({b})"
		two::Value::Str(s) => f"str({s})"
	}
}

fn main() {
	for n in two::make(" left ") {
		println(show(n))
	}
}
```

The architect ran it (scratch package, `nikaia test --project .`, today's develop): rustc `Struct takes 3 lifetime arguments but 1 lifetime argument was supplied` (3:1), then `Type bool cannot be dereferenced` and twice `The size for values of type str cannot be known` (4:2). Traced: only the first. Whether the other three follow from it is **unverified**; done means the program prints both lines. The package ledger is right: `[type."Node"] tethered = ["a", "b", "value"]`. The yaml draft (`origin/wip/y2-properties`) fails 9 of 10 package tests with exactly this set of errors.

## Locations

* `crates/nikaia-std/src/tools/emit_core_types.nika:329-331`: a borrowing or tethered type gets `self.lifetime_args(name, lifetimes.params())`.
* `crates/nikaia-std/src/tools/emit_sums.nika:80` `lifetime_args`: `self.view_slots.get(name) ?? return lifetime.clone()` (one lifetime on a miss).
* `crates/nikaia-std/src/tools/emit_tables.nika:299` `emit_tables_read_view_slots(ledger)`: another package's places, keyed by the ledger's bare type name (`Node`); `crates/nikaia/src/emit/mod.rs:857-864` chains it into `EmitState.view_slots` from `described`.
* `emit_function.nika:823` and `:1062` (`emit_split_lifetimes`) read `view_slots` by `emit_text(world, t.name)` too; `emit_item.nika:121` `emit_item_slots`.
* Suspects, unverified: the key (the written `two::Node` against `Node` in the map), or the dependency's ledger not reaching `described` on this path. Look first (`docs/builder-facts.md`, *A wrong lowering: look before you read code*): lower the probe test and read the `fn show` line and the `Node` struct.
* Each `.nika` under `crates/nikaia-std/src/tools/` has its lowered twin in `tools/lowered/`: `nikaia lower-std` (see `docs/builder-facts.md`, *Commands*), and commit both.

Read `docs/builder-facts.md` (its first trap: check the winnow-grammar rev of `target/fast/nikaia` before a probe).

## To build (one commit)

1. The reading package takes a dependency's places from its ledger (ADR-353 D7) wherever the type is written: a parameter, `ref`, a result, a field of a local struct. Test: the compiler's test project is itself a package (`tests/compiler/nikaia.toml`, name `compiler_tests`; `src/main.nika` holds an empty `main` today) and its tests reach its `pub` items with `use compiler_tests`, as `packages/yaml/tests` do with `use yaml` (the architect ran the probe in that layout, with `fn main` in the package: same failure). So put the probe's `Node`, `Value`, `make` as `pub struct Places`, `pub enum PlacesValue`, `pub fn places` into `tests/compiler/src/main.nika` (keep its `main`), and `tests/compiler/tests/a_dependencys_type_with_two_view_fields_keeps_its_places.nika` + `.stdout` with the probe's `show`/`main` (`compiler_tests::Places`); it fails on the unfixed binary (say so under `Tests:`).
2. `packages/yaml` gate green (its tests name `ref yaml::Node`; `Node` has one place today).

Not here: #1038 (a dependency's view field returned beside an owned text, NK1104): a separate defect in the checker or the tier pass; if your probe meets it, stop and name it, do not fix it in this commit.

## Done when

The new test passes; `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project tests/compiler > out.txt 2>&1` green (both settings as the gate asks); the package gate below green; committed and pushed with `Fixes #1037`. Files you may change: the emitter files above and their lowered twins, `crates/nikaia/src/emit/mod.rs`, `crates/nikaia-std/src/tools/emit_tables.nika` and twin, `tests/compiler/src/main.nika`, the new test. Stop and report if the fix needs a checker or ledger change (`ledger` writing, `check_*.nika`): name the fact and the field (`docs/architect.md`, *An emitter fix that needs what the checker knows*).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).
* **Package gate** (this lane): `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project packages/yaml` green, and `cargo test --profile fast -p nikaia --test it -- fmt::every_nika_file` (it formats every `.nika` under `packages/`). For a change only under `packages/yaml/` this is the gate the line above asks for; a change under `crates/` runs the gate of `.claude/agents/builder.md` (#852) as well. `target/fast/nikaia` is built by the session hook; if it is missing, `cargo build --profile fast -p nikaia` once.
* A package change raises no toolchain version (CLAUDE.md table; ADR-yaml-1 D1): `Changelog: none`, unless the brief changes `crates/`.
* Files not to touch: anything outside `packages/yaml/` and the brief's own report, unless the brief names it; never `docs/briefs/batch.txt` or a brief of another lane.

* **Stop about 15 turns before `max_turns`**: commit and push every finished step, then report `Result: partial` with what is left under `Needs:`. Uncommitted work at the end is lost work (y4 ran out at 110).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y13-1037-dependency-places.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
