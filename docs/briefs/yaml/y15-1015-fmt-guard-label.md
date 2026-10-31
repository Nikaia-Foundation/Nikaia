---
agent: builder
model: opus
max_turns: 70
budget_usd: 18
---
# Brief: #1015, `nikaia fmt` keeps a grammar rule line holding `if(cond, "label")` parseable

Score: a defect that blocks this lane (y2c run 2 stopped on it: the draft's 9 rule lines over 100 columns with a labelled guard do not survive `fmt`, so the package gate's `fmt::every_nika_file` is red). Runs first; `y2c-properties.md` (run 3) waits on it.

## Issue (#1015, copied)

A long rule line with a guard `if(cond, "label")` is reflowed by `nikaia fmt` (`crates/nikaia-std/src/tools/fmt.nika`) into text that no longer parses. To build: (1) the formatter breaks guard arguments only where the grammar parser accepts a break (or keeps the guard on one line); (2) a case the `fmt::every_nika_file` set formats.

## Probes (the architect ran both on today's develop, `target/fast/nikaia` of 06:15)

```
fn f(a: i64, b: i64) -> bool sync {
	return a < b
}

grammar G {
	rule WS = "" -> { }
	entry rule top -> i64 = a:here abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuv:here if(f(1, 2), "a node its tag fits") eof -> { 1 }
}

fn main() {
	println("x")
}
```

1. `nikaia check` passes; `nikaia fmt` refuses: "the formatted file does not parse: Expected `)` here, but found `,`" (line 9, column 24). The line is over 100 columns, `reshape` (fmt.nika:720) finds no top-level comma and no operator, and breaks the last bracket group that holds a comma, the guard's `if(...)`, through `fmt_broken` (:929), which puts each argument on its own line with a trailing `,` the grammar parser (winnow-grammar's, not ours) does not take. The same line under 100 columns formats fine.
2. The same rule broken by hand after `a:here ...:here` with the continuation at two tabs: `check` passes, `fmt` writes the continuation at one tab, the rule's own level (a second layout defect: a continuation line belongs one level deeper than the line it continues, Part III `nikaia fmt` and how `fmt` breaks any other long line).

## To build (two commits)

1. **No break that does not parse.** A guard's `if(..)` (and `fail(..)`, any grammar call with arguments the grammar parser reads) inside a `rule` line is never broken by `fmt_broken`: treat it as sealed (`fmt_sealed`, :851, or the caller in `reshape`), so a long rule line breaks elsewhere or stays long (`reshape` returns `[]` when nothing breakable is left, as for any unbreakable line). Do not change winnow-grammar. Test `tests/compiler/tests/fmt_keeps_a_grammar_guard_whole.nika` + `.stdout`: probe 1 with the guard's line over 100 columns, `main` calling `G::top` on an input it accepts and printing the result; `fmt::every_nika_file` formats it (it scans `tests/` as it scans `packages/`, as `fmt_keeps_a_nested_tuple_closure_whole.nika` shows), and it is red on the unfixed binary (say so under `Tests:`).
2. **A rule's continuation one level deeper.** A rule line broken by hand (or by `fmt`, between sequence elements) keeps its continuation at the rule's indentation + 1, and formatting again changes nothing (fixed point). Look at `indent_of` (:631) and the frames a `grammar { }` body pushes. Add probe 2's shape to the step 1 test (a second rule, broken by hand) so the fixed point is checked.

`fmt.nika` lives under `crates/nikaia-std/src/tools/`: format it by hand (never `nikaia fmt` on it, builder-facts), then `nikaia lower-std` and commit its lowered twin `crates/nikaia-std/src/tools/lowered/fmt.rs` with it. Read `docs/builder-facts.md` (its first trap: the winnow-grammar rev of `target/fast/nikaia`).

## Done when

Both probes format into a file that parses and is a fixed point; the new test passes; the gate of `.claude/agents/builder.md` (#852) and `cargo test --profile fast -p nikaia --test it -- fmt::` green; then, as a check for the next brief, `git fetch origin wip/y2c-properties && git show origin/wip/y2c-properties:packages/yaml/src/main.nika > $SCRATCH/main.nika` and `target/fast/nikaia fmt $SCRATCH/main.nika` (after rebuilding `target/fast/nikaia`) formats and the result passes `nikaia check` (say so under `Tests:`; do not commit that file). Committed and pushed, step 2's commit with `Fixes #1015`. Files you may change: `fmt.nika`, its lowered twin, the new test and its `.stdout`. Name under `Changelog:` what the change decided.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y15-1015-fmt-guard-label.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
