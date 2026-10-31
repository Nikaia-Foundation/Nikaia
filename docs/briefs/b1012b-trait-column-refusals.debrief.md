1. **What cost the most.** Working out the plumbing the brief left open: how the impls' own column gets from `keeps::infer` through `Noted`, the `ProgramHost` and `Program` to both `changed_contracts` and `CheckWorld`. Finding where the checker turns a kept lambda into NK2101 took about four reads across `check_core_args`, `check_lambdas`, `check_takes` and `check_core_call_on`. The second biggest cost was my own: I ran `nikaia fmt` on the touched `.nika` files, it rewrote them with tabs and rewrapped them, and I had to rebuild seven files from HEAD and re-apply my edits.

2. **What the brief could have handed me:**
   - Where a kept lambda becomes NK2101 (`a_kept_lambda_takes_these` → `moved_into_a_task` → `a_task_took_what_is_used_again`).
   - That the committed ledger is simply `nikaia.contracts` on disk, so a test only has to build twice and needs no git.
   - Where `changed_contracts` is called (project.rs → prove.rs → contract_changes.nika).
   - The one line in project.rs where `newly` is built with the program in scope.
   - Nikaia's parameter syntax (`mut t: T`, not `t: ref mut T`) and that a literal is never taken, so a lending test needs an owned String.

3. **Checks.**
   - Useful: the lower-std/build cycle (ran twice, because two `Newly` initialisers lacked the new field), the hand probes of (g) and (h) with a scratch binary, `tests/compiler --both-settings`, the `it` gate set, `--lib`, clippy and pre-push.
   - Wasted:
     - `nikaia fmt --check` and `fmt`: the guard checks only that formatting is idempotent, not that files are formatted.
     - A `--lib` run whose filters lacked `--`, so it printed nothing.
     - A whole second `tests/compiler` run just to confirm one updated `.stderr`, because `nikaia test` has no filter.
   - Missed: I expected that `a_lent_parameter_kept_by_a_trait_method_is_taken.stderr` would change with the NK2101 wording, but did not update it before the first run.
   - Not done: neither new test was run against the unfixed compiler, because no older binary was on disk.

4. **The one change that would have saved most.** A line in builder.md: "Never run `nikaia fmt` on `crates/nikaia-std/src/tools/*.nika` or the tests; `fmt::every_nika_file` checks only idempotence." After that, the brief naming the checker's NK2101 path and the `Newly` construction sites up front.

Also: the push was rejected because `develop` moved on, so the brief's "pushed" condition is not met. Commit a39c219f still needs your rebase and push.
