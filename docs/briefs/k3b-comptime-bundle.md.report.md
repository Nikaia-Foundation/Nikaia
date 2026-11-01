Result: done
Files: crates/nikaia-std/src/tools/comptime_bundle.nika crates/nikaia-std/src/tools/lowered/comptime_bundle.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/comptime_run.rs crates/nikaia-std/src/lib.rs scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: comptime: the bundle and the run move into Nikaia (K3b)
Body: This moves `run_bundle`, `run_fingerprint`, `read_record`, `bundle_text`, `bundle_dir`, `stamp_of`, `bundle_of`, `run`, `write_if_changed`, `Flavour`, `RUN_SOURCE`, `RUN_PROFILE` and `NAMED` into the new `comptime_bundle.nika`, behind `ComptimeBundleHost`. It also adds the Nikaia `ComptimeBundle`, and one Rust adapter makes the Rust `Bundle` from it. This is run 2's move (707d5e53), cherry-picked and re-lowered on top of the #1106 fix (1a5fdf6d); the lowered file was not hand-patched. Stays Rust: `Bundle`, `build_for_cpu`, `CPU_LEVEL`, `NOT_BUILT`, `waited` and the child's spawn (`run_child`), the `serde_json` loop (`artifacts`), `unique_beside`'s counter, and `rustc`, `toolchain`, `target_libdir` with their `OnceLock`s (`run_fast.rs` calls `rustc`). `bundle`, `run_bundle`, `run` and `write_if_changed` keep their signatures and now forward through `block_on`. The dead `cargo()` is gone. GR's `impl Error for CheckWall` (grammar_run.nika) is used, so this file has no copy of it. Three behaviour changes: `{path}: {e}` now carries an `io::IoError`'s text; `process::run` throws on output that is not UTF-8, where Rust read it lossily; `run_fingerprint`'s `fs::metadata` follows a link where `DirEntry::metadata` did not (there is no link under `std/`). The lowered walk in `run_fingerprint` trips `clippy::while_let_loop`, so that lint goes into the tools module's allow list in lib.rs. Toolchain 83.4% Nikaia.
Part: #125
Pushed: local 79e5571b508fdf80ede60cf93d9108203e16c39d
Tests: `it -- comptime build_time run_fast grammar_at_build_time dsl_grammar` 119 passed; `tests/compiler` comptime_* (7) passed with --both-settings, run as a scratch copy of the project (no tests/compiler test starts with run_); `--lib comptime_run::` has 0 tests; lower-std fixpoint holds; clippy clean with 1.97 and 1.99.1; pre-push-check green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB (the 1.99.1 clippy target dir was made and deleted again)
Rs2nika: kept 76 of 243 lines; costliest edits: the `let` and `ref` signatures redone for the host parameter (29 + 25 lines), the throwing calls rewritten as `catch` (17), the control flow of the Command/serde_json blocks replaced by host calls and `process::run` (15). Under 60%: the plan records it rather than stopping (bootstrap week)
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
