Result: done
Files: crates/nikaia-std/src/tools/project_tests.nika crates/nikaia-std/src/tools/lowered/project_tests.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/project.rs docs/selfhosting-plan.md
Commit: project: shown_path into Nikaia
Body: `shown_path` moves to `project_tests.nika` (no clash, no prefix), sync, so `project.rs` calls `project_tests::shown_path` without `block_on`; where `fs::canonical` throws it keeps the name as given, as the Rust did. Step 1 is f8614268 (`fs::canonical`, `Path.strip_prefix`, `Path.clone`); P3a's, P3b's and P4's rows in the selfhosting plan record the `std::fs` prerequisite as met. lower-std fixpoint: only project_tests' lowered file and its index line changed.
Part: #125
Tests: tests/compiler --both-settings 206 ok; it file_paths fs_ test_runner output_test program_test + guard set 51 ok; nikaia --lib project:: 10 ok; clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB (target/nikaia-project-tests 4.0G, / 16G free; new entry over 100 MB: run-bundle/65d964130cc06fb2 300 MB)
Rs2nika: kept 0 of 4 lines (under 60%); costliest: `path.canonicalize() catch { path.to_path_buf() }` should be `fs::canonical(path, fs::Root::Anywhere) catch { path.clone() }` (x2), `strip_prefix(..)?.display()?.to_string() catch {..}` should be `(path.strip_prefix(root) ?? path).display()`, and `sync` on the signature dropped
Pushed: 594cb42d885ced94f26f1302e31c3160d8dbe234
