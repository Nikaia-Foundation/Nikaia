---
agent: builder
model: opus
max_turns: 90
budget_usd: 22
---
# Brief: #1039, a grammar action reads a name the emitter never binds (unbound `text(d:P+)`, a name below a repeat)

Score: a defect (rustc errors on programs the checker passed), found by this lane (y14); nothing in `packages/yaml` needs it, it touches only the grammar checker and emitter. Runs before `y2b-keys-directives-limits.md`.

Read `docs/builder-facts.md`. Read bdbe8a7a's and 03d43063's commit messages first (`git show --stat <sha>`): they say how a bound `s:text(p)` binds its inner names (`(a) @ __nikaia_s_from ... (c) @ __nikaia_s_to`, `nikaia_std::grammar::covered`) and how a guard reads them (`__nikaia_covered`). The unbound form is to go the same way. y14's debrief: its first fix also let the unbound form through to a guard, and only a probe after the build showed rustc refusing it; so **run the five probe rows below on the unfixed binary first**, then after each build.

## Issue #1039 (copied)

## Found while

y14 (#1013 step 4, 03d43063, 2026-10-31), building the `yaml` package (#928). The checker's `bindings_of` (`crates/nikaia-std/src/tools/check_grammar.nika:158`) hands the action every name bound anywhere inside a pattern; the emitter binds a name only where `emit_pattern`'s `grammar_nested` rule lets it through (Seq, Group, Bind pass, everything else is nested; `emit_pattern.nika:14`) and, inside `text(..)`, only for a **bound** `s:text(p)` (`emit_is_a_binding_text`, `emit_pattern.nika:373`; bdbe8a7a). Where the two disagree the program passes the checker and rustc says `Cannot find value d`.

Part II 10 (`docs/specification/20-nikaia-advance.md:731`), the `text(p)` row: "a name inside `p` is bound as it is without the `text(..)` around it". That holds for an unbound `text(..)` too: a defect, not a decision.

## Probes (target/fast/nikaia on develop 6064835c; each with `rule WS = "" -> { }` and `rule Ch -> i64 = x:any if(x >= '0' && x <= '9') -> { 1 }`, called with `catch { println("error") return }`)

| # | entry rule | input | today | wanted |
| :- | :- | :- | :- | :- |
| a | `plain -> String = text(d:Ch+) ";" -> { f"{d.len()}" }` | `"12;"` | rustc: `d` not found | prints `2` |
| b | `plain -> String = s:text(text(d:Ch+) "-") ";" -> { f"{s} {d.len()}" }` | `"12-;"` | rustc: `d` not found | prints `12- 2` |
| c | `plain -> String = text(d:Ch+) if(d.len() > 1) ";" -> { "ok" }` | `"12;"` | NK1117 at the guard's `d` (y14 kept it inside on purpose because the emitter did not bind it) | prints `ok` |
| d | `plain -> String = (d:Ch ",")* ";" -> { f"{d}" }` | `"1,2,;"` | rustc: `d` not found | NK1117 at the action's `d` (a name below a repeat is not the action's: Part II 10's `p*` row yields a list of what `p` yields, nothing binds inside) |
| e | bound `s:text(d:Ch+) if(d.len() > 1) ";" -> { s }` (the y14 test) | `"12;"` / `"1;"` | `12` / `error` | unchanged |

## To build

1. **The emitter binds the names inside an unbound `text(p)`** at the alternative's level (rows a, b, c): written as the bound form with a hidden label (`__nikaia_text_N`, the lowering's own names start with `__nikaia_`), so `emit_text_bindings`, `emit_text_edge`, `emit_texts_a_guard_reads` (`emit_pattern.nika:386`) and the action's prelude (`emit_grammar.nika:103`) treat it as they treat `s:text(p)`. The checker's `grammar_values` (`check_grammar.nika:320`, helper `grammar_a_text_call:43`) then hands those names on to a later guard for the unbound form as well (row c). Over `Bytes` the covered text keeps its UTF-8 check (`covered_bytes`), as the bound form.
2. **The checker's action scope is the emitter's**: `bindings_of` hands the action only the names the emitter binds (below a repeat, a counted repeat, a choice, or a built-in's argument other than `text(..)` at the alternative's level: not bound), so row d is NK1117 at the use, with its span. The code exists; no new NK code. Untouched: what a `fold(p, init, step)`'s `step` reads of `p` (its own scope, `emit_fold`), and a bound repeat `xs:(d:Ch ",")*` (the label is the action's, `d` is not). Every `tests/compiler/tests/grammar_*` test stays green; one that reads a name below a repeat in its action and passes today is a counter-example: stop and report it.
3. Tests in `tests/compiler/tests/`: `grammar_an_unbound_text_binds_its_inner_names.nika` (rows a, b, c; `.stdout`), `grammar_a_name_below_a_repeat_is_not_the_actions.nika` (row d; `.stderr`). Part II 10's `text(p)` row: say it holds bound or not; the `x:p` row: a name below a repeat or a choice is not in the action's scope.

## Locations

`check_grammar.nika`: `bindings_of:158`, `grammar_values:320`, `grammar_a_text_call:43`. `emit_pattern.nika`: `emit_pattern:10` (`grammar_nested`), `emit_is_a_binding_text:373`, `emit_texts_a_guard_reads:386`, `emit_text_nested:442`, `emit_text_edge:452`. `emit_grammar.nika`: `emit_text_bindings:103`. Lowered twins in `crates/nikaia-std/src/tools/lowered/`. y14's test `tests/compiler/tests/grammar_a_guard_reads_a_name_bound_inside_text.nika` and bdbe8a7a's `grammar_text_binds_its_inner_parts.nika` must stay green.

Not needed by `yaml` (it reads no such name); built in the yaml lane because it touches only the grammar checker and emitter.

## Done when

Rows a-e print what the table wants; both new tests pass and fail on the unfixed binary (say so under `Tests:`); `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --both-settings --project tests/compiler > out.txt 2>&1` green with every `grammar_` test in it; the package gate below green; committed and pushed with `Fixes #1039`, both steps in one commit or two. Files you may change: the files under *Locations* with their lowered twins (`crates/nikaia-std/src/tools/lowered/*.rs`, regenerate them as `docs/builder-facts.md` says), `crates/nikaia-std/src/grammar.rs` if the covered-text helper needs it, the Part II 10 rows, the two new tests with their expected output. Name under `Changelog:` what the change decided. Stop and report if an existing test reads a name the new rule refuses.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y16-1039-unbound-text-names.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
