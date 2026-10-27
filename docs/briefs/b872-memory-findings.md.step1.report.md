Result: done
Files: crates/nikaia-std/src/tools/check_compare.nika crates/nikaia-std/src/tools/check_takes.nika crates/nikaia-std/src/tools/check_widen.nika crates/nikaia-std/src/tools/lowered/check_compare.rs crates/nikaia-std/src/tools/lowered/check_takes.rs crates/nikaia-std/src/tools/lowered/check_widen.rs docs/specification/10-nikaia-light.md tests/compiler/tests/let_moved_read_again.stderr
Commit: Messages and Part I name `scalar`, not `char`, among the types that copy, hash and order
Fixes: #874
Model: Opus 5.5
Tests: nikaia test --both-settings --project tests/compiler 137 passed; guard set 22 passed
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: NK2105, NK1260-adjacent `Eq`/`Ord` notes and the `[v; n]` note list `scalar`s, not `char`s; Part I 1695, 1902, 2384 likewise (Nikaia has no `char`).
Pushed: f1746cd9761d54e60e32a4b5fcb1775db4851469
