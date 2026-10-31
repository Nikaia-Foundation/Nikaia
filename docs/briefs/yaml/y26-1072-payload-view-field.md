---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a view field read off a match payload of a lent struct is a view (#1072)

Score: blocks #928 step 4 (`yaml`'s `events`, ADR-yaml-1 D7; y21-events run 2 blocked on it, draft in local branch `wip/y21-events`). A compiler defect found by the yaml lane; fixed in this lane because only `yaml` needs it now. It changes `crates/`: name in the report what it decided (the supervisor raises the version), `Changelog:` one sentence. The commit says `Fixes #1072`.

Facts: Where things live, Traps already met

## Problem

A view field read off a `match` payload binding of a lent struct, `match n.value { V::One(m) => out.push(E { name: m.name }) }` with `n: ref N`, is refused `NK1106` ("`E.name` holds `String`, but you're giving it `ref String`"): the text tiers decide `E.name` owned. The same read off the `ref` parameter itself (`n.name`) or off a `for` element of a `ref Vec` (#1063, ac156203) passes. Also through a `for` over a payload list (`V::Seq(items) => for x in items { .. x.name .. }`). Found by the `yaml` lane (#928 step 4, `events`, run y21-events): the walk matches `node.value` for `Seq`/`Map` children.

## Probe (`target/fast/nikaia run a.nika`; today NK1106; wanted `1 hello`)

```nika
enum V {
	One(M),
	Other,
}

struct M {
	name: String,
}

struct N {
	value: V,
}

struct E {
	name: String,
}

fn walk(n: ref N) -> Vec[E] sync {
	let mut out: Vec[E] = []
	match n.value {
		V::One(m) => out.push(E { name: m.name }),
		else => {}
	}
	return out
}

fn word_of(input: String) -> N {
	return N { value: V::One(M { name: input.trim() }) }
}

fn main() {
	let n = word_of(" hello ")
	let es = walk(n)
	println(f"{es.len()} {es[0].name}")
}
```

## Locations (suspected, medium confidence)

* `crates/nikaia-std/src/tools/text_tiers.nika`, `bind_static_payloads` / `bind_a_payload` (~1440): a payload binding is bound only for text or a list of text, so `m` of struct type `M` has no source and `source_of(m.name)` (~2086) finds nothing; the read then counts as owned (`tier_at`'s default), as #1063's field reads did before ac156203.
* The `for x in items` shape goes through `element_word` (~1292) / the `For` binding with `items` a payload binding.
* The checker side (`ref String` for `m.name`) is right; only the tiers are wrong.

## To build (after a `tools/*.nika` change, `lower-std` regenerates its `lowered/*.rs`; commit both)

1. A field read off a struct a `match` payload binds (tuple and named variant patterns), and off an element of a payload list, is that field's text: the probe prints `1 hello`, and its `for x in items` shape (payload `Seq(Vec[M])`) prints the same. Tests `tests/compiler/tests/view_field_of_a_payload_handed_on.nika` and `view_fields_of_payload_elements_handed_on.nika`, each with `.stdout`.
2. An owned field read off a payload stays owned where it goes into owned text; `tests/compiler --both-settings` green, `view_field_of_a_lent_struct_handed_on` and `view_fields_of_for_elements_handed_on` unchanged.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y26-1072-payload-view-field.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

