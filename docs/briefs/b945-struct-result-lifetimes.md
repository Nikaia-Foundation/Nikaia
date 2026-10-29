---
agent: builder
model: sonnet
max_turns: 110
budget_usd: 25
---
# Brief b945: a struct result points into the text parameter it is built from; a parameter only moved into a local struct is lent; a struct result viewing a local takes the caller's keep (#945 step 1, #942 steps 2 and 5)

Score: bugs before moving (`CLAUDE.md`, `docs/priorities.md`); #945 blocks the `yaml` package (#928), and every shape here reaches rustc today (a message about a file nobody wrote, Part III C.1). Decided: ADR-353 D8, D10 (d84a15f); probe 1 needs no decision (ADR-353 D1, D5; ADR-283 D24).

## Issue text (#945 and #942, copied; builders have no GitHub)
Common part of the probes (#945):
```nika
struct Event { text: String, line: i64 }
struct Parser { input: String, pos: i64 }

impl Parser {
    fn new(input: String) -> Parser { return Parser { input, pos: 0 } }

    fn next(ref mut self) -> Event? {
        let lines = self.input.lines().collect()
        if self.pos >= lines.len() { return null }
        let e = Event { text: lines[self.pos].trim(), line: self.pos + 1 }
        self.pos += 1
        return e
    }
}
```
**Probe 1** (#945): `main` builds `Parser { input: text.trim(), pos: 0 }`, so the tiers make `input` a view: `struct Parser<'a>`, `fn next(&mut self) -> Option<Event<'a>>`, correct. But `new` lowers as `fn new(input: &str) -> Parser<'a> { .. }`; rustc: *Explicit lifetime required in the type of `input`*. Expected `fn new(input: &'a str) -> Parser<'a>`. Without `new` the program runs and prints both events.

**Probe 2** (#945, now #942 step 2): a text parameter moved into a local struct, views handed back:
```nika
fn events(text: String) -> Vec[Event] {
    let mut p = Parser { input: text, pos: 0 }
    let mut events: Vec[Event] = []
    while true {
        let e = p.next() ?? break
        events.push(e)
    }
    return events
}
```
Lowered today: `Parser { input: String, .. }` (owned), `fn next(&mut self) -> Option<Event<'_>>`, `fn events(text: String) -> Vec<Event<'static>>`; rustc: *Cannot borrow `p` as mutable more than once at a time*. ADR-353 D8: a text parameter whose body only moves it into a local struct (tuple, list, variant) is not kept (Part I 6.5): it stays a view (`&'a str`), `input` is a view, `events` returns `Vec<Event<'a>>`.

**F7** (#942 step 5, `docs/decision-942-keep-moves.md` §4):
```nika
use std::fs
struct View { head: String }
struct Cursor { input: String, pos: i64 }
fn parse(t: String) -> View { return View { head: t } }
fn make() -> Cursor throws {
    let text = fs::read_to_string("in.txt", fs::Root::Anywhere)
    let v = parse(text)
    let c = Cursor { input: v.head, pos: 0 }
    println(v.head)
    return c
}
```
Today `NK2105` ("`v.head` was stored in `Cursor.input`"; help `.clone()`): the tiers make the returned field owned. ADR-353 D10: a struct (variant, tuple, list) result whose views point into what the body owns puts that buffer in the caller's keep (ADR-283 D10, D23), as `fn d() -> ref String` already does; the field stays a view.

## Locations (resolved 2026-10-29)
* Parameter lifetimes in a signature: `emit_how` in `crates/nikaia-std/src/tools/emit_function.nika` (~:217; `sig.carries_input`, `of_the_input` / `over_the_input` in `tools/emit_kinds.nika:148-169`).
* Whether a parameter is kept: `crates/nikaia-std/src/tools/keeps.nika` (`hand_over`, `classify`, `keeps_expression`); a move into a struct literal counts as keeping today.
* The owned fallback for a returned field: `crates/nikaia-std/src/tools/text_tiers.nika:1423` (doc of the rule; "a field it was stored in").
* The caller's keep for a returned view of a local (D23, the path to extend): `tools/emit_keeps.nika` (`emit_keep_expr`, :23; `__keep_frame`), `tools/keep.nika`, `tools/tether.nika`; tests of that path: `tests/compiler/tests/a_view_of_a_text_field_of_a_struct_a_call_made_lives_in_the_callers_keep.*`, `an_argument_made_in_a_call_to_a_function_with_a_keep_is_kept.*`, `crates/nikaia/tests/it/tether_keep.rs`, `result_source.rs`.
* Spec: Part I 6.5 (parameter rule), 6.6 (*What a body owns is handed out the same way*; *The same holds for a struct literal handed back*); ADR-353 D8, D10.
* Read `docs/builder-facts.md`. Use `nikaia run scripts/where.nika -- NAME` for any other definition.

## To build
1. **Constructor (#945 step 1)**: a function that builds a struct with a view field from a text parameter lowers the parameter with the struct's lifetime, for a named one (`Parser::new`) and the anonymous constructor (`pub fn(input: String) -> Parser`, called `Parser(text)`, Part I 4.2). Test: `tests/compiler/tests/a_constructor_of_a_struct_of_views_points_into_its_parameter.nika` + `.stdout`: probe 1 with `Parser::new(text.trim())` and a second struct built by its anonymous constructor, both called from `main`, the events of both printed. Commit `Fixes #945`.
2. **Parameter lent (#942 step 2)**: probe 2 runs. Test: `tests/compiler/tests/a_text_parameter_moved_into_a_local_struct_is_lent.nika` + `.stdout` (`main` reads a text, calls `events(text)`, prints each event's `text` and `line`, then prints `text` itself, which is still there). Commit `Part of #942`.
3. **Struct result (#942 step 5)**: F7 runs, `c.input` read in `main` after `make()`. Test: `tests/compiler/tests/a_struct_result_viewing_a_local_lives_in_the_callers_keep.nika` + `.stdout` (in the test, the local text is text of its own made without a file, `let text = f"a: {n}"` with `n` a parameter or local; not `.clone()` on a literal, which is `NK1301`). Also a tuple result `(View, i64)` viewing the local, in the same test. Commit `Part of #942`.

## Done when
The three new tests pass (`target/fast/nikaia test --project tests/compiler` filtered to each), each fails on the compiler before your change (say so under `Tests:`), the keep tests named above and `it -- tether_keep result_source` pass, the gate of builder.md (a change to `tools/emit_*.nika` runs `nikaia test --both-settings --project tests/compiler`), `lower-std` changes only the lowered files of the `.nika` you changed, clippy, `cargo fmt`, `scripts/pre-push-check.sh`. Changelog: one sentence per step.

## Stop and report if
A step needs a rule Part I/ADR-353 does not state (then `Owner:` names it), or a file another agent has uncommitted (`git status` first). Do not touch `tools/view_changes.nika` or `tools/buffers.nika` beyond what a step needs: the next brief (b942) changes them.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b945-struct-result-lifetimes.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
