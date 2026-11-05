---
agent: builder
model: opus
max_turns: 70
budget_usd: 14
---
# Brief: `yaml` `merge`, the `<<` merge key on request (#928 step 4, D6)

Score: #928 step 4, the step after `load` (`y27-load.md` run 2 runs before this brief in the same checkout; its commit is on your `develop`). This change touches only `packages/yaml` and needs no version raise and no `CHANGELOG.md` entry (ADR-yaml-1 D1).

Facts: Tests and the gate, Checker and views, Packages

Read ADR-yaml-1 **D6** with its paragraph of 2026-10-31 on `merge` (`docs/specification/adr/adr-yaml-1.md`), and in `packages/yaml/src/main.nika` the tree types, `Error`, `place_of`, `load`, `load_error`, `key_of`.

## Pre-flight

`rg -n "pub fn load|fn key_of" packages/yaml/src/main.nika` shows both (y27 built). If not, build nothing: `Result: blocked`, `Found: y27 not built`. Rebuild `target/fast/nikaia` (`cargo build --profile fast -p nikaia`) when it is older than `git log -1 --format=%ci -- crates/`.

## To build (architect's decisions, 2026-10-31, ADR-yaml-1 D6; helper names yours)

1. **`pub fn merge(node: Node, input: ref Bytes) -> Node sync throws`**, over a tree from `load`, every mapping at every depth (sequences, mapping values and keys walked):
   * A mapping's entry whose key is a **plain** scalar `<<` is taken out. Its value is a mapping, or a sequence of mappings; each such mapping is merged first (its own `<<`), then its entries are appended after the mapping's own entries, in order, each only when its key (`key_of`, as `load` compares keys) is not yet in the mapping: the mapping's own keys win, then the first merged mapping over a later one (`<<: [*a, *b]`: `a`'s key wins over `b`'s).
   * Any other value of `<<` (scalar, null, a sequence with a non-mapping, an `Alias` left) is an `Error` at that value's position (`load_error`), message naming what it found (`expected a mapping or a list of mappings to merge, found <kind>`, wording yours).
   * A quoted `"<<"` key is an ordinary key, untouched. Anchors and tags stay on the nodes; strings stay views (no `.clone()` of a text where the node's own `.clone()` serves).
   Commit.
2. **Tests**: `packages/yaml/tests/merge.nika` + `.stdout`: `<<: *base` with an own key overriding one of base's (order printed: own keys, then base's remaining); `<<: [*a, *b]` where `a` and `b` share a key (`a` wins); a nested merge (base itself holds a `<<`); a merge inside a sequence item; `"<<"` quoted left as a key; a scalar `<<: 1` refused with its line:column; a sequence holding a scalar refused; `merge` on a tree without `<<` gives it unchanged. Print each mapping as `key=value` lines through a small helper in the test. Write the test before the first package run, fill `.stdout` with `--bless` in that run only after `check` is clean (`docs/builder-facts.md`, *Packages*), and check every line by hand.

If a form the package needs is missing or refused in the compiler, the lowering or `std` (no ADR refusing it), stop: `Result: blocked`, `Found:` the gap with a probe of a few lines, `Needs: architect decision`. Do not write around it (a hand-written copy instead of `.clone()`, an index map because a map of views is refused, `x?.clone()` to dodge a `match` that rustc refuses: report them).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Your prompt holds the facts you need from `docs/builder-facts.md`. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y31-merge.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

