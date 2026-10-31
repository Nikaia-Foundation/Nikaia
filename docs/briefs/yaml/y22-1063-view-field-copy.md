---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: a view field of a lent struct is handed on as a view (#1063)

Score: blocks #928 step 4 (`yaml`'s `events` carries each node's anchor and tag, ADR-yaml-1 D7; `load` copies the tree with its strings still views, D6). A compiler defect found by the architect probing step 4; fixed in this lane because only the `yaml` package needs it now. It changes `crates/`: name in the report what it decided (the supervisor raises the version), `Changelog:` one sentence.

Facts: Where things live, Traps already met

## Problem

A struct whose `String` field is a view (built from a view of a parameter, ADR-353 D7, #1000) cannot hand that view on from a lent value: reading `w.text` of a `ref Word` (a `ref` parameter, a `for` element) into a new struct is refused with `NK2106` ("`w.text` is stored in `Word.text`, but `w` is only borrowed here"). A view is not a part taken out: copying it takes nothing from `w` (in Rust `&'a str` is `Copy`; `fn again<'a>(w: &Word<'a>) -> Word<'a>`). Part I 6.5 refuses a *part* of a loan handed over; a view field is already a view, and "an argument that is already a view" is passed as it is.

In the `yaml` package the refusal also cascades: once a function builds a `Node` from a `ref Node`'s `anchor`/`tag`, every `Node { .., anchor: p.anchor, tag: p.tag, .. }` in the grammar is refused `NK1106` (the inference decides the fields need text of their own) and `parse`'s `limits: Limits = default_limits()` gets `NK1127`.

## Probes (checked by the architect on today's `target/fast/nikaia`)

Probe A (`nikaia run a.nika`; today `NK2106` on `w.text`; wanted: prints `1 hello`):

```nika
struct Word { start: i64, text: String }

fn word_of(input: String) -> Word {
	return Word { start: 0, text: input.trim() }
}

fn again(w: ref Word) -> Word {
	return Word { start: w.start + 1, text: w.text }
}

fn main() {
	let s = " hello "
	let w = word_of(s)
	let v = again(w)
	print(f"{v.start} {v.text}")
}
```

Probe B (today `NK2106` on `w.text` and on `w.tag`; wanted: prints `1 hello`):

```nika
struct Word { start: i64, text: String, tag: String? }

fn word_of(input: String) -> Word {
	return Word { start: 0, text: input.trim(), tag: null }
}

fn again_each(ws: ref Vec[Word]) -> Vec[Word] {
	let mut out: Vec[Word] = []
	for w in ws {
		out.push(Word { start: w.start + 1, text: w.text, tag: w.tag })
	}
	return out
}

fn main() {
	let s = " hello "
	let ws = [word_of(s), word_of(s)]
	let vs = again_each(ws)
	print(f"{vs[1].start} {vs[1].text}")
}
```

Probe C, the package shape: copy `packages/yaml` (`src/`, `nikaia.toml`) to a scratch directory, append to `src/main.nika`

```nika
fn copy_node(n: ref Node, mut count: Count, limit: i64) -> Node sync throws {
	count.n += 1
	let v = match n.value {
		Value::Seq(items) => {
			let mut out: Vec[Node] = []
			for x in items {
				out.push(copy_node(x, count, limit))
			}
			Value::Seq(out)
		}
		other => other.clone()
	}
	return Node { start: n.start, end: n.end, anchor: n.anchor, tag: n.tag, value: v }
}

pub fn load(input: ref Bytes) -> Vec[Node] sync throws {
	let docs = parse(input)
	let mut out: Vec[Node] = []
	for d in docs {
		let mut count = Count { n: 0 }
		out.push(copy_node(d, count, 100))
	}
	return out
}

struct Count { n: i64 }
```

and a `tests/load.nika` that loads `Bytes("- 1\n- 2\n")` and prints `{docs.len()} {docs[0].end}` (`.stdout`: `1 7`). Today: 14 `NK1106` in the grammar, two `NK2106`, one `NK1127`; wanted: the test passes. Do not commit this probe into the package (`load` is a later brief).

## Locations (suspected by the architect, medium confidence)

* `crates/nikaia-std/src/tools/check_takes.nika`, `hands_over_path_of` (line ~353): `path.contains(".") && self.lent_here(root)` refuses through `a_part_of_a_loan_handed_over` (`check_values.nika:240`) whenever `takes_away(world, ty)` (`check_values.nika:12`) says the field's type moves; it does not ask whether the field is a view field of a view-holding struct. Find the fact that says so (the `Ty::Named { view }` flag, or `views.nika`'s view-struct facts by `node_key` through `check_facts.nika`) or add the field that carries it.
* `crates/nikaia-std/src/tools/views.nika`: `view_fields` (~486), `borrowing_types` / `package_borrowing_types`, which decide a struct holds views; Probe C's `NK1106` cascade comes from there deciding the fields owned once such a copy exists.
* Lowering: any lowering rustc accepts (the copy is a copy of the reference; the function's result gets the struct's lifetime, `&Word<'a> -> Word<'a>`).
* After changing a `tools/*.nika`, `lower-std` (builder-facts) regenerates `crates/nikaia-std/src/tools/lowered/<file>.rs`; commit both.

## To build

1. A view field read from a lent struct (a `ref` parameter, a `for` element) and stored in a struct, pushed or returned is a view of the same buffer, not a part taken out: Probes A and B print `1 hello`, Probe C passes. Tests in `tests/compiler/tests/`: `view_field_of_a_lent_struct_handed_on.nika` (Probe A) and `view_fields_of_for_elements_handed_on.nika` (Probe B), each with `.stdout`; if Probe C needs more than A and B fixed, a test of its shape in a package read by another (`use compiler_tests`, builder-facts). Commit.
2. An owned `String` field of a lent struct stays refused with `NK2106`: `let_lent_field_kept` and `let_moved_ref_parameter` keep their `.stderr` byte for byte; `tests/compiler --both-settings` green.

If the fix needs a language decision (the rule in Part I 6.5 does not settle it), stop: `Result: blocked`, `Needs: architect decision`. Stop about 15 turns before `max_turns`, commit what is finished, report `partial`.

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
* Files not to touch: anything outside `crates/nikaia-std/src/tools/` (the `.nika` files named under *Locations*, the checker files they lead to, and their `lowered/*.rs`), `crates/nikaia/src/` where the lowering needs it, `tests/compiler/tests/` and the brief's own report; never `docs/briefs/batch.txt` or a brief outside `docs/briefs/yaml/`, nor another agent's uncommitted file.

* **Stop about 15 turns before `max_turns`**: commit and push every finished step, then report `Result: partial` with what is left under `Needs:`. Uncommitted work at the end is lost work (y4 ran out at 110).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y22-1063-view-field-copy.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

