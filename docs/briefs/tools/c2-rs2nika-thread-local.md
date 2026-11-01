---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: the converter turns a call to a thread-local accessor into a host function parameter (Part #125)

Goal: Rust code that reaches thread-local state through an accessor (`crate::proofs::ask`, `in_function`, `record_bound_invariants`, `invariants_here`, the `WARNED`/`SHOWN_AS` cells of `project.rs`) converts to a draft that already has the Nikaia shape: the accessor becomes a function parameter of the converted function (`ask: fn(ref TermArena, ref Vec[i64], i64) -> SolverAnswer sync`, as `prove_rounds` in `prove_run.nika` takes it), and the Rust forward passes the accessor. Nikaia has no thread-local or global mutable state by design (ADR-281 D32, ADR-289); passing it explicitly is the form those ADRs give. Today the draft calls `proofs::ask(..)` as a free function and the hand edit rebuilds the signature (`model_of` in `prove.rs` kept 3 of 6 lines for this reason).

## To build
1. **The rule** in `crates/rs2nika/convert`: an accessor is a Rust function whose body only reaches a `thread_local!` static (`X.with(|t| ..)`, also the save-call-restore form of `in_function`). Recognise it from the facts or declare it in `rs2nika.names` (your choice; a declared row is enough where recognising costs more). A converted function that calls one gains a parameter of its function type, named after the accessor, threaded on to the converted callees that need it; the call site in the draft calls the parameter. A case in `crates/rs2nika/convert/tests` for each form (plain accessor, scoped accessor with a closure, a `Cell` read). Commit.
2. **The proof on a part**: `crates/nikaia/src/fold_bound.rs`'s `proven_counters` (small: `proofs::in_function` around a call, `proofs::ask` in the `FbHost` impl), then `bounds.rs`'s `proven` (`in_function`, `ask`, `record_bound_invariants` twice each). Convert, score the draft (`--score`) before and after the rule, move the part into its `tools/*.nika` (`fold_bound.nika`, `bounds_*.nika`) where the share makes it cheap, with the Rust forward passing the accessors. A part whose move teaches the converter nothing more may stay Rust after scoring; say so in `Body:`. Commit the move separately.

## Facts
* Accessors: `crates/nikaia/src/proofs.rs` :80-190, `prove.rs` :73-191 (counters), `project.rs` :1781, `comptime_run.rs` :561-657. The answering logic behind `proofs::ask` is already Nikaia (`proofs_answers.nika`).
* Shape of a host function parameter: `prove_rounds` in `crates/nikaia-std/src/tools/prove_run.nika`, its Rust caller `prove.rs` :200-210.
* Known defect: a local bound from a `sync` function field loses `sync` (#1118); call the field directly (`b.run(1)`).
* After `lower-std`, only the lowered files of your own `tools/*.nika` and `lowered.rs` differ from `HEAD`; build `target/fast/nikaia` from `HEAD` before the first `lower-std`.
* Tests: the converter's own (`crates/rs2nika/convert`, `--corpus`); for a move `cargo test -p nikaia --test it -- bound fold overflow proof` and `nikaia test --project tests/compiler --both-settings` on the tests whose names contain `bound`, `fold`, `overflow`; the `nikaia.proofs` files stay byte for byte (`git diff --stat tests/compiler/nikaia.*`).

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/c2-rs2nika-thread-local.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
