Result: partial (step 1 done; step 2 blocked)
Files: crates/nikaia-std/src/lib.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/modules.rs crates/nikaia-std/src/tools/lowered/threads.rs crates/nikaia-std/src/tools/modules.nika crates/nikaia-std/src/tools/threads.nika crates/nikaia/src/sysroot.rs
Commit: lower-std: test blocks of a tools file lower into its #[cfg(test)] tests (#1147)
Body: A tools file with `test` blocks is parsed again in `lower_one` (sysroot.rs, thin adapter), `modules_tests_as_functions` turns them into `__nikaia_test_k` throws functions, and the new `modules_test_wrappers` (modules.nika) writes `#[cfg(test)] mod nikaia_tests { #[test] fn <snake_title>() { super::__nikaia_test_k().unwrap() } }` after the file's Rust; that file's check facts are not reused. lib.rs: the tools module imports the `.shown()` traits an `assert` lowers to. First block: threads.nika `worst_of` joins answers.
Part: #1147
Pushed: local f91284c2613dbec98fbbd15434623cc3bdfaaa0c
Tests: `cargo test -p nikaia-std --features compiler threads_lowered` passes; with `assert(!worst.may())` flipped it fails (`assertion failed`), flipped back and lowered again; the old binary refuses the block ("can't translate this item yet"); cargo clippy --all-targets clean; pre-push green. No suite run.
Gate: green
Deviation: the first block asserts `worst_of`, not send.rs's first test (plain data crosses): see Found. Step 2 not built.
Found: none
Filed: #1152 (a struct with ref fields cannot be built from a local or a ref parameter; text_tiers.nika NullCoalesce error is another agent's uncommitted work)
Needs: #1152 before step 2
Changelog: lower-std: `test` blocks of a tools file lower into `#[cfg(test)]` tests of its lowered file (#1147)
Disk freed: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
