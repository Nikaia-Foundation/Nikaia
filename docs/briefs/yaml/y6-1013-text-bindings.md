---
agent: builder
model: opus
max_turns: 90
budget_usd: 25
---
# Brief: #1013, a binding inside `text(..)` is supported; the grammar's type parameter cannot collide with a program's `S` (owner's decision 2026-10-30)

**Read first: issue #1013 (copied below), Part II 10's element table (`docs/specification/20-nikaia-advance.md`, the `text(p)` row near line 671), ADR-yaml-1 D4's last bullet**, then `crates/nikaia-std/src/tools/check_grammar.nika` (`bindings_of`, the `text`/`until` case near line 115) and `crates/nikaia-std/src/tools/emit_grammar.nika` (how `text(p)` is written for winnow-grammar). The yaml escapes (`y5b-escapes.md`, the next brief) wait for this.

## The issue (#1013, label `ready`; point 1 decided by the owner 2026-10-30)

Found by the yaml lane (y5, #928).
1. `s:text(parts:Part*)`: a binding inside `text(..)` is not checked and reaches rustc. **Decided: supported.** `s` is the text `text(..)` covered, a view of the input as today; `parts` is bound as it would be without the `text(..)` around it; one pass over the input, no second parse (this is why the yaml escapes do not use `peek(text(..))`).
2. A program struct named `S` collides in rustc with the generated grammar's type parameter `S`. The emitter should use a name a program cannot declare.

## To build (each step a commit)

0. **Pre-flight (first five turns):** a scratch project with `rule top -> T = s:text(ps:P*) eof -> { T { raw: s, n: ps.len() } }` over `Bytes`, and one with `struct S { a: i64 }` beside any grammar; `nikaia run` both and keep the rustc errors. Read how winnow-grammar handles the form `emit_grammar.nika` writes for `text(p)`: if its engine discards the inner values (a `take`/recognize), the lowering is ours to change (keep the start, run `p` with its bindings, slice the covered input), not a reason to stop. Stop with `Result: blocked`, `Found:` the probe, `Needs: owner decision` only if the form needs a change inside winnow-grammar itself (the main session reports `peek` binding upstream already; do not edit `~/.cargo`).
1. **Bindings inside `text(..)`**: the checker types the inner bindings as without `text(..)`, the emitter keeps them, the outer binding stays the covered text (a view; `text::Span` rules unchanged). Nested `text(text(..))` and a binding under a repeat inside `text(..)` behave as without the outer `text`. Part II 10: one sentence in the `text(p)` row or right under the table saying the inner bindings are bound as in `p` (spec text: `Changelog:` one sentence; the version raise is the supervisor's).
2. **#1013 point 2**: the emitted grammar's type parameter(s) take a name a program cannot declare (follow the convention the lowering already uses for its own names; name it in `Body:`).
3. **Tests** in `tests/compiler/tests/`: `grammar_text_binds_its_inner_parts.nika` (+ `.stdout`: the raw text and the decoded parts of one input, one pass), `grammar_a_struct_named_s.nika` (+ `.stdout`). Run each through `nikaia test --project tests/compiler` (builder-facts), and against the unfixed compiler once (they must fail there; say so under `Tests:`).

The last commit says `Fixes #1013` and `Part of #928`. This brief changes `crates/`: `lower-std` for every changed `tools/*.nika`, and the gate of `.claude/agents/builder.md` (#852) applies, including the whole `it` binary if the parser changes.

Files this brief may touch (beyond `packages/yaml/`): `crates/nikaia-std/src/tools/check_grammar.nika`, `emit_grammar.nika` and what they call, their lowered files, `tests/compiler/tests/grammar_*`, Part II 10 of `docs/specification/20-nikaia-advance.md`.

## Facts the last builders paid for

* A grammar action's claims are proved since 43312d30 (`prover_bodies`); the guards at an alternative's head are facts.
* The lowering cache misses an inferred `sync` (#1014): a `tools/*.nika` function whose body turns `async` leaves callers stale; declare it `sync`.
* `nikaia fmt` can break a long rule line holding `if(cond, "label")` (#1015): check the formatted test still parses.
* In winnow-grammar `peek`, `not`, cut, guard and `fail` cannot be bound (`winnow-grammar-model/src/parser.rs`); not this brief's.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y6-1013-text-bindings.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
