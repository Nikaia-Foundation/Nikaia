Result: done
Batch: p3-1053-reasons.md, p4-1054-loop-checks.md
Figure: off 1,493/2,583 indexes (57.8 %), 712/1,594 operations (44.7 %); prove-through-loops:on 1,525 (59.0 %), 719 (45.1 %); on 1ebd8fc's binary, 4 files refused because the binary predates 578fd67
Owner: none
Decided: p2's `Needs` (#1054 step 2). D10 takes its candidates from prover_loops' portfolio. `bounding_fits` may prove a one-way step by the goal `t <= y`, where `y` is a name whose type's range lies inside the operation's type (comment on #1054). The headline figure is counted at prove-through-loops:on (state.md).
Changed: p3 refreshed (figures, current common.part, proved.nika steps); p4 new; state.md rewritten with a *Lane facts* block (the architect debrief's request); comments on #1053 and #1054.
Changelog (owed from earlier reports): p1 lower-std --bounds (#1053); p2 in-loop asserts proved through their loop (ADR-361 D2, D3).
