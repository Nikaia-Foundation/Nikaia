Result: done
Files: see the step reports p557e-parser-helpers2.md.step1.report.md (compiler defect) and .step2.report.md (the move)
Commit: 5853ffbf checker: a part a taking arm takes out is lent to a ref parameter with its &; 80bf6cda parser: flat_shape, lambda, Parsed's logic and the grammar types' expansion from tools/*.nika (#557 step 4b)
Body: All three steps are built, in one commit (they share parser/mod.rs and lowered.rs). One compiler defect was fixed as its own commit, in tools/check_core_flow.nika, a file of the bug lane: a part that a taking arm takes out of a `mut` parameter was passed to a `ref` parameter without its `&` (E0308). The new test tests/compiler/tests/a_part_a_taking_arm_takes_out_is_lent_to_a_ref_parameter fails on the unfixed compiler.
Part: #557
Pushed: local 80bf6cda
Tests: new test fails on the unfixed compiler (rustc E0308) and passes after the fix; it filtered on parse, grammar and span: 147 passed; tests/compiler --both-settings: 571 passed, 7 failed (6 call target/fast/nikaia by a relative path when run by hand, and agent_script on a /tmp file); no .stderr changed; errors:: and the full it binary not run: the first full run failed en masse with the disk full (64 MB free), and later another agent's unfinished describe change stopped the nikaia lib from compiling; another build replaced target/fast/nikaia twice during the runs; clippy clean; lower-std --no-cache fixpoint holds
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: target/nikaia-project-tests removed (about 2 GB); 2.1G free on / at the end, close to the 2 GB stop line
Rs2nika: none
