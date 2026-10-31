Result: done
Batch: p11-1090-carried-precondition-count.md, p9-1053-claims-in-the-figure.md
Figure: off 1,536/2,659 indexes (57.8 %), 838/1,625 operations (51.6 %); prove-through-loops:on 1,568/2,659 (59.0 %), 844/1,625 (51.9 %); binary of 76c216e
Owner: none
Notes: p10's Found (a precondition carried back to a caller's entry, ADR-269 D15, counted as proved although the call to the caller proves nothing; NK1300 missing, the run stops) is the prover's own: filed #1090 (ready, wip), briefed as p11 ahead of p9, whose claim count would otherwise publish it as proved. p9 unchanged but for its pre-flight (needs p11) and a staging rule for `sysroot.rs`, which bug-lane b17 also edits. Next run: the prover walk's `for` (state item 12). Push: local develop holds p10 unpushed and is behind origin by 13; this commit is local, the driver rebases and pushes.
