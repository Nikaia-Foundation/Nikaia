Result: done
Cause: 783e576d (keeps: a copied scalar is never kept) broke `sysroot::tests::the_per_file_cache_follows_the_interface` (bootstrap_compiler arm64): its last step changed the contract by returning an i64 parameter bare, which is no longer a keep
Fixed: the test's two files now use a `String` parameter pushed into a `Vec` (`s.clone()` vs `s`), so the last step still changes the inferred contract (a test that stayed valid under the new rule, no expectation loosened)
Tests: `cargo test -p nikaia --lib the_per_file_cache_follows` ok; clippy -p nikaia --all-targets clean; fmt
Found: none (pre-push "version" red: assets/badges/self-hosted.svg out of date, not from this change; supervisor owns badges)
Files: crates/nikaia/src/sysroot.rs
Commit: sysroot test: change the inferred contract with a String parameter, not a copied scalar
Changelog: none
Owner: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 6a09623428d463ac327380be8675343f0bf142f9
