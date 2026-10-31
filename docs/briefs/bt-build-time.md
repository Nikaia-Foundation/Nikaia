---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: BT - the build-time folder's refusal, the run check and the default read move into Nikaia (Part #125)

Score: self-hosting lane, bootstrap week lane 1, row BT of `docs/selfhosting-plan.md` (next in the plan's order after K3a, which is built). No `#922`/`#920` entry needed.

Facts: Moving Rust into Nikaia, Where things live, Traps already met, Commands, exactly

Cut: crates/nikaia/src/build_time.rs:1-387
Cut: crates/nikaia/src/comptime_run.rs:602-722
Convert: --profile check-free --prefix bt_
Shape: faithful

Line numbers at 042cee5d; re-find `enum Refusal`, `fn may_not_run`, `thread_local!` (the `DEFAULTS` one), `const STOPPED_AT` if they moved, and cut the same items. The draft converts 534 Rust lines to 313 with 5 TODOs (a type alias, two `thread_local!`, two `mem::replace`): those five are exactly what stays Rust (below), so do not port them.

## What moves (into `crates/nikaia-std/src/tools/build_eval.nika`, prefix `bt_`)
1. **`Refusal` becomes `BtRefusal`'s, and the slot indirection goes.** `BtRefusal::Host(i64)` is replaced by `GrammarWall { owner: String, rule: String, why: CheckWall }` (`CheckWall` is already the Nikaia twin of Rust's `grammar_run::Wall`, `check_reports.nika:15`; the field names follow `CheckBuilt::GrammarWall`, `check_host.nika:28`). `BtRan.slot` becomes `wall: CheckWall?` (null where a value came back or nothing was found), and the evaluator throws `GrammarWall` from it where it threw `Host(slot)` (find the throw with `nikaia run scripts/where.nika -- BtRefusal`). Its `message` arm is `` f"the grammar `{owner}` stopped at `{rule}`" `` (no test prints it).
2. **`grammar_run::Wall` becomes `pub use nikaia_std::tools::check_reports::CheckWall as Wall;`** in `grammar_run.rs` (the enum at :43-64 goes). Its one renamed field: `InsideAnother { grammar }` is `InsideAnother { outer }` (`grammar` is a reserved word in Nikaia): `grammar_run.rs:172`, `comptime_run.rs:500`, `check/host.rs:172`. `check/host.rs:162` `a_wall` is then the identity: delete it and hand `why` on (`a_wall` callers: `where.nika -- a_wall`).
3. **`build_time::Refusal` becomes `pub use nikaia_std::tools::build_eval::BtRefusal as Refusal;`.** `Host::refusal` and `Host.kept` (the `RefCell<Vec<Refusal>>`) go; `BuildTime::run` returns `bt_evaluate(...)`'s thrown value as it is. `OutOfBounds.len` is `i64` now: `check/host.rs:233` drops its cast. `run_grammar` (the `BtHost` impl) fills `wall` instead of pushing into `kept`.
4. **`may_not_run` and `is_the_builds_own`** become `pub fn bt_may_not_run(contract: ref FnContract) -> String? sync` (and `bt_is_the_builds_own`) over the Nikaia `FnContract`/`Touch` (`ty.nika:1075`, `:780`); the three texts byte for byte. Rust `may_not_run` stays as a one-line forward; its result becomes `Option<String>` (callers: `check/host.rs:66`, `comptime_run.rs`, 3 calls; adjust them to `.as_deref()` or a `String`, nothing else).
5. **`known_default`'s body** becomes `pub fn bt_known_default(ledger: ref Ledger, package: String?, key: ref String, at: i64) -> String? sync` (`Ledger` is `ty.nika:1233`); the Rust `known_default` keeps `KNOWN` and calls it inside `KNOWN.with` (one forward; caller `contracts/mod.rs:1217` unchanged).

