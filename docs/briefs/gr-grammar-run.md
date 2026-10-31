---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: GR - a grammar run's build and run of its parser move into Nikaia (Part #125)

Score: self-hosting lane, row GR of `docs/selfhosting-plan.md`. It needs `hash::sha256_hex` and `env::var` (#922 steps 1-2, brief `s922-sha256-env.md`, the batch's step before this one) and BT's `Wall` = `CheckWall` (brief `bt-build-time.md`, the first step of the batch).

Facts: Moving Rust into Nikaia, Where things live, Traps already met, Commands

Cut: crates/nikaia/src/grammar_run.rs:185-262
Convert: --profile check-free --prefix grammar_run_
Shape: faithful

Line numbers at 042cee5d, before BT. BT deletes `Wall` (:43-64), so re-find `fn compile_and_run`, `fn cargo`, `fn write_if_changed` and cut those three items.

## Pre-flight (before any edit; stop if one fails)
1. `hash::sha256_hex` and `env::var` are on develop: `grep -n 'hash::sha256_hex\|env::var' crates/nikaia-std/std.contracts`. If absent: `Result: blocked`, `Needs:` s922 first.
2. `grammar_run::Wall` is a `pub use` of `CheckWall` (BT built). If not: `Result: blocked`, `Needs:` BT first.

## What moves (a new `crates/nikaia-std/src/tools/grammar_run.nika`, prefix `grammar_run_`)
1. **`compile_and_run`** becomes `pub fn grammar_run_compile_and_run(host: ref GrammarRunHost, at: ref fs::Path, input: ref String) -> String` throwing `CheckWall`. Follow how `build_eval.nika` throws `BtRefusal` and how `check_host.nika` names `CheckWall`. The steps are the Rust ones, in order:
   * `host.driver()`;
   * the key `hash::sha256_hex(program bytes)`;
   * `grammar_run_write_if_changed` for `src/main.rs`, then for `Cargo.toml` with `host.manifest(key, program)`;
   * `process::run(env::var("CARGO") ?? "cargo", ["build", "--quiet", "--manifest-path", …, "--target-dir", …], dir: host.where_tools_start())`. On a non-zero exit: `DidNotBuild { detail: stderr }`. A thrown `io::IoError` becomes `DidNotBuild { detail: f"running cargo: {e}" }`;
   * write `input`;
   * run `target/debug/p<key>` over it. On success: the trimmed stdout. Otherwise: `Refused { detail: stderr }`.
   The texts stay byte for byte (`creating …`, `writing …`, `running …`).
2. **`write_if_changed`** becomes `grammar_run_write_if_changed(path: ref fs::Path, text: ref String)` throwing `CheckWall`, over `fs::create_dir_all`, `fs::read_to_string`, `fs::write` with `fs::Root::Anywhere` (as `sysroot_cache.nika` uses them). `cargo()` is the one `env::var` line above, and goes.
3. **The host trait `GrammarRunHost`** has four methods. Check the name with `nikaia run scripts/where.nika -- GrammarRunHost`.
   * `driver(ref self) -> String`, throwing `CheckWall`. The Rust impl holds `parsed` and `ask` and calls `driver`, which stays Rust: `Parsed`, emit.
   * `manifest(ref self, key: ref String, program: ref String) -> String sync`. It calls `project::runtime_dependencies_for`, which is Rust.
   * `shared_target(ref self) -> fs::Path sync`. It uses `env!` and the orchestrator cache, and moves with O1.
   * `where_tools_start(ref self) -> fs::Path sync`.
4. **Rust `Workshop::run` keeps the `running` guard**: a `Mutex`, and the guard is lock glue. It calls the lowered function through `nikaia_std::rt::exec::block_on`, because `process::run` pauses (the pattern is in `modules.rs:88`). `compile_and_run`, `cargo` and `write_if_changed` are deleted from Rust.

## Stays Rust (say so in the commit body)
`Ask` (it borrows `Ty` and `Parsed`), `Workshop` and its `Mutex`/`OnceLock`, `run_comptime`, `driver`, `dumper`, `shapes` (`Ty::from_ast` has no Nikaia counterpart), `decode` (a forward already), `shared_target` and `manifest` (as host methods).

## Forms to watch (CLAUDE.md, *A missing form stops the work*)
* **This is the first tools file that starts a child process.** `block_on` is called from inside a `comptime` evaluation (`BtHost::run_grammar` → `Workshop::run`). If it panics there ("cannot block inside a runtime" or the like), or `process::run` refuses a tools caller, stop: `Found:` with the probe, a `comptime` with a grammar, as `tests/compiler` has one (`where.nika -- grammar_at_build_time`).
* Check the lowered file right after the first `lower-std`, before touching Rust: `cargo check --profile fast -p nikaia-std --features compiler --message-format short`. A rustc error in `lowered/grammar_run.rs` is a lowering defect: stop, `Found:` with the lowered line.

## Tests
The `it` filters `grammar_at_build_time dsl_grammar build_time comptime`, plus the guard set of `.claude/agents/builder.md`. Then the gate on the whole `tests/compiler` package, and the lower-std fixpoint: a new `lowered/grammar_run.rs` and its index line in `lowered.rs`, nothing else. No new test: the paths are the same as before. A text no existing test prints is named under `Tests:`, since a failing cargo cannot be staged offline.

## To build
1. Convert, read the draft against the cut, write `grammar_run.nika`, `lower-std`, check the lowered file.
2. Rust: the host impl, `Workshop::run` calls the lowered function, delete the three items. Update GR's row in `docs/selfhosting-plan.md`: built, what is left, and that `shared_target` and `manifest` move with O1.
3. Commit `grammar run: the parser's build and run move into Nikaia (GR)`, `Part: #125`.

## Not yours
`crates/rs2nika/convert/nikaia.{contracts,derived,proofs}`, `crates/nikaia-std/src/tools/text_tiers.nika`, the other briefs and reports in `docs/briefs/`, and any file `git status` shows modified at your start.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/gr-grammar-run.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
