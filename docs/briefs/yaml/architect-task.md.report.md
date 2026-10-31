Result: done
Batch: y22-1063-view-field-copy.md y21-events.md
Owner: none
Decided: y19's Deviation (two error offsets of a bad `\x`/`\u` escape now point at the offending character) accepted: it is "where reading stopped" (the package's own error rule); recorded on #928.
Found: #1063 (filed, `ready` + `wip`), by the architect probing step 4: a view field of a lent struct (`w.text` of a `ref Word`, a `for` element) stored in a new struct is refused NK2106; in `yaml` it cascades into NK1106 across the grammar and NK1127 on `parse`'s default. Both `events` (anchor/tag per event) and `load` (D6: strings still views) need it, so y22 fixes it first and y21 gets a pre-flight probe.
Process: y21's rules block had no report form or `Attribution:`; refilled from y19, and the lane fact in architect-task.md now says to copy from the newest brief that ends in `Attribution:`.
Next after this batch: `load` (aliases within the alias limit, duplicate keys, `<<` ordinary, merge on request), probed on the fixed binary (scratch probe of copy_node with an anchors map also hit NK2305 on `anchors` returned through `target.clone()`: recheck after #1063).
Pushed: see commit
