Result: done
Files: crates/nikaia-std/src/tools/views.nika crates/nikaia-std/src/tools/lowered/views.rs tests/compiler/tests/views_handed_back_through_if_and_match.nika tests/compiler/tests/views_handed_back_through_if_and_match.stdout tests/compiler/tests/view_stored_in_a_branch_refused.nika tests/compiler/tests/view_stored_in_a_branch_refused.stderr
Commit: views: a list or struct of views handed back through an if, match or block is handed back
Body: The view checker's `handed_back` now follows an `if`, a `match` and a block to the value each branch, arm or block ends in, and asks each as handed back (via `walk_block` with `tail` true); the condition and the matched value are asked as before. `local_vec_handed_back` counts a block's tail naming the local list as a hand-back where that block's value is the result (the `let mut out = [..]; out` arm), and takes an `if`/`match` value whose every branch is a list literal (`list_shaped`) like a list literal, so `let r: Vec[E] = if .. { [..] } else { [] }; return r` lists its items back. A store inside such a branch (a push into a list that is not handed back) is still refused: view_stored_in_a_branch_refused. An owned-field assignment `self.last = n.name` was accepted before the change too, so the refused test pushes into a parameter list instead.
Fixes: #1085
Pushed: local 91b72357dbdaa74cdae980825f3aabf6c5935316
Tests: tests/compiler --both-settings 497 passed (both new tests ok; views_handed_back_through_if_and_match refused 5x NK2302 by the unfixed binary, nikaia-suite); it text_views stored_views tethers views_kept result_source keeps 174 passed; packages/yaml 16 passed; fmt::every_nika_file ok; yaml draft 1c1199d6 main.nika checks with no error; clippy 1.97 green, 1.99 not run (only the generated lowered/views.rs changed); pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A list or struct holding views of a `ref` parameter that is handed back through an `if`, a `match` or a block arm is now treated as handed back, not as kept past the call (NK2302).
Owner: none
Disk freed: 0 GB (no clone, no own target; cache target/nikaia-project-tests 3.7G, new entry run-bundle/3f8b93b88f7f213d 364M; / 9.3G free)
Rs2nika: none
