Result: done
Batch: y23-scalar-written.md y21-events.md
Owner: none
Decided: ADR-yaml-1 D7 last paragraph: Node gets `pub raw: String` and `pub style: Style` for every scalar node (y21's Found; probe confirmed on 2a303980). Not payloads on Value::Null/Bool. Architect's decision, package-only, no version raise.
Changed: y23 written; y21 re-queued (run 1 report -> y21-events.md.run1.report.md, run 2 paragraph); #928 step 4 updated (ready + wip); builder-facts: package commit files, `for x in v.drain()`.
Found: none filed (y21's gap is a package design choice, decided in the ADR, not a compiler gap).
Changelog: none (y22's line is in its report, for the supervisor)
Pushed: rejected
