Result: done
Batch: y15-1015-fmt-guard-label.md y2c-properties.md y14-1013-guard-inner-label.md
Owner: none
Input: CUT y2c-properties.md blocked. The RED line is y2c's own package gate (fmt::every_nika_file), not CI. Run 2 built the step (11/11 package tests, wip/y2c-properties b223f972); only fmt was red, on #1015.
Decided: fix #1015 first (y15: a guard's `if(..)` on a rule line is never broken, and a rule's continuation line goes one level deeper; test tests/compiler/tests/fmt_keeps_a_grammar_guard_whole.nika). Then y2c run 3 applies run 2's branch, formats, runs the gate and commits. y14 stays third. The architect reproduced both fmt defects on develop.
Changed: y2c run 2's report and debrief renamed to *.run2.*; y2c brief: run 3 steps, 60 turns, newest rules block; y14 now names the order; batch.txt; builder.md: a run that starts from a draft or wip branch runs fmt, check and the tests on it before writing anything (y2c debrief). #928 To build updated (ready+wip); #1015 comment (stays ready).
Note: the main checkout holds run 2's uncommitted packages/yaml/nikaia.derived and nikaia.lock. y2c run 3 restores them by name (step 2).
Pushed: f5a6fd29 (db275fc8 before it holds only the renames)
