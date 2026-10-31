---
agent: builder
model: opus
max_turns: 100
budget_usd: 26
---
# Brief: #928 step 2b (properties), `yaml`: anchors, tags, aliases, from the y2 draft

## Issue (#928, copied)

The owner's direction: an official package `yaml` with a zero-copy tree pointing into the input, spans, safety limits on by default (alias expansion, depth, size), YAML 1.2 core schema, tags kept, a duplicate key an error, `<<` not merged by itself. Every gap in the compiler, lowering or `std` is a finding fixed in the compiler, never routed around in the package.

**Read first: `docs/specification/adr/adr-yaml-1.md`** (D2-D5 and D8 for this brief), then `packages/yaml/src/main.nika` and its tests as the previous brief (`docs/briefs/yaml/y1-tree-core.md`, its report beside it) left them. Its *Facts the architect probed* hold here too: input `ref Bytes`, `rule WS = "" -> { }`, the tree built inside the grammar's actions (#1000), entries as a struct (#999). Also read `y5-escapes-and-radix.md` (the brief before this one): its *Facts the last builder paid for* hold here, and the `Scalar` shape it left (a double-quoted scalar's value decoded by the grammar, every other scalar's value a view, ADR-yaml-1 D4 amended, D11).

**The check happens once, where the knowledge is** (owner, 2026-10-30; ADR-yaml-1 D5's last paragraph, D11): what the grammar can check (a block scalar header, a tag's shape, a `%YAML` version) it checks with a guard, so a mistake is a parse error with line and column, and no later function checks it again.

Block scalars (y2 step 1) are on `develop` (9d92e474). This brief builds y2's step 2, which stopped on #1026 (a view field read from a struct a grammar label binds was refused, NK1106); `y8-1026-grammar-label-struct.md` fixed it (555fe6e9; a field bound by a struct pattern in a `match` keeps its tier too). A label on a one-element group, `n:(P)`, works since #1027 (y9, 9293d338); a label on a literal returned or stored binds its text since #1030 (y10, 24751a95); the draft's `text(..)` captures stay as they are.

**Start from the draft**: branch `origin/wip/y2-properties` (ea068fa4) holds the step's `main.nika` on top of the pre-rebase step 1 (9fe7210f). Take it with `git fetch origin wip/y2-properties && git diff 9fe7210f ea068fa4 -- packages/yaml/src/main.nika | git apply` (9d92e474 has the same content as 9fe7210f, rebased). Its shape: `struct Props { start, end, anchor: String?, tag: String? }` built by `rule Props -> Props`, copied onto the `Node` in `Propped` and `Content`; a `TagKind` handed to the scalar rules so a core tag fixes and checks the kind in the grammar (`fits_node`, `empty_fits`). Keep that shape; finish and test it. The y2 report and debrief are beside the old brief (`y2-tree-properties.md.report.md`, `.debrief.md`).

**Pre-flight** (first turn, before the draft): run #1026's probe (the y8 brief holds it) with `target/fast/nikaia run`; if it does not print `12 34 12 34`, build `cargo build --profile fast -p nikaia` once (the binary may predate y8) and run it again; still refused: stop, `Result: blocked`, `Found:` #1026 not fixed.

Facts the y2 builder paid for: several named options in a rule call are `Block(n; c: c, k: x)` (Part II 10); a package test's loop is `nikaia test --project packages/yaml --bless` then `nikaia fmt FILE`; a one-letter struct name (`S`) in a probe is read as a type parameter.

## To build (one commit)

1. **Properties and aliases.** Anchors `&a`, aliases `*a` (a `Value::Alias` node, not resolved: ADR-yaml-1 D6), tags: `!local`, `!!str` and the other core tags (they fix the kind, D5: `!!str 12` is a string, `!!int "12"` an int; a core tag that fixes a kind checks the scalar's shape there, in the grammar, so `!!int "x"` is an error at its position and `as_i64` still checks nothing per digit), `!<tag:yaml.org,2002:str>`, a named handle `!e!x` (kept as written; `%TAG` resolution is the next brief's). Properties before a block collection on the line above, and on an empty node. Test `tests/anchors_tags_aliases.nika`.

The expected `.stdout` is what YAML 1.2.2 says; the yaml-test-suite case for a construct is the reference (read only, nothing copied: ADR-yaml-1 D9 waits for the owner); name it in a comment.

Explicit keys, directives and `Limits` are the next brief, `y2b-keys-directives-limits.md`: not this run.

## Done when

`target/fast/nikaia test --project packages/yaml` passes (the earlier tests included); the package gate below is green; the step committed and pushed; its message says `Part of #928`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y2c-properties.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
