---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief: `yaml` events, a walk of the parsed tree in the yaml-test-suite's form (#928 step 4, first half)

Score: #928 step 4, the next step of the package; #1000 (which step 4 needed) is built (d715e425). This change touches only `packages/yaml` and needs no version raise and no `CHANGELOG.md` entry (ADR-yaml-1 D1).

Read `docs/builder-facts.md`, ADR-yaml-1 D4, D6 and **D7 with its new paragraph** (architect, 2026-10-31), and `packages/yaml/src/main.nika` (the tree types at the top, `rule stream`/`Docs`/`NextDoc`/`AnyDoc`, `parse`). The yaml-test-suite's event form: https://github.com/yaml/yaml-test-suite (`test.event` files), summarised below. You do not need the network for it.

## Pre-flight (added 2026-10-31, before the first run)

The architect's probe of this step found #1063: a view field of a lent struct (`n.anchor` of a `ref Node`) stored in a new struct is refused `NK2106`, and in the package it cascades into `NK1106` across the grammar. `y22-1063-view-field-copy.md` fixes it and runs before this brief. First run its Probe A (`target/fast/nikaia run` on the `Word`/`again` program there): it must print `1 hello`. If it does not, stop: `Result: blocked`, `Found: #1063 not fixed`. An event's anchor, tag and value are views of the input copied from the node (ADR-yaml-1 D4: nothing is copied), not `.clone()`d text.

## To build

1. ~~**The tree keeps what the walk needs**~~ (built in run 1, 2a303980; keep it as it is) (D7, new paragraph). `Node` gets `pub flow: bool` (true for a flow sequence or mapping, false for everything else); every `Node { .. }` in the grammar sets it. A `pub struct Document { pub root: Node, pub start: bool, pub end: bool }` (`---` written, `...` written after it), and `pub fn documents(input: ref Bytes; limits: Limits = default_limits()) -> Vec[Document] sync throws`, with the size and depth errors `parse` gives today. `parse` becomes `documents` with each root taken out (no second parse, no copy of a node). A `...` belongs to the document before it. The existing 14 package tests keep their output byte for byte. Commit.
2. **Events** (D7). `pub enum` / `pub struct` for an event (names yours): stream start/end, document start/end (explicit or not), sequence and mapping start (block or flow) and end, scalar (style, its value through `text()`), alias; each with its anchor, tag and span (`start`, `end` byte offsets). `pub fn events(docs: ref Vec[Document]) -> Vec[Event]` (or the input as argument, if the views make that the plainer call), and `pub fn event_line(e: ref Event) -> String`, the suite's line form:
   * `+STR`, `-STR`; `+DOC` or `+DOC ---`; `-DOC` or `-DOC ...`;
   * `+SEQ`, `+SEQ []` (flow), `-SEQ`; `+MAP`, `+MAP {}` (flow), `-MAP`;
   * `=VAL` followed by the style mark and the value (a `Value::Str` gives its `Scalar`'s style and `text()`; every other scalar node its `Node.raw` and `Node.style`, y23: `=VAL :~`, `=VAL :True`, `=VAL <tag:yaml.org,2002:int> "1`): `:` plain, `'` single, `"` double, `|` literal, `>` folded; in the value `\` is `\\`, a line break `\n`, a tab `\t`, a backspace `\b`, a carriage return `\r`;
   * `=ALI *name`; an anchor is written `&name` and a tag `<tag>` (the tag as the tree keeps it, between angle brackets) after the `+SEQ`/`+MAP`/`=VAL` word and before the value, e.g. `+MAP {} &a <tag:yaml.org,2002:map>`, `=VAL &x :1`. If the tree keeps a tag in its short form (`!!str`) rather than resolved (`tag:yaml.org,2002:str`), write what the tree has and say so in `Deviation:`; resolving it is not this brief.
   * An empty node (null with no text) is `=VAL :`.
3. **Tests**: `packages/yaml/tests/events.nika` + `.stdout`: block and flow collections, every scalar style, a document with and without `---`/`...`, two documents, an anchor, an alias and a tag, nested collections; and one line per event printed with its span for one small input. Compare by hand with the suite's form above.

`load` (aliases expanded, duplicate keys, the alias limit) is the next brief, not this one.

If a form the package needs is missing or refused in the compiler, the lowering or `std` (no ADR refusing it), stop: `Result: blocked`, `Found:` the gap with a probe of a few lines, `Needs: architect decision`. Do not write around it.

## Run 2 (2026-10-31): what changed

Run 1 built step 1 (2a303980: `Node.flow`, `Document`, `documents`, test `document_markers`) and stopped before step 2: the tree dropped the text and style of a scalar that resolves to `Null`, `Bool`, `Int` or `Float` (`~` and `null` one node, `!!int "1"` without its quotes). The architect decided it (ADR-yaml-1 D7, last paragraph): `Node` has `pub raw: String` and `pub style: Style` for every scalar node, built by `y23-scalar-written.md` just before this run. Pre-flight for run 2, instead of y22's Probe A: `rg -n "pub raw: String|pub style: Style" packages/yaml/src/main.nika` shows both on `Node`, and `packages/yaml/tests/scalar_written.stdout` exists; if not, stop: `Result: blocked`, `Found: y23 not built`. Build steps 2 and 3 only. Write `tests/events.nika` before the first package run and fill its `.stdout` with `--bless` in that run (one run is 3-4 minutes).

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y21-events.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

