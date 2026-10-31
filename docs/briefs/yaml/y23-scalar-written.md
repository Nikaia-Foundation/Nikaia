---
agent: builder
model: opus
max_turns: 70
budget_usd: 12
---
# Brief: `yaml`, a scalar node keeps its text and style as written (#928 step 4, ADR-yaml-1 D7)

Score: #928 step 4; `y21-events.md` (run 2, next in this batch) needs it for the suite's `=VAL` lines. Touches only `packages/yaml`: no version raise, `Changelog: none` (ADR-yaml-1 D1).

Read ADR-yaml-1 D4, D6 and D7 **with its last paragraph** (architect, 2026-10-31: a scalar keeps how it was written), and in `packages/yaml/src/main.nika` the tree types at the top and every rule that builds a `Node` (`rg -n "Node \{" packages/yaml/src/main.nika`: about 30 places; the scalar ones are `Double`, `Single`, the plain rules, `BlockScalar`, and the empty node after properties).

## The problem (probed by the architect on 2a303980)

The tree drops a scalar's text and style when it resolves to `Null`, `Bool`, `Int` or `Float`. This test, run in a copy of the package, prints `Null Null Null Bool(true) Bool(true) Int(1) at 28`: `~`, `null` and `NULL` cannot be told apart, nor `true` and `True`, and `!!int "1"` lost its quotes.

```nika
use yaml

fn main() {
	let input = Bytes("[~, null, NULL, true, True, !!int \"1\"]".bytes().collect())
	let docs = yaml::parse(input) catch {
		println("error")
		return
	}
	for n in docs {
	match n.value {
		yaml::Value::Seq(items) => {
			for i in items {
				let s = match i.value {
					yaml::Value::Null => "Null"
					yaml::Value::Bool(b) => f"Bool({b})"
					yaml::Value::Int(t) => f"Int({t}) at {i.start}"
					else => "other"
				}
				println(s)
			}
		}
		else => println("not a seq")
	}
	}
}
```

## To build

1. `Node` gets `pub raw: String` and `pub style: Style` (D7's last paragraph), with a comment saying what they hold. For every scalar node, whatever its `Value`: `raw` is the text as that rule's `Scalar.raw` takes it (a view, between the quotes, after a block header; for a double-quoted scalar with escapes the raw view `s`, not the decoded parts), `style` the style it was written in. For a collection, an alias and an empty node (also after properties, `!!null` alone): `raw: ""` and `Style::Plain`. A node that takes its properties copies `raw` and `style` from its content, as it copies `flow`. Nothing is copied: views only (D4). A `Value::Str`'s `Scalar` stays as it is.
2. Test `packages/yaml/tests/scalar_written.nika` + `.stdout`: for each of `~`, `null`, `NULL`, empty (`[a: ]` value or `- ` item), `true`, `True`, `FALSE`, `0x1F`, `.inf`, `"x"`, `'y'`, `!!int "1"`, `!!null ''`, a literal and a folded block scalar, an anchored `&a 7`, and a sequence and a mapping: print the value's kind, `raw` between brackets and `style`. Write the `.nika` before the first package run and fill `.stdout` with `--bless` in that run; check every line by hand against step 1. The 15 existing package tests keep their output byte for byte.

Stop about 15 turns before `max_turns`: commit what is finished, report `partial`.

If a form the package needs is missing or refused in the compiler, the lowering or `std` (no ADR refusing it), stop: `Result: blocked`, `Found:` the gap with a probe of a few lines, `Needs: architect decision`. Do not write around it.

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

* **Stop about 15 turns before `max_turns`**: commit and push every finished step, then report `Result: partial` with what is left under `Needs:`. Uncommitted work at the end is lost work (y4 ran out at 110).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y23-scalar-written.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

