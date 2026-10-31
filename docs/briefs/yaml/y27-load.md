---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: `yaml` `load`, the tree a program reads: aliases expanded within the limit, a duplicate key an error (#928 step 4, second half)

Score: #928 step 4, the next step of the package after `y21-events.md` (which runs just before this brief, in the same checkout; its commit is on your `develop`). This change touches only `packages/yaml` and needs no version raise and no `CHANGELOG.md` entry (ADR-yaml-1 D1).

Read `docs/builder-facts.md`, ADR-yaml-1 **D4, D6, D8** (`docs/specification/adr/adr-yaml-1.md`), and in `packages/yaml/src/main.nika` the tree types at the top, `Limits`, `Error`, `place_of`, `documents`, `parse`, `as_i64`, `as_f64`.

## Pre-flight

`rg -n "pub fn events|pub fn event_line" packages/yaml/src/main.nika` shows both (y21 run 3 built). If not, build nothing here: `Result: blocked`, `Found: y21 not built`. Rebuild `target/fast/nikaia` first (`cargo build --profile fast -p nikaia`): it must hold #1070-#1072 (y24-y26).

## To build (architect's decisions, 2026-10-31; names yours where not given)

1. **`pub fn load(input: ref Bytes; limits: Limits = default_limits()) -> Vec[Node] sync throws`**: `documents` (one parse), then each root resolved, one `Node` per document. The strings stay views of the input (D4, D6): a copied node is its structure copied (`.clone()` of a `Node` whose `String`s are views is a copy of the views, not of text). The result has no `Value::Alias` left; anchors and tags stay on the nodes as written.
   * **Aliases**: an alias is replaced by the resolved node of the most recent anchor of that name before it **in the same document** (anchors do not cross documents). An alias to an anchor not seen before it is an `Error` (`unknown anchor *name`, position of the alias). An alias inside the node its own anchor names (`&a [ *a ]`) is an `Error` (`alias *a inside its own anchor`), as libyaml-based loaders refuse a recursive alias; an earlier, finished `&a` does not make it legal.
   * **Alias limit (D8)**: per document, the nodes the expansions produce (each alias counts the node count of the resolved node it copies, its own expansions included) may not exceed `10 * <the document's own nodes> + limits.alias_nodes`. Keep each anchor's resolved node together with its node count, and check **before** copying, so a billion laughs stops at the first alias over the limit without building it. The error names the limit and its value and the alias's position (`alias expansion exceeds <n> nodes (alias_nodes limit <alias_nodes>)`, wording yours).
   * **Duplicate keys (D6)**: a key given twice in one mapping (after alias expansion) is an `Error` at the second key's position, whose message also names the first's line and column (`duplicate key at L:C, first at L:C`). Keys compare by kind and value: `Null` = `Null`; `Bool` by its bool; `Str` by `text()` (so `a` and `"a"` are one key); `Int` by `as_i64` (so `16` and `0x10` are one key; an `Int` that does not fit compares by its text); `Float` by `as_f64` (a NaN equals no key); `Seq`/`Map` structurally, element by element and entry by entry in order. Different kinds never match (`1` and `"1"` are two keys).
   * **`<<` is an ordinary key**: nothing is merged (D6). A merge function on request is the next brief, not this one.
   * `parse` and `documents` stay as they are (the text as written).
   Commit.
2. **Tests**: `packages/yaml/tests/load.nika` + `.stdout`: an anchor used twice (both copies printed); an alias inside a nested collection; anchors are per document (two documents, the second's `*a` unknown); unknown anchor; recursive alias; a billion laughs of 9 levels with 9 aliases each stops with the limit error, and the same with a raised `Limits { alias_nodes: .. }` small enough to build passes; a `Limits` lowered so an ordinary anchor fails; duplicate keys: `a`/`"a"`, `16`/`0x10`, `[1, 2]` as a key twice, and `1`/`"1"` accepted; `<<: *base` kept as a key `<<` with the copied mapping as its value. Write the test before the first package run and fill `.stdout` with `--bless` in that run (one run is 3-4 minutes); check every line by hand.

If a form the package needs is missing or refused in the compiler, the lowering or `std` (no ADR refusing it), stop: `Result: blocked`, `Found:` the gap with a probe of a few lines, `Needs: architect decision`. Do not write around it (a hand-written deep copy instead of `.clone()`, a map from anchor to index because a `Map[String, Node]` of views is refused, are both ways around: report them).

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y27-load.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

