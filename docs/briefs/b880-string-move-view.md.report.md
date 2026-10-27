Result: done
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs crates/nikaia-std/src/tools/lowered/emit_core_match.rs crates/nikaia-std/src/tools/lowered/emit_program.rs tests/compiler/tests/string_move_not_view.nika tests/compiler/tests/string_move_not_view.stdout crates/rs2nika/convert/src/main.nika crates/rs2nika/convert/nikaia.contracts crates/rs2nika/convert/tests/entry_write.args crates/rs2nika/convert/tests/entry_write.in/f.rs crates/rs2nika/convert/tests/entry_write.stdout
Commit: ae26238c (Fixes #880, pushed); 1996ab8f trimmed `opt ?? ""` (Part of #880); f2bc4032 rs2nika R92 (Part of #877)
Fixes: #880
Pushed: ae26238c pushed; 1996ab8f f2bc4032 rejected (not rebased, per the rules)
Tests: new test failed (rustc Mismatched types) on the unfixed binary; nikaia test --both-settings 151 ok; it keep view move + guard set 366 ok; clippy clean; rs2nika 45 ok; issue's expected `ab||x` was a slip, Nikaia prints `ab|x`
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A `String` moved into another `String` variable beside a `(x.strip_prefix(..) ?? "").clone()` (trimmed or not) of it is lowered as a move; rustc refused it before (#880).
Owner: none
Disk freed: 0 GB (no clone made; target/nikaia-project-tests 2.2G, 20G free; new entries over 100 MB: 2 rlib 234/230 MB, 1 run-bundle 242 MB)
