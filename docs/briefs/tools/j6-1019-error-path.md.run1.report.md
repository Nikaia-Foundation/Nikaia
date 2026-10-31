Result: blocked
Files: none
Commit: none
Pushed: none
Tests: none run (nothing built); the brief's probe run on target/fast/nikaia at 843e716 prints the 21 `in` lines and no `item N` frame between `Value__Contact` and `Value__Vec_Contact_`, though the failure is in list element 1: the Cut drops the backend's element frame
Gate: green
Deviation: none
Found: none
Needs: architect decision: the list index cannot reach the error without a change to winnow-grammar 4f8e669. (1) `src/rt.rs`'s repetitions (about lines 1005, 1082, 1248) and `src/rt/stream.rs` (515, 868) call `e.item(n)` only on `ErrMode::Backtrack`; a Cut error takes `Err(e) => return Err(e)` and loses the index. Every `members[S]` failure is a Cut (`k:Key => ...`, `fail` writes `ErrMode::Cut`), and so is the probe's `"{" =>`, so `$.contacts[1]`/`[0]` cannot be told in steps 1 and 2. (2) An `item N` frame does not say which repetition it came from (`digit+` inside an element also pushes `item 1`), and the template rule `__nikaia_list` is inlined by the macro (no frame of its own), so the list's repetition cannot be marked. (3) A frame's name is a static Label, and an `extern rule` call maps every error to Backtrack (codegen/expr.rs 1348-1358), so a hand-written list cannot carry it either. The backend must offer one of: (a) repetitions that also number a Cut error, plus a way to mark the list's repetition (for example a non-inlined rule boundary); or (b) a pattern that pushes a frame computed at run time onto a failing error, Backtrack or Cut (for example `frame(expr, p)`, using `Diagnostics::item`'s path). Field segments need no backend change: a marker rule per field, `__nikaia_members_N_field_NAME` wrapping `_value`'s alternative i, read and dropped in `nikaia_std::grammar::parse*` before `render`. I left that half unbuilt on purpose, because a path without its indices (`$.contacts.mail`) would be wrong
Changelog: none
Owner: none
Disk freed: 0 GB (target/nikaia-project-tests 3.6G, / 11G free; no new cache entries)
Rs2nika: none
