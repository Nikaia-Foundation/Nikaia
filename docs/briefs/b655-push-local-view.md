---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 20
---
# Brief: #655 step 1, a view pushed into a local `Vec` that never leaves is not kept (NK2302 false positive)

Score: bugs lane (`docs/priorities.md`): a false refusal in the checker that holds a detour in our own code (`check_arguments`' arm index and `check_arguments_at`); small, one area (`views.nika`). Steps 3 and 4 of #655 are done (2003b4c, deed9ad, f4dfb8c); steps 2 and 5 come in later briefs.

## Issue text (#655, item 1 and step 1)
**NK2302 false positive.** `expanded.push(Arg { given: given[at] })` into a local `Vec` that never leaves the function is refused. The rule is `views.nika` `a_field_built`; a local struct is already allowed (`views_a_local_struct`).
To build 1: NK2302: a push into a local that does not escape is treated like `views_a_local_struct`. After that, `check_arguments` drops its arm index and `check_arguments_at`.

## Probe (reproduces on f4dfb8c: `error[NK2302]: \`count\` keeps \`given\` after this call` at line 11)
```
struct E {
    n: i64
}

struct Arg {
    given: ref E?
}

fn count(given: ref Vec[E]) -> i64 {
    let mut expanded: Vec[Arg] = []
    expanded.push(Arg { given: given[0] })
    return expanded.len()
}

fn main() {
    let g = [E { n: 1 }, E { n: 2 }]
    println(f"{count(g)}")
}
```
Expected after the fix: prints `1`. Matrix to run after the one build: (a) the probe; (b) the same, but `return expanded` (the `Vec` leaves: NK2302 stays, as now); (c) `expanded` handed to another function as `mut` (stays refused unless that function's contract keeps nothing; say which in the commit); (d) push of `given[0]` straight into a local `Vec[ref E?]` that does not leave (if it fails for the lowering reason of #655 item 2, that is step 2's, not yours: note it under `Tests:`).

## Locations
* `crates/nikaia-std/src/tools/views.nika`: `views_a_local_struct` 769, `struct_handed_back` ~795 (how a local's uses are counted: reuse it for a local `Vec` read only by `len`/index/`for`), `a_field_built` 1111, `subject_destination` ~1130 (a call on the subject: `push` on a local).
* `crates/nikaia-std/src/tools/check_arguments.nika`: `CheckArgument.arm` 12-16, `check_arguments_at` 19, `check_arguments` 123. After the fix, remove the arm index and `check_arguments_at` where they only stood for this detour (the comment at 1-10 and the git log of the file say why they were added; if they serve #596 `if`/`match` arms too, keep what #596 needs and say so in the commit).
* Generated files change with them (`crates/nikaia-std/src/tools/lowered/views.rs`, `.../lowered/check_arguments.rs`, index `lowered.rs`): `docs/builder-facts.md` says how to re-lower.
* Tests: new `tests/compiler/tests/a_view_pushed_into_a_local_vec_is_not_kept.nika` + `.stdout`, and one for case (b) with `.stderr`.

## Done when
`nikaia test tests/compiler --both-settings` passes, plus the `it` filters naming NK2302 (`nikaia run scripts/where.nika -- NK2302` lists them). Commit `A view pushed into a local Vec that does not leave is not kept (Part of #655)`; no `Fixes` (steps 2 and 5 remain). Changelog line: yes (checker accepts more).

## Stop and report if
The rule needs a choice the issue does not make (e.g. a local `Vec` later read by a closure): `Result: blocked`, the question under `Owner:`.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-push-local-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
