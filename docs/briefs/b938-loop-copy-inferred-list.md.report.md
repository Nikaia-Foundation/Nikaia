Result: partial
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs tests/compiler/tests/a_loop_over_an_untyped_number_list_copies.nika tests/compiler/tests/a_loop_over_an_untyped_number_list_copies.stdout
Commit: check: a loop over an untyped number list binds the value (#938)
Body: An integer literal types as Unknown until its open number (ADR-285 D34) is decided after the body, so the element of `[1, 2]` is Unknown at the `for` and a_copy_by_value never counted it. The `for` already joins a walked local's open element number; where it does and the walk lends, the binding is now recorded in copied_loop_bindings (and is no lent view), since that element is a number whatever type it gets. The nested rows (`for r in grid { for n in r }`, `for n in grid[0]`, with `let grid = [[1, 2], [3]]`) have a second cause and are left out of the test: no open number exists for a list of lists, so the emitter writes no type for `grid`, rustc falls back to i32, and a copy alone then fails as `<i32 as Value>::Out == i64` (probed with a replica of num::value).
Part: #938
Pushed: a6ec14b34e088adf976662017fc01afe1a91f320
Tests: new test fails on the unfixed compiler (rustc Mismatched types at line 11), prints 6 fixed; tests/compiler --both-settings 415 passed 0 failed (both new and model test ok); guard set green (sequences ok); clippy clean; pre-push-check clean
Gate: green
Deviation: grid rows of step 1 not built (second cause, see Found); test holds the row case only; Part instead of Fixes
Found: #938 nested rows: `let grid = [[1, 2], [3]]` then `for r in grid { for n in r { twice(n) } }` and `for n in grid[0] { twice(n) }` fail in rustc (Mismatched types, p.nika:16, :22): a list of number lists gets no open element number (ADR-285 D34 covers one level), so its type is never written and rustc infers i32
Needs: architect files the nested-list open number (D34 one level deeper, or a decision whether it extends)
Changelog: A loop over a list of number literals with no declared type binds each number by value, as one over `Vec[i64]` does.
Owner: none
Disk freed: 0.04 GB (two scratch copies of the binary); cache 1.8G, 13G free, no new entries over 100 MB
Rs2nika: none
