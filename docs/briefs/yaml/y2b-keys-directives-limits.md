---
agent: builder
model: opus
max_turns: 100
budget_usd: 26
---
# Brief: #928 step 2b (second half), `yaml`: explicit keys, directives, limits

## Issue (#928, copied)

The owner's direction: an official package `yaml` with a zero-copy tree pointing into the input, spans, safety limits on by default (alias expansion, depth, size), YAML 1.2 core schema, tags kept, a duplicate key an error, `<<` not merged by itself. Every gap in the compiler, lowering or `std` is a finding fixed in the compiler, never routed around in the package.

**Read first: `docs/specification/adr/adr-yaml-1.md`** (D2-D5 and D8 for this brief), then `packages/yaml/src/main.nika` and its tests as `y2-tree-properties.md` (block scalars) and `y2c-properties.md` (properties, aliases; reports beside them) left them; y1's report too. Its *Facts the architect probed* hold here too: input `ref Bytes`, `rule WS = "" -> { }`, the tree built inside the grammar's actions (#1000), entries as a struct (#999). Also read `y5-escapes-and-radix.md`: its *Facts the last builder paid for* hold here, and the `Scalar` shape it left (a double-quoted scalar's value decoded by the grammar, every other scalar's value a view, ADR-yaml-1 D4 amended, D11).

**The check happens once, where the knowledge is** (owner, 2026-10-30; ADR-yaml-1 D5's last paragraph, D11): what the grammar can check (a block scalar header, a tag's shape, a `%YAML` version) it checks with a guard, so a mistake is a parse error with line and column, and no later function checks it again.

## What changed since this brief was written (architect, 2026-10-31)

* The step before this one is y2c (95a690fe, `Props`, `TagKind` handed to the scalar rules, `fits_node`): read its commit message; its tags `!e!x` are kept as written, this brief resolves them.
* **Pre-flight**: #1041 (`y17-1041-entry-takes-a-parameter.md`, the brief before this one) lets `parse(input: ref Bytes, limits: Limits)` hand `limits` to the entry rule; before step 2 run its test `tests/compiler/tests/grammar_an_entry_takes_a_parameter_of_its_caller.nika`. If it is not on develop or fails, stop after step 1 and report `partial` with `Needs: #1041`; do not pass the fields one by one around it.
* **Probed by the architect** on develop 6064835c (`String` input, scratch file): a rule taking `ds: Vec[Dir]` (a struct of two views made by an earlier rule `dir -> Dir`, `%TAG !e! tag:ex,2000:`) and an `i64` read in a guard, called as `t:Tag(ds, lim.depth)` after `ds:dir*`, prints `tag:ex,2000:x` for `!e!x`; an entry rule `doc(lim: Limits)` given a struct literal or a local works. So the `%TAG` handles can be handed down the rules as a parameter, and the depth as an `i64` parameter checked by a guard.
* **Not in an action**: `return` (rustc today, #1040 waits on the owner): compute the value with a `let mut` and a loop.
* No issue closes here: write `Part: #928` in the report, never `Fixes: none` (y2c's commit message read `Fixes none`).
* `nikaia fmt` keeps a grammar rule's guard whole now (#1015, 2e553fe5); run it on every changed `.nika` before the gate.

## To build (each step a commit)

1. **Explicit keys and directives.** `? key` / `: value` in block and flow, a collection as a key; `%YAML 1.2` (another major version an error, `1.1` accepted as 1.2), `%TAG !e! tag:example.com,2000:` applied to `!e!x` in that document only; a directive needs `---`. Tests `tests/explicit_keys.nika`, `tests/directives.nika`.
2. **Limits (D8).** `pub struct Limits { pub depth: i64, pub alias_nodes: i64, pub bytes: i64 }` with the defaults of D8 (`alias_nodes` is checked by `load`, a later brief: store it, document it), and `parse` taking a `Limits` the caller may pass (omitted: the defaults; follow how an entry rule takes values after its input, `tests/compiler/tests/grammar_an_entry_takes_values_after_its_input.nika`). The parse refuses nesting deeper than `depth` (a depth parameter on the rules and a guard) and an input longer than `bytes`, with an `Error` naming the limit, its value and the position. Test `tests/limits_depth_and_size.nika`: 129 nested flow sequences refused by default, accepted with `depth: 200`; 128 accepted; an input over a small `bytes` refused.

The expected `.stdout` is what YAML 1.2.2 says; the yaml-test-suite case for a construct is the reference (read only, nothing copied: ADR-yaml-1 D9 waits for the owner); name it in a comment.

## Done when

`target/fast/nikaia test --project packages/yaml` passes (the earlier tests included); the package gate below is green; each step committed and pushed; the last commit's message says `Part of #928`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y2b-keys-directives-limits.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
