Result: done
Files: crates/nikaia-std/src/tools/modules.nika crates/nikaia-std/src/tools/lowered/modules.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/modules.rs docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: modules: tests as functions into Nikaia (M2)
Body: The cut: the loop over units (it needs `&mut Unit`) and the re-parse (`parse_to_ast`) stay Rust; Nikaia does the per-file work: `modules_test_count`, `modules_cli_word` (the entry's word for `std::cli`, over `use_segments`/`alias_of`), `modules_dispatcher_as_main` (drops `main`, renames the dispatcher to the symbol `main`), `modules_tests_as_functions` (one file's `Item::Test` into `Item::Fn` over `mut Vec[Spanned[Item]]`, numbered from `first`, returning titles and lines) and `modules_names_in` (one file, over `emit_walk::visited_in_*`; Rust unions the files). Symbols come from the interner through an `intern` callback (`intern_string`), so `parse_expression` is no longer used for `main` or the test names. The Rust `test_function` wrapper is deleted, and `is_a_test_function` too: it had no caller, its Nikaia counterpart is `prover_a_test_function` (`prover_walk.nika`).
Fixes: none
Part: #125
Pushed: 2000485cd5f01baa4a19ca65ca302ccc86222c70
Tests: nikaia test --both-settings --project tests/compiler 205 passed; it filtered on test_ dispatch names_in permission package + guard set 91 passed; nikaia --lib modules:: 0 tests; lower-std fixpoint (only lowered/modules.rs and its index line changed); self-hosting 80.4 %
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 13 of 89 lines; costliest edits: the units loop cut into per-file functions (the draft moved the whole Rust function, `Unit`, `parse_to_ast` and `anyhow::bail!` included), the inner `fn note` of `names_in` dropped by the draft (rewritten as `modules_note`), the `&mut` field rewrites (`item.node = ...`, `*name = main`) rebuilt as drain-and-push of `Spanned` items
