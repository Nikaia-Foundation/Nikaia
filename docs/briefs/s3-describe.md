---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 10
effort: medium
---
# Brief: S3, `describe`'s adopt check, digest and source walk into `describe.nika` (Part #125)

Score: self-hosting lane, no plan row (`describe.nika` holds the draft since 0.0.394; the manifest, `toml` and the parser stay Rust). About 160 lines of Rust move, by hand.

Cut: crates/nikaia/src/describe.rs:157-213
Cut: crates/nikaia/src/describe.rs:400-443
Cut: crates/nikaia/src/describe.rs:461-475
Cut: crates/nikaia/src/describe.rs:490-534
Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **What moves**, into `crates/nikaia-std/src/tools/describe.nika` (new functions `describe_`-prefixed, doc comments carried over): `adopt`'s check (:157-213: the `# crate:` header against the version, the empty `[sources]`, the files that moved and the three refusals' texts) as `pub fn describe_adopt_refusal(declared: String, crate_word: String, shipped: String, derived: String, text: String, version: String, recorded: ref collections::BTreeMap[String, String], files: ref collections::BTreeMap[String, String]) -> String?` (null: adopt; the hashes of `files` with `hash::sha256_hex`, Rust's `orchestrator::cache::sha256_hex`); `descriptions_digest` (:400-443) over `ref fs::Path` (pattern `project_members_proofs_digest`, `project_members.nika:542`; `fs::read_dir` answers in byte order, which is Rust's `files.sort()`); `without_dots` (:461-475) over a `String` split on `/`, a leading `/` kept, `..` on nothing dropped; `names_the_program_writes` (:490-534) with `pub trait DescribeHost { fn qualified_names(ref self, text: String) -> Vec[String] sync }`, Rust answering with `parse_to_ast` and `crate::foreign::qualified_names(..).keys()`, nothing for a file that does not parse. `crate::project::CONTRACTS` is the text `"contracts"` (as `project_members.nika:569`).
* **Probe** (supervisor, this round, `target/fast/nikaia run`, right output): `s3_digest.nika` (the digest of a `contracts/` with two `.contracts` and a `.txt`, equal to a reference sha256 of `name\0hex\0...`; `""` for no directory; `without_dots("/a/./b/../c/x")` is `/a/c/x`, of `a/../../b` is `b`; the moved files over the union of two maps' keys, `" and {n - 1} more file{if n == 2 { "" } else { "s" }}"`; `lines().find_map(fn(line) { line.strip_prefix("# crate: ") })` matched with `null =>`) and `s3_walk.nika` (a generic host, `pending.pop() ?? break`, `fs::metadata(..).is_dir()`, `read_to_string` with `catch { continue }`, `target` and `contracts` skipped). A path is `let p: fs::Path = "..."` (`fs::Path::from` / `fs::Path(..)` are refused); maps are `collections::BTreeMap()`; `use std::fs` and `use std::hash`. Trap: Rust hashes `read_to_string(..).unwrap_or_default()`, so a non-UTF-8 file hashes as `""`: use `fs::read_to_string` with `catch { "" }`, not `fs::read`. Defect met (#1144, fixed by bug brief b36 before this brief runs; write `rest.to_string()`, and if it still fails, stop): `out.insert(rest.to_string())` for `rest` from `strip_prefix` makes the set's element a view (`rustc`: "missing lifetime specifier").
* **Rust keeps** `adopt` but for the check (:157-213 become `let derived = ...;` and `if let Some(refusal) = describe_adopt_refusal(..) { bail!("{refusal}") }`), `describe`, `draft`, `rust_dependency`, `crate_sources`, `crate_root`, `cargo_version`, and forwards with their signatures: `pub fn descriptions_digest(root: &Path) -> String`, `fn without_dots(path: &Path) -> PathBuf`, `fn names_the_program_writes(root: &Path, crate_word: &str) -> Result<BTreeSet<String>>` (`block_on` as `project.rs:749`). Untouched callers: `descriptions_digest` at `project.rs:745`; `without_dots` at `describe.rs:141`, `:373`, `:457`; `names_the_program_writes` at `:222`, `:254`.
* **#1060 guard**: after `lower-std`, only `lowered/describe.rs` and `lowered.rs` are yours. `lowered/emit_core_leaves.rs` differs in the tree from another agent's uncommitted `emit_core_leaves.nika`: neither stage nor restore it.
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start.

## To build
1. The four moves, the host, the forwards, `lower-std` and its fixpoint; every other `lowered/*.rs` unchanged; the `describe.nika` row of `scripts/self_hosting.py` (:92) names what it holds now. Tests: `cargo test -p nikaia --lib describe`, `cargo test -p nikaia --test it -- describing:: described:: described_entries:: foreign_runtime:: self_hosting::` (anchored `grep -E '^test result:|FAILED|panicked'`). No plan row; self-hosting share in `Body:`. Commit `describe: the adopt check, the digest and the source walk move into describe.nika (S3)`, `Part: #125`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s3-describe.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
