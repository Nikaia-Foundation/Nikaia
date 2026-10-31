---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief p11: #1090, a precondition carried back to a function's entry counts as proved although a call to that function does not prove it

Facts: Where things live, Traps already met, Commands

Score: S=3 (the prover's own report claims a proof that is not there, and NK1300 lets the program through: the first thing an outside reader of `--asserts` or of claim (b) would find), O=3 (every user's preconditions that cross one function), C=3 M (one count in `prover_passes.nika`, two tests). Area: prover. Before p9 (claims in the figure), whose claim count would otherwise report these as proved.

## Decision (architect, 2026-10-31: a defect, kind 3; ADR-269 D15, D20, ADR-380 D1 already say what is right; same family as #1076/p8)

A claim held as a precondition is proved only when every call that reaches it proves it. A call carried back to its caller's entry (D15) proves nothing by itself: the precondition is then the caller's, and its callers decide. So the unproved calls of a claim are those of its own function **plus those of every function its precondition was carried to, transitively** (the carrying stops at cycles, pass 1's comment). A claim with one or more such calls reads "one call checks it when it runs" / is refused as today for `h`'s direct calls; the default build gives NK1300 (ADR-380 D1). The text may keep naming `h` (the `assert`'s function); change it only if it is wrong, say so in `Body:`.

## Problem (probed 2026-10-31 on the binary of 76c216e)

```nika
fn h(b: ref Vec[u8]) -> i64 sync {
    assert(b.len() >= 4)
    return b[3] as i64
}

fn hb(x: ref Vec[u8]) -> i64 sync {
    return h(x)
}

fn main() {
    let xs = [1, 2, 3]
    let mut u: Vec[u8] = []
    u.resize(xs[2], 0)
    print(f"{hb(u)}")
}
```

`nikaia check --asserts`: `preconditions proved at every call 1`, `precondition of \`h\`: proved at every call the compiler sees`, `Checked`. `nikaia run`: stops, `precondition of \`hb\`: \`x.len() >= 4\`, from \`assert(b.len() >= 4)\` in \`h\` / x.len() is 3`. With `hb([1, 2, 3, 4])` instead (p10's literal length) the same `--asserts` text, rightly: that twin must stay proved.

Cause (read): `count_checked_calls` (`prover_passes.nika` :185) counts `walk.reaches` by callee and looks the claim up as `checked[function]` (`ClaimHeld::Precondition(function, _)`, the `assert`'s function, `h`). `h`'s only call is carried to `hb`'s entry (`carried_here`, `prover_state.nika` :666, `continue`s without a reach), and the unproved call `hb(u)` is counted under `hb`. The carried claims are in `walk.state.preconditions[owner].claims`, each a `PreClaim` with `origin` (the `assert`'s function, or the callee it came through) and `site: CarriedFrom { at, callee, place }` (`prover_claims.nika` :12, :51). Build the map "function -> functions its preconditions were carried to" from those, and sum over its closure. Confirm the cause on a build before writing the fix; if it differs, fix what you find and say so in `Body:`.

## Locations (`scripts/where.nika`, 2026-10-31)

* `crates/nikaia-std/src/tools/prover_passes.nika` :185 `count_checked_calls` (the fix), :68 its call.
* `crates/nikaia-std/src/tools/prover_claims.nika` :12 `PreClaim`, :43 `carried_from`, :51 `CarriedFrom` (read; add a helper only if needed).
* `crates/nikaia-std/src/tools/prover_state.nika` :590-745 `precondition_call`, :752 `carry_back` (read only).
* Re-lower each changed `.nika` (`lower-std`, builder-facts), commit its `lowered/*.rs`. None of these files is in another lane's current batch.

## Steps

1. The fix. Tests: `tests/compiler/tests/precondition_carried_unproved.nika` + `.stderr` (the probe: refused, NK1300, the message as it prints) and `tests/compiler/tests/precondition_carried_proved.nika` + `.stdout` (`hb([1, 2, 3, 4])` and a two-step chain `hc(x) { return hb(x) }` called with `let z = [0 as u8; 8]`; both probed today, they print `4 0`, both proved: the program runs and prints). Run the first on the unfixed binary (`/home/user/nikaia-suite/target/fast/nikaia`, the suite worktree at 76c216e, when no suite is running there; else a scratch clone, its `target/` deleted after) and say under `Tests:` that it was accepted there.
2. Keep passing: every `tests/compiler` program whose source holds `assert(` (by name, `--both-settings`), `cargo test -p nikaia --test it -- assert prove claim precondition`. A test that now rightly reports an unproved carried precondition: re-bless and name it in `Body:` if it only changes the `--asserts` text; one that is newly **refused** and is a true unproved call: give its program a length the prover reads (as p10 did), never `unproved-claims = "warn"`, and name it in `Body:`; one that is not a true unproved call: stop, `partial`, the list.

Changelog: write one sentence (it changes what is refused), not `none`.

## Done when

The tests above pass; build, fmt, clippy, `pre-push-check.sh`, `lower-std` fixpoint; committed locally with `Fixes #1090`. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if

The fix needs a file outside *Locations*, their lowered twins and the tests; an existing test that is not a true unproved call becomes refused; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (`crates/nikaia-std/src/tools/lowered/project_members.rs` is residue, #1060), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p11-1090-carried-precondition-count.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
