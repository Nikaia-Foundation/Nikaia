---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #928 step 2a, the `yaml` package's skeleton and its core grammar into a zero-copy tree with spans

## Issue (#928, copied)

The owner's direction: an official package `yaml` (ADR-338 D2, `packages/yaml/`), a native API nobody fills well (a zero-copy tree pointing into the input, spans for linters and language servers, format-preserving editing, safety limits on by default), later a drop-in Rust facade for the archived `serde_yaml` 0.9. Behaviour is serde_yaml's: YAML 1.2 core schema (`yes`/`no`/`on` are strings), a duplicate key an error, `<<` not merged by itself, tags kept. Every gap in the compiler, lowering or `std` met on the way is a finding fixed in the compiler, never routed around in the package.

**Read first: `docs/specification/adr/adr-yaml-1.md`** (D1-D5 are this brief; D6-D10 are later briefs). The precedent package is `packages/json/` (`nikaia.toml`, `src/main.nika`, `tests/*.nika` with `.stdout`, a test says `use json`).

## Facts the architect probed (target/fast/nikaia, develop 833321f)

* A `grammar` handed `ref Bytes` builds structs and enums whose `String` fields are views of the input, UTF-8 checked as it reads; `a:here` gives a `u32` byte offset; a `pub fn parse(input: ref Bytes) -> Value throws { return G::doc(input) }` exports it to a test that says `use yaml` (a struct `Node { start: u32, end: u32, value: Value }` inside `Value::Seq(Vec[Node])` works). Taking `input: Bytes` by value is `NK2302`; write `ref Bytes`.
* `here` and `span` over a `String` are `NK1187` (#743): the input is `ref Bytes` (ADR-yaml-1 D2). In a test: `let input = Bytes("a: 1\n".bytes().collect())`.
* A rule named with an uppercase letter is lexical (nothing skipped between its elements), a lowercase one syntactic (`WS` skipped between elements), Part II 10.8. YAML's whitespace is structure: declare `rule WS = "" -> { }` and write spaces explicitly, as `tests/compiler/tests/grammar_yaml_block_structure_by_indentation.nika` does (rule parameters `n`, `c`, `count(" ")`, `" "{m}`, guards `if(...)`, ADR-grammar-1). Start from that test's grammar.
* A rule result that is a tuple mixing `String` with another type refuses the views (#999): a mapping entry is a struct `Entry { key: Node, value: Node }` (ADR-yaml-1 D4), not a tuple.
* A plain function that builds a struct or enum from views of a `ref` parameter is refused or mis-lowered (#1000): **build the whole tree inside the grammar's actions**. A function that computes new text (a scalar's value with escapes decoded) returns text of its own and is fine.
* The error a failing entry throws is `ParseError`; print it with `f"{error}"` (it has no `.message()`); `catch { ... error ... }` is the form (`packages/json/tests/errors_carry_position_and_path.nika`).

## To build (each step a commit)

1. **Skeleton.** `packages/yaml/nikaia.toml` (`name = "yaml"`, `version = "0.1.0"`), `src/main.nika` with a header comment in the manner of `packages/json/src/main.nika`, the types of ADR-yaml-1 D4 (`Node`, `Value`, `Entry`, `Scalar`, `Style`) and `Error { message, line, column, offset }` with `impl Error`, and `pub fn parse(input: ref Bytes) -> Vec[Node] throws` (one `Node` per document of the stream). Test `tests/empty_stream_and_comments.nika`: an empty input, a comment-only input, `a` alone.
2. **Grammar, structure.** Documents: implicit, `---` starting one, `...` ending one, several in a stream. Comments: on their own line and after a value. Block sequences, block mappings with implicit single-line keys (plain or quoted), compact nested entries (`- - a`, `- k: v`), a sequence as a mapping's value at the mapping's own indentation, an empty value (`k:` is `Null`). Flow sequences and flow mappings, nested, across lines, with a trailing comma. Every `Node` has `start`/`end`: from its first character to just after its last, never including trailing spaces, a comment or the line break. Tests `tests/block_collections.nika`, `tests/flow_collections.nika`, `tests/documents.nika`, `tests/spans.nika` (offsets of every node in a small document holding a two-byte character).
3. **Scalars.** Plain scalars, single- and multi-line, in block and flow context (the flow indicators end them inside flow only); single-quoted (`''`); double-quoted (`\n \t \\ \" \/ \0 \x.. \u.... \U........`, escaped line break, line folding). `raw` is the view as written; `pub fn text(s: ref Scalar) -> String` computes the value (YAML 1.2.2 6.5 line folding, 7.3 quoted styles). Plain scalars resolve by the core schema in the grammar's action (ADR-yaml-1 D5); `pub fn as_i64(v: ref Value) -> i64 throws`, `as_f64` convert `Int`/`Float` (decimal, `0o`, `0x`, `.inf`, `.nan`). Tests `tests/plain_scalars_resolve_by_the_core_schema.nika` (`yes`, `no`, `on`, `off` print as strings; `~`, `null`, `NULL`, empty; `true`, `False`; `0x1F`, `0o17`, `-12`, `1e3`, `.inf`, `-.Inf`, `.NaN`; `012` is an int; `1_000` is a string), `tests/quoted_scalars.nika`, `tests/multi_line_scalars.nika`.
4. **Errors.** A malformed input throws `yaml::Error` with line and column (from 1, in characters) computed from the byte offset where reading stopped. Test `tests/errors_carry_line_and_column.nika`: a tab used as indentation, an unclosed flow sequence, an entry indented one space less than its siblings, an invalid UTF-8 byte.

The expected `.stdout` of each test is what YAML 1.2.2 says; where you are unsure, the yaml-test-suite case for that construct (`https://github.com/yaml/yaml-test-suite`, read only, nothing copied into the repository yet: D9 waits for the owner) is the reference; name it in a comment. A test helper `show(node)` printing `{k: v}`, `[a, b]`, `str(..)`, `int(..)`, `null` is written in each test (tests cannot share code).

Not in this brief: block scalars, anchors, tags, aliases, explicit `?` keys, directives, limits (next brief, `y2-tree-properties.md`); events and `load` (after #1000).

## Done when

`target/fast/nikaia test --project packages/yaml` passes; the package gate below is green; each step committed and pushed; the last commit's message says `Part of #928`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y1-tree-core.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
