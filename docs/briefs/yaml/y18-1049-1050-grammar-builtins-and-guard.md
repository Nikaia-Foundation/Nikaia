---
agent: builder
model: opus
max_turns: 110
budget_usd: 25
---
# Brief: #1049 (a sequence inside `peek(..)`/`not(..)` fails at run) and #1050 (a guard handing an optional label to a by-value `Node?` parameter reaches rustc)

Score: two defects y2b found in the `yaml` lane (c8d95984): `nikaia check` passes, the run does not. Nothing in `packages/yaml` needs them now (it uses a named rule `peek(Dash)` and `t: ref Node?`, both sound as written); they touch only the grammar checker, emitter and lowering, so this lane builds them before #1000 (`y3`). Two issues, two commits: #1049 first (small, probed), then #1050 (needs a reduction first). Read `docs/builder-facts.md`; y2b's report `docs/briefs/yaml/y2b-keys-directives-limits.md.report.md`, its `Found:` (2) and (3).

## Issue #1049 (copied, without frontmatter)

Part II 10 (`docs/specification/20-nikaia-advance.md`, the element table): `not(p)` and `peek(p)` take a pattern `p`, and `p q` is a pattern. The checker accepts a sequence there; the run refuses it.

```nika
grammar G {
    entry rule doc -> String = peek("-" "a") s:until(";") ";" -> { s }
}

fn main() {
    let r = G::doc("-a;") catch {
        println("error")
        return
    }
    println(r)
}
```

Today (develop 5817b28d, probed by the architect): `nikaia check` prints `Checked main.nika`; `nikaia run` stops with `Error: Unexpected token, expected `)`.` at the second element inside `peek(..)`; the same with `not("+" "a")`. `("-" "a")?` and `text("-" "a")` run. Wanted: `-a` for both. (A syntactic rule skips whitespace before each element, so a probe with a `" "` element after another inside a lowercase rule fails for that reason, not this one: keep the probe's elements without spaces.)

To build: (1) find which parser refuses it at run (`nikaia run` parses the grammar a second time somewhere the checker does not, or the emitter writes the built-in's argument as one element) and make it take a sequence inside `peek(..)` and `not(..)`, as `text(..)` does; try every other one-pattern built-in of the table (`until`, `bytes`, `span`, `count`, `dec[T]`, `intern`) the same way and fix those that refuse it; (2) test `tests/compiler/tests/grammar_a_builtin_takes_a_sequence.nika` + `.stdout`, failing on the unfixed binary. `Fixes #1049`.

## Issue #1050 (copied, without frontmatter)

`packages/yaml/src/main.nika:567-568`:

```nika
| k:FlowAny(ds, left) t:FlowTail(ds, left - 1)?
if(pair_fits(left, k, t), "nesting within the depth limit") -> { .. match t { .. } }
```

with `fn pair_fits(left: i64, k: ref Node, t: ref Node?) -> bool sync` (`main.nika:992`) compiles. With `t: Node?`, `nikaia check` passes and rustc refuses the lowered guard closure: "closure only implements FnOnce". A 20-line probe with a small struct did not reproduce it. The guard reads `t` by value and the action reads `t` again afterwards: a guard must lend what it reads, or the checker must refuse the move with an existing NK code; a rustc error is a defect either way.

To build: (1) reproduce: copy `packages/yaml` to a scratch directory (outside the checkout), change line 992 to `t: Node?`, `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project .`; read the generated Rust under the run's `target/nikaia/gen/`; shrink (drop rules the failing rule does not call, then fields of `Node`) until a probe of a few dozen lines fails; name what makes it fail (a non-`Copy` field, the optional label, the label read again in the action, the `sync` marker). (2) Fix it where the guard's call is checked or lowered (`check_grammar.nika`, `emit_pattern.nika`'s guard closure, `crates/nikaia/src/contracts/keeps.rs` if the guard's call site counts as keeping, see 7a91e7d9 for the entry's case), so a by-value parameter fed from a label in a guard is treated as an ordinary call's argument that is read again afterwards. (3) Test `tests/compiler/tests/grammar_a_guard_hands_an_optional_label_by_value.nika` + `.stdout` (or `.stderr` if the checker rightly refuses), failing on the unfixed binary. `packages/yaml` keeps `ref Node?`. `Fixes #1050`.

If the reduction does not reproduce in 25 turns, commit #1049, leave #1050 open with a comment saying what was tried (`gh api repos/nikaia-foundation/nikaia/issues/1050/comments`), and report `partial` with `Needs: #1050 reduction`.

## Done when

Both tests pass and fail on the unfixed binary (say so under `Tests:`); `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --both-settings --project tests/compiler > out.txt 2>&1` green with every `grammar_` test in it; the package gate below green; each fix its own commit with its `Fixes #N`, pushed. Files you may change: `crates/nikaia-std/src/tools/check_grammar.nika`, `emit_pattern.nika`, `emit_grammar.nika`, the grammar parser the run uses (name it under `Body:`), their lowered twins (`crates/nikaia-std/src/tools/lowered/*.rs`, regenerate as `docs/builder-facts.md` says), `crates/nikaia/src/contracts/keeps.rs`, the two new tests with their expected output. Name under `Changelog:` what each change decided.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y18-1049-1050-grammar-builtins-and-guard.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24

## Earlier attempt (supervisor, 2026-10-31)

The first builder on this brief was stopped mid-work when its driver hit a time limit. **#1049 (step 1) is built and committed** (0c9f96a4, gate green, report `y18-1049-1050-grammar-builtins-and-guard.md.step1.report.md`). Build #1050 only, and copy step 1's `Changelog:` line into your report beside your own. The unfinished #1050 work is saved outside the checkout: `/tmp/claude-0/-home-user-Nikaia/e431aae9-4983-5166-8583-f9b3eb3b6f61/scratchpad/y18-step2-partial/` (`tracked.diff` for `check_grammar`/`check_takes`, plus the started test `grammar_a_guard_hands_an_optional_label_by_value.*`). Read it, take over what holds up, and check every piece of it. It has not passed any gate.
