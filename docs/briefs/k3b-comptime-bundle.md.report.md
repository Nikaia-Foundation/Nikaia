Result: blocked
Files: none
Commit: none (the move is on local branch wip/k3b-comptime-bundle-run2 707d5e53, made without touching the shared index; my files in the shared tree are restored)
Body: I cherry-picked run 1 (9a1678a8) and re-lowered. #1077 and #1078 are fixed: neither of run 1's rustc errors comes back. GR has re-landed with its own `impl Error for CheckWall` (grammar_run.nika :28), so I dropped my copy from comptime_bundle.nika (it gave E0119 and E0592). `write_if_changed` is now a forward that calls `comptime_bundle_write_if_changed` with `unique_beside(path)`. The dead `cargo()` is deleted. `rustc`, `toolchain` and `target_libdir` stay plain Rust: `run_fast.rs` :20 calls `rustc`, and the brief's *Stays Rust* keeps the OnceLocks. With a temporary 6-line std fix (below, then removed) both `cargo check -p nikaia-std --features compiler` and `cargo check -p nikaia` are clean.
Part: #125
Pushed: none
Tests: lower-std runs clean. One rustc error is left in lowered/comptime_bundle.rs:176. The probe below reproduces it in a plain `nikaia run`. No it or compiler tests were run, since nothing was committed.
Gate: not run
Deviation: none
Found: Compiler/std defect. A `String` parameter that lowers to `EitherText` cannot be converted to `fs::Path`: `let p: fs::Path = text` lowers to `PathBuf::from(text)`, and rustc says `PathBuf: From<EitherText<'_>>` is not satisfied. Probe: `fn as_path(text: String) -> fs::Path { let path: fs::Path = text; return path }`, called once with a `split_once` part (`lines.iter().filter_map(fn(l) { l.split_once("=") }).map(fn((n, r)) { as_path(r) })`) and once with an owned `String`. Fix, verified: in crates/nikaia-std/src/either_text.rs add `impl From<EitherText<'_>> for std::path::PathBuf { fn from(t: EitherText<'_>) -> Self { PathBuf::from(t.as_ref() as &str) } }`, with a tests/compiler program built from the probe.
Needs: bug lane fixes the EitherText -> PathBuf conversion (one commit, a test, a version raise). Then K3b run 3 cherry-picks 707d5e53 (`--no-commit`, then `git restore --staged`), re-lowers, and runs the brief's tests. Also: lowered/emit_core_leaves.rs is re-lowered in the tree from another agent's uncommitted emit_core_leaves.nika. I did not stage or restore it.
Changelog: none
Owner: none
Disk freed: none; target/nikaia-project-tests 3.8G, / 6.0G free; no new cache entry over 100 MB
Rs2nika: none (nothing committed; score at the re-run)
