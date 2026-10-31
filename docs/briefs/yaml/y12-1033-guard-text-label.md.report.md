Result: done
Files: crates/nikaia-std/src/tools/emit_pattern.nika crates/nikaia-std/src/tools/emit_grammar.nika crates/nikaia-std/src/tools/lowered/emit_pattern.rs crates/nikaia-std/src/tools/lowered/emit_grammar.rs tests/compiler/tests/grammar_a_guard_reads_a_binding_text_label.nika tests/compiler/tests/grammar_a_guard_reads_a_binding_text_label.stdout
Commit: grammar: a guard reads a `text(..)` label whose pattern binds names (#1033)
Body: winnow-grammar writes a guard as its own closure `|i| guard(i, cond, what)` before the action: its `i` is hygienic (unreachable from the condition), and naming the rule's `input` there borrows it against the `parse_next(input)` that runs the closure. So a bound `text(..)` that a later guard in the same sequence names is now bound right after its element by a helper rule the grammar gets, as with `list`/`span`/`here`: `s:__nikaia_covered(at = FROM.start..TO.end)`, with `rule __nikaia_covered(at: Range<usize>) -> &'a str = empty -> { covered(input, at.clone()) }` (`covered_bytes::<_, E>(..)?` over bytes). A value rule is inlined into a closure whose parameter is `input`, so its action reads the text without a borrow conflict; `empty` consumes nothing. The action's prelude is unchanged. Grammars without such a guard lower as before. Probed by hand: text and bytes input, a nested `text(..)`, a label named `i`, two texts read by one guard.
Fixes: #1033
Pushed: 6e843b68 (pushed by drive.nika)
Tests: new grammar_a_guard_reads_a_binding_text_label passes; on the unfixed binary the same program failed in rustc with `Cannot find value s` (4:43); guard set green (24 passed, sequences figure not above its ceiling); clippy clean; pre-push-check ok; tests/compiler --both-settings 414 passed 0 failed (20 grammar_ ok); packages/yaml 10 passed; fmt::every_nika_file ok
Gate: green
Deviation: none
Found: the checker refuses a guard that reads a name bound inside a `text(..)` (NK1117), e.g. `entry rule plain -> String = s:text(d:Ch+) if(d.len() > 0) ";" -> { s }` gives "`d` isn't declared anywhere", though the action may read `d` (#1013) and the lowered Rust already binds `d` before the guard; the refusal is the checker's (check_grammar.nika)
Needs: supervisor note, environment: target/fast/nikaia had winnow-grammar rev 9e23474f baked in by a stale build.rs run while Cargo.toml pins 3b2dd3d6, so every grammar program failed in rustc with two winnow_grammar versions; `touch crates/nikaia/build.rs` and a rebuild fixed it
Changelog: A guard in a grammar can read a `text(..)` label whose pattern binds names, as the action can.
Owner: none
Disk freed: 0 GB (scratch project target and run caches, about 1 MB; target/nikaia-project-tests 5.9G, disk 8.7G free, no new entry over 100 MB)
Rs2nika: none
