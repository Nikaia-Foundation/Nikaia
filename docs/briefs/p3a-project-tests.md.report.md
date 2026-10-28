Result: done
Files: crates/nikaia-std/src/tools/project_tests.nika crates/nikaia-std/src/tools/lowered/project_tests.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/project.rs scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: project: the test runner's discovery and verdicts into Nikaia (P3a)
Body: files_under, output_tests, copy_into, program_tests, is_refusal, does_not_compile, named_by, difference and verdict move into tools/project_tests.nika; Outcome (as ProjectTestsOutcome, verdict compares it), OutputTest and ProgramTest become Nikaia structs holding Bytes and fs::Path, re-exported by project.rs, whose run loop now compares Bytes with the process's Vec<u8> by deref. project.rs keeps output_tests and program_tests as one-line block_on adapters for main.rs. files_under is fs::walk (sorted, links followed once, an unreadable subdirectory left out); a missing expectation file is fs::exists before fs::read; is_refusal takes the text (a stderr file through from_utf8_lossy, the same answer for its ASCII markers). An unreadable test file now fails with std's IoError text (it names the path) instead of anyhow's "reading PATH". shown_path stays Rust: std::fs has no canonicalize and fs::Path no strip_prefix. Needed 9c199572 (Bytes in the tools preamble). Not added to rs2nika.names (the brief keeps crates/rs2nika/ unchanged): verdict and difference are project_tests_verdict and project_tests_difference, Outcome is ProjectTestsOutcome.
Part: #125
Pushed: b8547a635ac2a287bcff71e8c670d0cd56258e25
Tests: nikaia test --both-settings --project tests/compiler 205 passed (as before); it filters output_test program_test bless test_runner refusal plus the guard set 52 passed; nikaia --lib project:: 10 passed; lower-std fixpoint; clippy clean; self-hosting 80.3 %
Gate: green
Deviation: shown_path not moved (std::fs has no canonicalize, fs::Path no strip_prefix)
Found: none
Needs: std fs canonical path and fs::Path strip_prefix, for shown_path (P3b)
Changelog: none
Owner: none
Disk freed: 0 GB
Rs2nika: kept 28 of 117 lines; costliest: Rust std::path calls (file_stem, strip_prefix, components, is_dir) drafted onto fs::Path, which has none (read_dir+stems helper, fs::walk); collect::<Result<..>> chains with fallible reads drafted as one-line closures (loops); the optional() closure matching io::ErrorKind::NotFound drafted as a Rust match (fs::exists then fs::read)
