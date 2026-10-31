Result: done
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/check_function.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/check_function.rs tests/compiler/tests/match_on_a_field_of_a_get_bound_local.nika tests/compiler/tests/match_on_a_field_of_a_get_bound_local.stdout
Commit: check: a local a get binds past a jump is a view; a match on its field borrows (#1089)
Body: The let walk already recorded `let a = m.get(k) ?? <jump>` in `got_past_a_jump`, but `check_core_flow_through_a_view` (which decides the `&` of a match's scrutinee) did not read it, and the `match m.get(k) { null => <jump>  x => x }` spelling was not recognised at all. Now `check_core_stmt_got_by_a_match` recognises that spelling (two unguarded arms: a null arm that jumps, a binding handed back as it is), the let records it in `got_past_a_jump` (the `?? jump` form keeps its own `mut` path unchanged), and through_a_view takes a name in `got_past_a_jump` as a view, so `match a.node` lowers `match &a.node`, over a map's or a `ref Vec`'s `get`. `got_past_a_jump` was never cleared between functions (a name could leak into another function's binding of the same name); it is now cleared where `lent_lets` is, in `check_function.nika`. Row e (`a.node ?? throw` on such a local) already ran at 8073f652 and is kept as a test row.
Fixes: #1089
Tests: new tests/compiler/tests/match_on_a_field_of_a_get_bound_local (rows a-h plus an owned local) refused by the unfixed binary (6 x rustc E0507), passes fixed; probes a-h print 2 fixed; tests/compiler --both-settings 500 passed; it text_views stored_views tethers views_kept 67 passed; clippy 1.97 and pre-push-check green, clippy 1.99.1 green; eef3a2e7's yaml main.nika: `check` clean, a scratch copy calling `load` once (default written as a Limits literal) builds and prints 1 (unfixed: E0507 at `match a.node`)
Gate: green
Deviation: none
Found: a non-`ref` parameter whose field a match moves out is inferred lent and lowered `&A`, then rustc E0507: `fn owned(a: A) -> N sync throws { let node = match a.node { null => throw E { msg: "no node" }  x => x }  return node }` called with `owned(A { node: N { v: [1, 2, 3] } })` (types as in the brief's probe), same on 8073f652's binary
Needs: none
Changelog: A local bound by a `get` whose miss jumps (`m.get(k) ?? throw ..`, or `match m.get(k) { null => throw ..  x => x }`) is a view of the element, so a `match` on one of its fields borrows it instead of being refused by rustc.
Owner: none
Disk freed: 0.65 GB (clippy 1.99 target); cache target/nikaia-project-tests 3.8G, new entry run-bundle/3f8b93b88f7f213d 364 MB, disk 7.7G free
Rs2nika: none
Pushed: local 445f606452d926c412ee3e9bd8dc0fe1e6599b79
