Result: done
Files: docs/briefs/tools/j8-1019-cut-item-frame.patch docs/briefs/tools/j8-1019-cut-item-frame.md.report.md
Commit: briefs: winnow-grammar patch numbering a Cut leaving a repetition element (#1019 step 3d, backend)
Body: winnow-grammar e9de602c7b3720774a21fbd0fa205dab56ff729d (local in /home/user/winnow-grammar, on 4f8e6691 = origin/main) numbers a Cut leaving a repetition element with the same item N as a Backtrack and returns it as a Cut: repeat_recording_bounded, repeat_counting_bounded (repeat_recording/repeat_counting delegate to them), fold_loop (after stopped, as for a Backtrack), and in src/rt/stream.rs the item loop (Fail::Cut, item n+1) and attempt's Cut path (item k, before the rule frames). record, priorities and rendering untouched. New test tests/cut_test.rs a_cut_leaving_a_repetition_element_keeps_its_number: [{1},{2},{x}] has item 2 right above item; [{x}] gives exactly [item 1, item, list] (that item 1 comes from digit+ inside the element, not from the outer repetition); ("a" => "b")* on ababac holds item 3. No existing expectation changed.
Part: #1019
Pushed: local 54791dca028f88caead35564d8d74fdbe28695b9
Tests: cargo test in the clone green (76 test binaries ok); the new test fails on 4f8e6691 with src stashed (the [{1},{2},{x}] case: item 2 missing; the ababac case run alone: stack [pairs]; the [{x}] case is a guard and passes on both); cargo fmt --check clean
Gate: green
Deviation: cargo clippy --all-targets -D warnings in the clone fails on src/rt.rs:711 single_range_in_vec_init (vec![0..input.len()]), already there on 4f8e6691 and outside this change; left as it is
Found: winnow-grammar 4f8e6691 fails clippy 1.97 -D warnings at src/rt.rs:711 (clippy::single_range_in_vec_init, `return vec![0..input.len()]`)
Needs: help from main session: push winnow-grammar e9de602c7b3720774a21fbd0fa205dab56ff729d to keywan-ghadami/winnow-grammar main (patch: docs/briefs/tools/j8-1019-cut-item-frame.patch)
Changelog: none
Owner: none
Disk freed: 2.8 GB (/home/user/winnow-grammar/target); target/nikaia-project-tests 3.8G, / 8.8G free, no new cache entries over 100 MB
Rs2nika: none
