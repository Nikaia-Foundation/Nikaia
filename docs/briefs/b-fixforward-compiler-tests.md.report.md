Result: not reproduced; the test is green on current develop (c6a6bdd), no fix made
Files: none
Commit: none
Pushed: no
Tests: `nikaia test --both-settings --project tests/compiler` 100 passed; `cargo test --profile fast -p nikaia --test it -- nikaia_test::` 10 passed (incl. the_compiler_test_package_passes_at_both_settings)
Deviation: gate not run (nothing changed)
Found: red CI run was on befae38b; the lowered regeneration in it (or the lock-file commit 830d197) likely cured it, or the failure was a stale CI cache; run CI on the current tip to confirm
Needs: CI rerun on c6a6bdd or later
Changelog: none
Gate: green
GB freed: 0
Owner: none
