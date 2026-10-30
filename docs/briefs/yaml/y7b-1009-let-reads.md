---
agent: builder
model: opus
max_turns: 120
budget_usd: 32
---
# Brief: #1009 and three neighbours: what json's `\u` path needs from the prover (ADR-361 D9, Part I *mut parameters*)

Score: #928 step 2c blocks on #1003 step 3, which blocks on these prover defects (json's `scalar::from(cp)` refused NK1300). Each is a defect (specified or plainly meant, not working), no owner decision.

**Read first:** `docs/builder-facts.md` (also its new *Red on develop now*), ADR-361 D9 (the join after an `if`), ADR-359 D1-D3 (result summaries; D3: a function promises its result with an `assert` before `return`), Part I's paragraph on `mut` parameters (`docs/specification/10-nikaia-light.md`, "A parameter the function changes in place says `mut`": only a `mut` parameter changes the caller's value).

## The end-to-end probe (architect, on develop 1e179d9c: refused NK1300 "an argument is not a whole number the prover reads"; must be proved after this brief)

```nika
enum Bad { No }
impl Error for Bad {
    fn message(ref self) -> String {
        return "bad"
    }
}

fn four(x: i64) -> i64 throws {
    if x < 0 || x > 0xFFFF {
        throw Bad::No
    }
    assert(x >= 0 && x <= 0xFFFF)
    return x
}

fn decode(a: i64, b: i64) -> scalar throws {
    let mut cp = four(a)
    if cp >= 55296 && cp <= 56319 {
        let low = four(b)
        if low < 56320 || low > 57343 {
            throw Bad::No
        }
        cp = 65536 + (cp - 55296) * 1024 + (low - 56320)
    } else if cp >= 56320 && cp <= 57343 {
        throw Bad::No
    }
    return scalar::from(cp)
}

fn main() {
    let c = decode(65, 0) catch { 'x' }
    println(f"{c}")
}
```

It is json's `string` (`packages/json/src/main.nika` 168-196) with `hex4` replaced by `four`. Narrowed by the architect into four defects, each with its own probe (all run on 1e179d9c):

