Result: done
Batch: p5-1069-bounds-expectation.md, p6-1053-for-counters.md
Figure: off 1518/2628 indexes (57.8 %), 729/1612 operations (45.2 %); prove-through-loops:on 1550/2628 (59.0 %), 736/1612 (45.7 %) (ecf435a, 246 files, 0 refused)
Owner: none
Decided: ADR-361 D13 (a `for` counts its turns; a counter is bounded by k times them), #1069 (`.bounds` / `.loops.bounds` beside a program test). Both under *Language decisions*; p5 and p6 write the Part III text and their Changelog lines.
Filed: #1066, #1067, #1068 (bug lane, `ready`); #1069 (`ready`, `wip`). Comments on #1053 (figure, plan) and #432 (D13 covers its loop part).
Supervisor: Part III `30-nikaia-tooling.md` :315 writes `time::Duration::new(30, 0)`, refused by NK1149; `async e` refused is #984, not a defect; the reassigned `split().collect()` probe did not reproduce.
