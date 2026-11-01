Result: done
Batch: k3b-comptime-bundle.md c1c-ledger-lookup.md
Owner: none
Notes: K3b run 3 cherry-picks 707d5e53 (clean at e1a88e49; #1106 fixed by 1a5fdf6d; old report renamed .run2). C1c moves `lookup`; its shape `(String, ref FnContract)?` was probed and runs after b26. Decided: ADR-386, `drain()` on BTreeMap/HashSet/BTreeSet (#1130, filed `ready,wip`, bug brief b32, which also undoes C1b's clone), and ADR-387, a view of a copy type is copied where the value is wanted (b26's Needs on #1107, comment 6560569364, bug brief b33). Both appended to bugs/batch.txt. b28's Found is already #1124. Pushed 05fffc84.