1. **A call's precondition does not read the join's history.** `let first = four(a); let mut cp = first; if cp >= 55296 && cp <= 56319 { cp = 65536 } else if cp >= 56320 && cp <= 57343 { throw Bad::No }; let fin = cp; return scalar::from(fin)`: refused "nothing before the call shows it", while `assert(fin >= 0 && fin <= 0x10FFFF && (fin < 0xD800 || fin > 0xDFFF))` in the same place is proved, and with that assert before it the call is proved too. `an_assert` (`crates/nikaia-std/src/tools/prover_state.nika:356-372`) asks once more with `scope.with_history()` (ADR-361 D9, D2); `precondition_call` (same file, 572) does not. Fix: the precondition is asked the same way.
2. **A `let mut` whole number read by any call or method is dropped**, even for a claim before the call: `let mut cp = first; if cp > 5 { cp = cp + 1 }; assert(cp >= 0 && cp <= 0x10000); let s = id(cp)` (with `fn id(n: i64) -> i64`) or `let s = cp.to_string()`: the assert is refused "`cp` is not a whole number the prover reads"; without the later call it is proved. Cause: `untracked_in` (`crates/nikaia-std/src/tools/bounds_body.nika:95`, `Ask::Untracked` at 352-361) counts every variable handed to a `Call`/`MethodCall`/`SafeMethod` as changed; its only caller is `prover_block` (`prover_walk.nika:321`). Fix: a whole number handed to a parameter that is not `mut`, or as the receiver of a method on a whole number, is not changed (Part I); a `mut` parameter, a callee the walk cannot resolve, and a deferred body stay as today. Keep `bounds_body`'s other users (`bounds`, loops) unchanged in what they decide; if the signature lookup needs `program`, hand it in.
3. **`let mut cp = call()` is not read** (#1009's case): `let mut cp = small(a); if cp > 5 { cp = cp + 1 }; assert(cp >= 0 && cp <= 11)` (spike probe `h_1009_call`, `git show 09a7d79f:spike-probes/h_1009_call.nika`) refused "`cp` is not a whole number the prover reads", while `let first = small(a); let mut cp = first` (h2_first) is proved. `prover_let` (`prover_walk.nika:572-633`): the call branch (600-628, `prover_returns_whole`, the callee's `ensures`/summary) runs only for `!mutable`, and 632 returns for a mutable `let`. Fix: a mutable `let` of a call gets the same facts.
4. **A `let` of a call inside a branch loses its facts at the join**: spike probe `h3_branch_call` (`let low = small(b)` inside the `if`, then `cp = cp + low`; `assert(cp >= 0 && cp <= 20)`) refused "nothing before it shows it". `prover_join` (`prover_walk.nika:1350-1430`, `prover_side`, `prover_own_names`): what a side made known over its own names must be kept as that side's facts in the disjunction, its own names renamed apart (ADR-361 D9: "each side that goes on past the `if` adds what it made known over the names that live past it": a branch-local name is not one of those, but what it bounds of `cp` is).

Out of this brief (do not touch): the untyped `let d = ds[k]` read (spike `b_acc_ret`, `c_acc_for`; json does not need it), a result summary through a loop (ADR-359 D1 excludes loops by design; json's `hex4` promises its range with an `assert`, ADR-359 D3, in y4c), `packages/json/` (y4c's).

## To build (each step a commit; order 2, 3, 1, 4 is fine if it is cheaper)

1. Defect 1, test `tests/compiler/tests/prove_a_precondition_reads_what_an_if_made_known.nika` (+ `.stdout`): the `fin` probe above with `cp = 65536`.
2. Defect 2, tests `prove_a_let_mut_handed_to_a_call_stays_read.nika` (the `id(cp)` and `cp.to_string()` cases, `.stdout`) and `prove_a_let_mut_given_to_a_mut_parameter_is_not_read.nika` + `.stderr` (stays NK1300; the program, checked by the architect: `fn bump(mut n: i64) { n += 1000 }`, `fn pick(a: i64) -> i64 { if a < 0 || a > 10 { return 0 }; let mut cp = a; bump(cp); assert(cp >= 0 && cp <= 10); return cp }`, `main` prints `pick(3)`; one statement a line).
3. Defect 3, test `prove_a_let_mut_of_a_call.nika` (from h_1009_call).
4. Defect 4, test `prove_a_branch_let_of_a_call_reaches_the_join.nika` (from h3_branch_call), and the end-to-end probe above as `prove_a_surrogate_pair_decodes_to_a_scalar.nika` (+ `.stdout`, prints `A`). The last commit says `Fixes #1009` and `Part of #1003`.

Each new test fails on develop's compiler before your change (say so under `Tests:`). Probe each fix also at its edges: an `else`-less `if`, a `match` instead of `if`, a `throw` in the branch, and the end-to-end probe's body inside a `while true { .. }` loop that pushes the scalar onto a `String` and `continue`s (json's `string` has that shape); if the loop shape stays refused, add it as a test when you can fix it here, else name it under `Found:`.

## Gate and facts

* `lower-std` after each `tools/*.nika` change; the gate of `.claude/agents/builder.md` (#852), plus `cargo test --profile fast -p nikaia --test it -- prove unproved_claims bounds`, and `target/fast/nikaia test --both-settings --project tests/compiler` once at the end. The `sequences` guard is red on develop (18/16, *Red on develop now*): your change must not raise it; give its figure under `Tests:`.
* Declare new or changed prover functions `sync` (#1014: the lowering cache misses an inferred `sync`; y5 lost 8 turns).
* A pure call with literal arguments is run at build time and proved (ADR-359 D11): keep the probes' inputs behind parameters, as above.
* `nikaia check` inside `tests/compiler` checks the whole project: probe in a scratch copy. `0..x` fails `range_spelling` (`0..<(x + 1)`). No switch prints why the solver said *unknown*; a temporary `eprintln!` in a lowered `.rs` is restored by name.
* `Changelog:` one sentence (a `crates/` change); the version raise is the supervisor's.

Files this brief may touch (beyond its report): `crates/nikaia-std/src/tools/prover_*.nika`, `crates/nikaia-std/src/tools/bounds_body.nika`, their lowered files and `lowered.rs`, `tests/compiler/tests/prove_*`; Part III 14.2 only if a sentence there contradicts the fix.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y7b-1009-let-reads.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
