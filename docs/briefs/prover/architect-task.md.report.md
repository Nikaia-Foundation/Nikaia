Result: done
Batch: p1-1053-measure.md, p2-1054-in-loop-claims.md, p3-1053-reasons.md
Figure: 965/1,821 indexes (53 %), 560/1,381 operations (41 %), `check --bounds` per file on d206915; an undercount, since 227 of 246 files are refused standalone (p1 measures in package context)
Owner: none
Filed: #1053 (epic, ready+wip), #1054 (defect: in-loop claims not asked through their loop, ADR-361 D2, ready+wip); #385 labelled decision; comment on #796.
Reading corrected: the certificate checker comes after p2 and the counter extension, because its unproved sites are in-loop counters and indexes.
Pushed: rejected (local commit on develop; the driver rebases)
