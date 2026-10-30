---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #1012 step 1, a trait method's `keeps` column for `fn` parameters is computed in the fixpoint (ADR-277 D17)

Score: self-hosting lane; the build of an owner decision (#1012, option E, 2026-10-30) that unblocks K3a part 2 (`k3a2-comptime-compute.md`, next in the batch). Today's rule (by value means kept) forces `Arc` and `'static` captures on every callback through a trait. Step 1 of the *To build* list of #1012; steps 2 and 3 are `b1012b-trait-column-refusals.md` and `b1012c-trait-column-data.md`. Precondition: ADR-277 D17 and its Part I 6.5 / Part III text are on `origin/develop` (read D17 first).

## Problem

`crates/nikaia/src/contracts/keeps.rs` `infer` gives a body-less trait method `keeps` for every parameter it takes by value (the `Item::Trait` arm, ~220-237), and the `trait_keeps` pre-pass (~144-171, 0.0.1312) copies that column onto every impl. D17: the column is the union, by position, of the default body's `keeps` and those of every impl of the trait in its own package (private impls included, impls under `tests/` not), computed inside the least fixpoint `kept_by` (`crates/nikaia-std/src/tools/lends.nika`); with neither it is empty (run). This step does it for parameters of `fn` type. Data parameters keep today's by-value seed until step 3 (`b1012c`), so the hand-written Rust impls of the lowered host traits do not move yet.

## Probe (`nikaia run`, 6929d69b)

```nika
use std::collections
trait H {
    fn ask(ref self, known: fn(ref String) -> i64? sync) -> i64? sync
}
struct Plain { base: i64 }
impl H for Plain {
    fn ask(ref self, known: fn(ref String) -> i64? sync) -> i64? sync { return known("a") }
}
fn twice[T: H](host: ref T, names: ref collections::BTreeMap[String, i64]) -> i64 {
    return host.ask(fn(n) { names.get(n) }) ?? 0
}
fn main() {
    let mut names: collections::BTreeMap[String, i64] = collections::BTreeMap()
    names.insert("a", 4)
    let p = Plain { base: 1 }
    println(twice(p, names))
    println(names.len())
}
```
Today: "Lifetime may not live long enough" at the lambda (the lowered `ask` takes `Kept<dyn Fn>`). Expected: `4`, `1` (the free-function control prints them).

## Locations
* `crates/nikaia/src/contracts/keeps.rs` `infer`: the trait method becomes a node "H::m" of the graph. Its own evidence is its default body's (else empty). Edge pairs (`Passed { parameter, callee, at }`), per `fn` parameter position: `H::m -> X::m` for each impl in the package ("the trait keeps what an impl keeps") and `X::m -> H::m` back (the 0.0.1312 rule, moved inside the loop). Remove the `trait_keeps` pre-pass and the by-value arm for `fn` parameters; the impl-side union (~199-215) is then the back edge. The node and its impls are one SCC, so they agree by construction; the least fixpoint keeps a storage-free cycle run.
* An impl of a trait from another package: its entry takes the library column (`keeps_its` reads `library.functions["H::m"]`). The comparison and refusal are step 2's.
* `keeps_its` falls back to **kept** for a callee absent from `own` (fail closed): every trait method, `pub` or not, needs its own entry, or a private trait reads kept.
* `kept_args` (`check_core_args.nika`) and the emitter (`emit_function.nika` ~565) read the column and need no change of their own; if one does, say so in `Body:`.

## To build
1. The graph change above. Tests in `tests/compiler/tests/` (+ `.stdout`), each also run against the unfixed compiler:
   * (a) the probe: `4`, `1`.
   * (b) one impl that stores (`Keeps { held: Vec[fn(i64) -> i64 sync] }` pushes) and one that runs, through `fn hand[T: H](mut h: T, n: i64) -> i64 { return h.take(fn(x) { x + n }) }`: prints today and after; the column is kept, both impls lower `Kept`. Call the held ones with `for f in k.held { println(f(10)) }` (`let first = k.held[0]; first(10)` hits a separate defect, not yours).
   * (c) `a_trait_method_callback_is_handed_on.nika` still prints `6`, `3`, `9`.
   * (d) a trait with no default body and no impl, and a generic caller lending a lambda that borrows a local map: runs (the column is empty, lowers `impl Fn`).
   * (e) two impls handing the callback to each other through the bound (`relay[T: H]`), no storage, called with a borrowing lambda: stays run, prints.
   * (f) and (i) need the toolchain: write them in the pattern of `tests/compiler/tests/commit_dry_run.nika` (`toolchain()`, files into `mktemp -d`, `process::run`). (f): one program and the same with its items reversed give an identical `nikaia.contracts` through `nikaia lower`; print `same`. (i): a package whose `src` has a `pub trait H` and no impl, and whose `tests/` file `use`s it with an impl that only runs: `nikaia test --project` passes (today rustc E0049: the library trait lowers `Kept`, the test impl `impl Fn`), and the package's ledger shows no `keeps` for `H::ask`.
   * `lower-std` fixpoint clean (`docs/builder-facts.md`): only files whose lowering moves. Commit `contracts: a trait method keeps what its package's impls keep (#1012)`, `Part of #1012`.

## Done when
(a)-(f) and (i) pass through `nikaia test` (copy a file to the scratchpad to `nikaia run` it alone), the `it` filter `contracts` and the guard set of `.claude/agents/builder.md` are green, the fixpoint is clean; pushed. Changelog: one line (a callback through a trait method borrows its captures when no impl in the trait's package keeps it).

## Stop and report if
An edit outside `crates/nikaia/src/contracts/keeps.rs`, `crates/nikaia-std/src/tools/*.nika` and their lowering (`lowered/*.rs`, `lowered.rs` index lines), and the new tests; a hand-written Rust impl of a lowered host trait has to change (that is step 3's: `Found:` with the trait and the file); an error in another agent's file; the trait's impls of the package are not reachable from `infer` (`Found:` with where they are).

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1012a-trait-column-fn.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
