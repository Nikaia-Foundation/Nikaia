Result: done
Batch: y13-1037-dependency-places.md, y2c-properties.md, y14-1013-guard-inner-label.md
Owner: none
Pushed: rejected (local commit on develop; the driver rebases and pushes)
Input: CUT y12 on its Found (not a red CI; RED is empty). y12's Found is #1013 step 4 (a guard reading a name bound inside `text(..)`, NK1117), #1013 relabelled `blocked` -> `ready`, brief y14 after y2c (the package does not need it).
Probed: the y2c draft on today's develop checks, but 9 of 10 package tests fail in rustc: a dependency's struct with three view fields (`ref yaml::Node`) gets one lifetime (ADR-353 D7). Filed #1037 (ready), fixed by y13 before y2c's run 2. Found beside it: #1038 (ready, NK1104 on a dependency's view field returned beside an owned text; not needed by the package, for the bug lane).
Process: builder-facts gets the stale winnow-grammar rev trap (y12 debrief) and the `use compiler_tests` layout for cross-package tests; architect-task gets lane facts and a push rule without `pull --rebase`/stash (last architect's debrief). Note: CI does not run `packages/yaml/tests`.
#928: *To build* updated (y11, y12 done; y13, y2c, y14 next), labels ready+wip.
