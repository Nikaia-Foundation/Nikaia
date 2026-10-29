---
agent: builder
model: opus
max_turns: 90
budget_usd: 20
---
# Brief: P5a rest, `program_name`, `root_tag`, `own_copy`, `trace` of `project.rs` into `project_settings.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`, P5 `project.rs` build drivers). The rest of plan row P5a (`docs/selfhosting-plan.md`, *Large files 3*), left by 2e13458 on two defects: #934 (fixed b1759a1) and #933 (brief `b933-method-result-owned.md`, the brief before this one in the batch). Small move (~70 lines), S=1.

Cut: crates/nikaia/src/project.rs:2740-2779
Cut: crates/nikaia/src/project.rs:3365-3391
Convert: --profile check-free --prefix project_settings_
Shape: faithful (methods become `project_settings_` free functions over `root: ref fs::Path, variant: String?`, as P5a's `project_settings_own_dir` :398 already is; learned a53f456)

The driver converts these before you start and names the draft (`.draft.nika`, `.notes`, `.rs`): start from it. Ranges are at e9fda67 (re-grep `fn program_name`, `fn trace(` if `git log -- crates/nikaia/src/project.rs` shows a later change).

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read `crates/nikaia-std/src/tools` as `crates/nikaia-compiler/src`.

**Precondition:** #933 is on `develop` (34519e7, met). Its lowering of a method's owned text viewed and returned is ADR-353 D10's caller keep (`-> &'k str` with `__keep`), not `-> String`: take whatever the lowering gives that rustc accepts.

## Facts
* `Project` stays Rust (fields read 86 times); the Rust methods stay as one-line adapters with unchanged signatures, so callers (`project.rs` :2008, :2382, :2441, :2537) do not change.
* Host: add to `ProjectSettingsHost` (`tools/project_settings.nika` :249; Rust impl `SettingsHost`, `project.rs` :135) two methods, full signatures:
  `fn sha256_hex(ref self, text: ref String) -> String sync` (Rust: `orchestrator::cache::sha256_hex(text.as_bytes())`; `modules.nika` :905 has the same signature) and
  `fn hard_link(ref self, from: ref fs::Path, to: ref fs::Path) -> bool sync` (Rust: `std::fs::hard_link(from, to).is_ok()`). `env_var` is there (`CARGO_TARGET_DIR`, `TRACE_VAR` = `"NIKAIA_WRAPPER_TRACE"`, `project.rs` :103; make it a `comptime` beside the other `PROJECT_SETTINGS_*_VAR` :224-244 only if no other Rust reader of `TRACE_VAR` is left, else pass it in).
* `std` has what the rest needs (checked): `fs::create_dir`, `fs::remove`, `fs::copy`, `fs::write(path, data; append: true)` (Part III, `fs.rs` :820), `Path.file_name`/`join`, `String.replacen`, `find`. `trace` is silent on every failure: `catch` each, no `throws`.
* `trace(invocation: &Invocation, verdict: &str)`: `Invocation` stays Rust; the Nikaia function takes `crate_name: String?, is_probe: bool, verdict: String`, the Rust `trace` forwards (3 callers unchanged).
* `own_copy` returns `Result<PathBuf>` with `.with_context("copying A to B")`: keep that text byte for byte in the Nikaia `throw`; the Rust adapter turns the refusal into `anyhow` as `refused` (:126) does.
* Do not touch `crates/rs2nika/`, `emit/*`, `check/*`, `contracts/*`, nor the uncommitted files in the checkout (`emit_core_flow`, `emit_function`, `keeps`, `text_tiers` `.nika`/`.rs`, `crates/rs2nika/convert/nikaia.*`, `docs/briefs/k1-comptime-tree.*`): another session's.
* Tests: `it` filters `project cpu shared_target own_copy trace program` (whichever match; `nikaia run scripts/where.nika -- root_tag own_copy trace` lists them), `cargo test --profile fast -p nikaia --lib project::`.

## To build
1. From the draft, write the four functions into `tools/project_settings.nika` with doc comments, the two host methods, the Rust adapters; `lower-std` (only `project_settings.rs` changes, and the index if a name is new). Commit `project: program name, root tag, own copy and trace into Nikaia (P5a)`, `Part: #125`.
2. `docs/selfhosting-plan.md`: P5a built in full; self-hosting % (`python3 scripts/self_hosting.py`) under `Tests:`.

A lowering or checker defect met is fixed in the compiler as its own commit with a test; more than one step: stop, `Needs:`.

## Done when
The `it` filters plus the guard set of `.claude/agents/builder.md`, once at the end, beside clippy; `--lib project::`; lower-std fixpoint. Changelog: none (a move).

## Stop and report if
#933 not on develop; a message text that would change; an edit needed in a lane 2 file; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p5e-project-tag-copy.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
