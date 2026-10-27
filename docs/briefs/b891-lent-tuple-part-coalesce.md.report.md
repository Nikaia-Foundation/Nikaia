Result: done
Files: crates/nikaia-std/src/tools/keeps.nika crates/nikaia-std/src/tools/lowered/keeps.rs tests/compiler/tests/coalesce_on_a_lent_tuple_part.nika tests/compiler/tests/coalesce_on_a_lent_tuple_part.stdout
Commit: keeps: a part moved out by `??` keeps its parameter, a tuple part as a field (#891)
Body: The left side of `??` is classified with keep_part instead of keep_named, so a tuple part (`p.0`, `p.1.0`) keeps its parameter as a struct field does: the parameter is taken, and the lowering's `unwrap_or_else` no longer moves out of a view. A `??` passed at a position the callee's result points into (`shown(p ?? "none")`) is walked side by side instead, so its parameter stays lent; before, `fn whole(p: String?) -> String { shown(p ?? "none") }` was refused with NK2302. Steps 1-3 in one commit.
Fixes: #891
Pushed: 3ba85829773378489473bd053285c67f3991844e
Tests: new coalesce_on_a_lent_tuple_part failed on the unfixed compiler (NK2302 on `shown(p ?? ..)`; without that form rustc's "cannot move out of `p.0`/`p.1.0`/`s.name`"), passes now; nikaia test --both-settings tests/compiler 176 passed; lower-std changed only lowered/keeps.rs (the fix itself), fixpoint
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: `??` on a part of a lent tuple parameter (`p.0 ?? ".."`) no longer fails in rustc
Owner: none
Disk freed: 0
