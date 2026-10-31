Result: done
Batch: y19-1002-hex-digit.md, y21-events.md
Owner: none
Red: RED 28bfacae (arm64, bootstrap_compiler) fails three sysroot fixpoint tests, red since 263d13d0. The cause is drift in `lowered/project_members.rs`, which no yaml commit wrote. y3's d715e425 (this lane, not pushed yet) already fixes it forward: `lower-std` at HEAD gives "already what this compiler lowers". The driver's suite push clears the red. I took no other fix and no revert.
Decided: ADR-yaml-1 D7 gets a paragraph saying what events need from the tree: `Node.flow`, and `documents()` returning `Document { root, start, end }`. `parse` stays and is `documents` with the roots taken out. This is a package ADR, so no version raise.
Issues: #928 *To build* now marks y20, y3 and #1000 done and lists y19 and y21 for step 4. Labels stay `ready`+`wip`. y3's Found (`ns[0].raw` moved out through an index reaches rustc) is commented on #927 as a second probe, `ready`, and nothing needs it.
Process: the briefs' rules block now says commit only, the driver pushes, and `Needs: architect decision`. y19 lacked its report line, which is now added. `docs/briefs/common.part` belongs to the main lane and I did not touch it.
Commit: d3d41acb
Pushed: none (local; pushing HEAD would carry the driver's 7 untested commits, so the driver pushes after its suite)
