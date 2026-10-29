Result: done
Pushed:
Tests: split_once_default_owned (new; failed unfixed with 4x E0277, passes at both settings); it 23 passed; tests/compiler 332 passed, 2 failed (a_ref_nullable_narrowed is lane C's untracked test; mut_text_pushed_then_trimmed is #915's test, its fix not in this commit)
Deviation: none
Found: none
Needs: none
Changelog: Fixes the `??` default of `split_once` / `rsplit_once`: an owned tuple default (`("a".clone(), b)`) now lowers to views of it, a computed one maps the found pair to owned text (#908). Decided: the answer stays a pair of views when every default part is a literal or a place; otherwise both sides are owned `String`s.
Gate: green
Files: crates/nikaia-std/src/tools/emit_core_flow.nika crates/nikaia-std/src/tools/lowered/emit_core_flow.rs tests/compiler/tests/split_once_default_owned.nika tests/compiler/tests/split_once_default_owned.stdout assets/badges/self-hosted.svg docs/briefs/lane-e.md.report.md
Commit: Lower an owned `??` default of split_once to views of it
Fixes: #908
