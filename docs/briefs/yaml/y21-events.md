---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: `yaml` events, a walk of the parsed tree in the yaml-test-suite's form (#928 step 4, first half)

Score: #928 step 4, the next step of the package; #1000 (which step 4 needed) is built (d715e425). This change touches only `packages/yaml` and needs no version raise and no `CHANGELOG.md` entry (ADR-yaml-1 D1).

Read `docs/builder-facts.md`, ADR-yaml-1 D4, D6 and **D7 with its new paragraph** (architect, 2026-10-31), and `packages/yaml/src/main.nika` (the tree types at the top, `rule stream`/`Docs`/`NextDoc`/`AnyDoc`, `parse`). The yaml-test-suite's event form: https://github.com/yaml/yaml-test-suite (`test.event` files), summarised below. You do not need the network for it.

## To build

1. **The tree keeps what the walk needs** (D7, new paragraph). `Node` gets `pub flow: bool` (true for a flow sequence or mapping, false for everything else); every `Node { .. }` in the grammar sets it. A `pub struct Document { pub root: Node, pub start: bool, pub end: bool }` (`---` written, `...` written after it), and `pub fn documents(input: ref Bytes; limits: Limits = default_limits()) -> Vec[Document] sync throws`, with the size and depth errors `parse` gives today. `parse` becomes `documents` with each root taken out (no second parse, no copy of a node). A `...` belongs to the document before it. The existing 14 package tests keep their output byte for byte. Commit.
2. **Events** (D7). `pub enum` / `pub struct` for an event (names yours): stream start/end, document start/end (explicit or not), sequence and mapping start (block or flow) and end, scalar (style, its value through `text()`), alias; each with its anchor, tag and span (`start`, `end` byte offsets). `pub fn events(docs: ref Vec[Document]) -> Vec[Event]` (or the input as argument, if the views make that the plainer call), and `pub fn event_line(e: ref Event) -> String`, the suite's line form:
   * `+STR`, `-STR`; `+DOC` or `+DOC ---`; `-DOC` or `-DOC ...`;
   * `+SEQ`, `+SEQ []` (flow), `-SEQ`; `+MAP`, `+MAP {}` (flow), `-MAP`;
   * `=VAL` followed by the style mark and the value: `:` plain, `'` single, `"` double, `|` literal, `>` folded; in the value `\` is `\\`, a line break `\n`, a tab `\t`, a backspace `\b`, a carriage return `\r`;
   * `=ALI *name`; an anchor is written `&name` and a tag `<tag>` (the tag as the tree keeps it, between angle brackets) after the `+SEQ`/`+MAP`/`=VAL` word and before the value, e.g. `+MAP {} &a <tag:yaml.org,2002:map>`, `=VAL &x :1`. If the tree keeps a tag in its short form (`!!str`) rather than resolved (`tag:yaml.org,2002:str`), write what the tree has and say so in `Deviation:`; resolving it is not this brief.
   * An empty node (null with no text) is `=VAL :`.
3. **Tests**: `packages/yaml/tests/events.nika` + `.stdout`: block and flow collections, every scalar style, a document with and without `---`/`...`, two documents, an anchor, an alias and a tag, nested collections; and one line per event printed with its span for one small input. Compare by hand with the suite's form above.

`load` (aliases expanded, duplicate keys, the alias limit) is the next brief, not this one.

If a form the package needs is missing or refused in the compiler, the lowering or `std` (no ADR refusing it), stop: `Result: blocked`, `Found:` the gap with a probe of a few lines, `Needs: architect decision`. Do not write around it.

Report to `/home/user/Nikaia/docs/briefs/yaml/y21-events.md.report.md`.

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
