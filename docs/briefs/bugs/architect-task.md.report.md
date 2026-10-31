Result: done
Batch: b1-999-mixed-tuple-views.md b2-939-shift-in-heads.md b3-1016-ref-list-result-slice.md
Owner: none
Pushed: rejected (local HEAD of develop; rebase refused: other agents' unstaged files in the shared tree; supervisor rebases)
Notes: first bug-lane batch (input BATCH DONE, no prior reports). All three probes re-run at c30b7bc0 and reproduce; #939 traced to the head chain in parser/mod.rs skipping the bit/shift levels, #1016 reduced to `fn f(a: ref Vec[i64]) -> ref Vec[i64] { return a }`. `wip` set on #999, #939, #1016 (status `ready` kept). Filed #1036 (ready): a tuple result holding a view of a temporary argument gets no keep (rustc), found probing #999. Held back: #947 (wider than filed, commented) until #938 of the main lane lands, both touch `check_core_stmt.nika`; #1002 lives in the pinned `winnow-grammar` (no `hex_digit` builtin there), needs a lowering mapping or a spec correction, next batch.
