Result: done
Batch: j8-1019-cut-item-frame.md j9-1108-panic-at-build-time.md
Owner: none
Pushed: rejected (local 5e76e17; HEAD is 20 behind origin/develop and carries the batch's unsuite-tested j7/j5 commits: the driver rebases and pushes)
Found: j5's two defects filed: #1108 (panic() in a sync fn refused at build time, NK1152; this lane, `wip`, brief j9, which also removes j5's Vec/.remove(0) way around it) and #1109 (pattern inside a boxed named field fails in rustc; `ready`, for the bug lane). Also #1110 (`ready`, bug lane): rustc's non_upper_case_globals / unused_braces lints reach the user from generated comptime code.
Changelog (for the supervisor, from the reports): j7 #1095 a sync fn or comptime may call `id[i64](3)`; j5 #1087 a comptime in a turn may hand the field to a function and read its attributes, T? crosses from build time.
Next after this batch: j6 run 2 once the main session has pushed j8's winnow-grammar commit (moves the pin); 3c (`name:` per field, NK1311/NK1312), probe in #1019.
Needs from main session: push j8's winnow-grammar commit to keywan-ghadami/winnow-grammar main when j8 reports it.
