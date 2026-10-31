Result: done
Batch: p5c-project-drive.md, c1b-ledger-stored.md
Owner: none
Filed: #1106 (EitherText into fs::Path, blocks K3b; bug brief b25) and #1107 (a `ref T?` map view unwrapped by `??`/`match` loses `ref`, blocks C1c `lookup`; bug brief b26), both `ready,wip`, appended to docs/briefs/bugs/batch.txt.
Main: C1a done. K3b waits on b25 (run 2 on wip/k3b-comptime-bundle-run2 707d5e53). P5c re-runs (#1088 built, probe passes; old report run1). C1b is the stored fold and the deletion of the dead LedgerOps::candidates; lookup split off as C1c behind #1107.
Next: K3b run 3, C1c, and a converter brief for S1 (`.get(range)` by design NK1214, `else if let`, #886).
Process: b22 stopped on K3b's untracked tools file in the shared tree; K3b run 2 then worked on a branch, which is the pattern to keep.
