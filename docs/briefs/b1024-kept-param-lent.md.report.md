Result: done
Files: crates/nikaia-std/src/tools/check_lambdas.nika crates/nikaia-std/src/tools/lowered/check_lambdas.rs tests/compiler/tests/a_lent_parameter_kept_by_a_trait_method_is_taken.nika tests/compiler/tests/a_lent_parameter_kept_by_a_trait_method_is_taken.stderr tests/compiler/tests/a_copied_parameter_kept_by_a_trait_method_is_used_again.nika tests/compiler/tests/a_copied_parameter_kept_by_a_trait_method_is_used_again.stdout
Commit: check: a fn parameter lent to a kept position is refused with NK2101 (#1024)
Body: A plain `s: String` parameter the function lends is bound as a view, which `takes_away` counts as copied, so a kept lambda capturing it recorded nothing and rustc refused the lent capture. `takes_of_a_lambda` now judges a lent parameter (a `CheckKind::Parameter` binding whose type is a view) by its owned type, as a local `String`; a copied parameter (`i64`) stays untouched. The same holds for a lambda kept by a free function, not only a trait method.
Fixes: #1024
Tests: the two new tests/compiler programs; the lent-parameter probe gave a raw rustc `Lifetime may not live long enough` on the unfixed compiler (checked `Checked`); tests/compiler --both-settings 400 passed; guard set 24 passed (sequences within its ceiling); clippy clean
Gate: green
Deviation: none
Found: the same program without the use after the lambda (`h.take(fn(x) { x + s.len() })` alone, or `keep(fn(x) { x + s.len() })` to a free fn keeping it) still reaches rustc's `Lifetime may not live long enough`: the keeps walk (`tools/keeps.nika`, `keeps_blocks_in`) counts a lambda's body as running during the call, so a parameter a kept lambda captures stays lent, where a `spawn` makes it kept (owned); fix belongs in keeps.nika, outside this brief's edit set
Needs: none
Changelog: A `String` parameter captured by a lambda a callee keeps and used afterwards is refused with `NK2101`, no longer by rustc (#1024).
Owner: none
Disk freed: 0.02 GB (scratch binary copy); cache 1.8G, disk 4.9G free, no new entry over 100 MB
Rs2nika: none
Pushed: ac8a38a6a57343152f3d15c6ed788d05ca85ddde