## What stays Rust (and why; say so in the commit body)
`BuildTime` and the `BtHost` impl's reads of `Parsed` (`text`, `known`, `constant`, `variant_of`, `rule_of`, `run_grammar`, `read`: the `Parsed` arena is Rust, plan *Stays Rust*); the `Known` type alias; the escape forwards and `char_of`; `DEFAULTS`/`KNOWN` `thread_local`s with `defaults_compiled_in`, `defaults_known_from`, `defaults_workshop`; `with_driver` (reads `emit::Lowered`, Rust) and `Bounds` (read by `grammar_run.rs`'s `Workshop`, which GR moves next).

## Forms to watch (CLAUDE.md, *A missing form stops the work*)
* An enum of another tools file used as a field type (`CheckWall` inside `BtRefusal`): tools share one namespace; if `lower-std` refuses it or orders the modules wrongly, stop: `Found:` with the probe.
* Check the lowered file right after the first `lower-std`, before touching Rust: `cargo check --profile fast -p nikaia-std --features compiler --message-format short` (without `--features compiler` it checks nothing). A rustc error in `lowered/build_eval.rs` is a lowering defect: stop, `Found:` with the lowered line and a probe.

## Locations
* Nikaia: `build_eval.nika` (`BtRan` :31, `BtHost` :46, `BtRefusal` :61 and its `message`, `bt_evaluate` :108, the grammar-run arm: `where.nika -- run_grammar`); `check_reports.nika:15` `CheckWall`; `check_host.nika:19` `CheckBuilt`; `ty.nika` `FnContract`, `Touch`, `Ledger`.
* Rust: `build_time.rs` (whole); `comptime_run.rs:602-722` (defaults), :500 (`InsideAnother`); `grammar_run.rs:43-64` (`Wall`), :172; `check/host.rs:66`, :162-181 (`a_wall`), :226-256 (`evaluate`, the `Refusal` match); `contracts/mod.rs:1217`, :1267-1298.
* Tests that read these: `crates/nikaia/tests/it/` `build_time.rs`, `build_time_dependencies.rs`, `comptime*.rs`, `grammar_at_build_time.rs`, `dsl_grammar.rs`; `nikaia run scripts/where.nika -- Refusal Wall may_not_run known_default` for the rest.

## To build
1. Read the draft and the cut; `lower-std` after the Nikaia steps 1, 4, 5; the fixpoint changes only `lowered/build_eval.rs` (and `lowered/check_reports.rs` only if you had to touch it) and their index lines in `lowered.rs`.
2. Rust steps 2, 3, 4, 5; delete the moved Rust bodies. `cargo build --profile fast -p nikaia`.
3. Tests: the `it` filters `build_time comptime grammar_at_build_time dsl_grammar` singly, the guard set of `.claude/agents/builder.md`, and `tests/compiler` through the gate (`scripts/gate.nika`; the checker changed, so the whole package runs). No new test for a faithful move; no text a test compares may change.
4. Row BT of `docs/selfhosting-plan.md`: built, what stays Rust, the share as `scripts/self_hosting.py` counts it. Commit (`Part: #125`), push.

## Done when
`Refusal`, `Wall`, `may_not_run`'s rule and `known_default`'s read are Nikaia; the Rust left in the cut is the adapter list above; the tests above pass; pushed. Changelog: none (a move). Commit subject: `build time: the folder's refusal, the run check and the default read move into Nikaia (BT)`. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
A text a test compares would change; a missing form (above); an edit needed outside `build_time.rs`, `comptime_run.rs`, `grammar_run.rs`, `check/host.rs`, `contracts/mod.rs` (a call's type only), `build_eval.nika`, `check_reports.nika` (only if `lower-std` needs it), their lowered twins and `lowered.rs`'s index lines, `rs2nika.names` (`bt_` rows), the plan row; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (today among them `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}`, `crates/nikaia-std/src/tools/text_tiers.nika`, and `docs/briefs/*.report.md` files), and every `docs/briefs/*` file except your report. If `lowered.rs` shows another session's hunk, stage only yours (`git diff ... > p; edit p; git apply --cached p`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bt-build-time.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
