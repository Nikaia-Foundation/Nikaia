Result: done
Cause: none in the unpushed commits (6077173, ef8d832, 84aa402, fbd6ccd are briefs, a badge and a test fixture): examples::every_example_package_passes_its_own_tests failed in rs2nika/convert because the `rust-analyzer` rustup component was missing from the toolchain (17 output tests of that package print "rust-analyzer parse failed: Unknown binary")
Fixed: `rustup component add rust-analyzer` in this environment; no repository change, nothing committed
Tests: `nikaia test --both-settings --locked --project crates/rs2nika/convert` 139 passed; cargo test --profile fast every_example_package_passes_its_own_tests ok
Found: none. Note: the rs2nika convert/facts nikaia.{lock,proofs,contracts,derived} files are modified in the tree (not mine, left alone; --locked passes with them)
Files: none
Commit: none
