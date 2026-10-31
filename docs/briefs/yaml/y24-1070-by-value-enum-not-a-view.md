---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a by-value parameter that holds no text is no view source (#1070)

Score: blocks #928 step 4 (`yaml`'s `events`, ADR-yaml-1 D7; y21-events run 2 blocked on it, draft in local branch `wip/y21-events`). A compiler defect found by the yaml lane; fixed in this lane because only `yaml` needs it now. It changes `crates/`: name in the report what it decided (the supervisor raises the version), `Changelog:` one sentence. The commit says `Fixes #1070`.

Facts: Where things live, Traps already met

## Problem

A by-value parameter whose type holds no text and no view (a field-less `enum`) is marked as a view source (`!kind`) by the result-source walk, so a function that builds a view-holding struct from it is refused `NK2302` "hands back a view of `kind`, which it takes by value". An `i64` parameter in the same place passes (`rsrc_scalar`). Found by the `yaml` lane (#928 step 4, `events`, run y21-events).

## Probe (`target/fast/nikaia run a.nika`; today NK2302 on both returns; wanted `hello`)

```nika
enum K {
	A,
	B,
}

struct N {
	name: String,
}

struct E {
	kind: K,
	name: String,
}

fn bare(kind: K) -> E sync {
	return E { kind: kind, name: "" }
}

fn opening(kind: K, n: ref N) -> E sync {
	return E { kind: kind, name: n.name }
}

fn word_of(input: String) -> N {
	return N { name: input.trim() }
}

fn main() {
	let n = word_of(" hello ")
	let e = opening(K::A, n)
	let b = bare(K::B)
	println(f"{e.name}{b.name}")
}
```

## Locations (suspected, high confidence)

* `crates/nikaia-std/src/tools/views.nika`, `rsrc_function` (~2913): `if arg.ty.code == null && !arg.mutable && !sources.contains(owned) && !rsrc_scalar(asked, arg.ty)` marks every by-value non-scalar parameter `!`. `rsrc_scalar` (~1978) knows only the number types, `bool`, `char`.
* What says a type holds no text: the type's fields/variants (`unit.fields`, the declared variants), `rsrc_ty_holds_view` (~1989) for ledger types. The test is "nothing can be read out of it that points into it": no `String`, no view, no list, recursively through its own fields; a field-less enum is the first case.

## To build (after a `tools/*.nika` change, `lower-std` regenerates its `lowered/*.rs`; commit both)

1. A by-value parameter of a type that holds no text and no view (a field-less enum; a struct or enum of only such fields and scalars) is not a view source: the probe prints `hello`. Test `tests/compiler/tests/by_value_enum_beside_a_view.nika` + `.stdout`.
2. A by-value `String` or a struct with a `String` field stays refused `NK2302` where a view of it is handed back: the existing tests with that message keep their `.stderr` byte for byte; `tests/compiler --both-settings` green.

Probes checked by the architect on `target/fast/nikaia` at 11f808ef. Files you may change: the `tools/*.nika` under Locations with their `lowered/*.rs`, the new tests named under To build, and an existing test's expected output your change rightly changes (name it in `Body:`). If the fix needs a language decision, stop: `Result: blocked`, `Needs: architect decision`. Stop about 15 turns before `max_turns`, commit what is finished, report `partial`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y24-1070-by-value-enum-not-a-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

