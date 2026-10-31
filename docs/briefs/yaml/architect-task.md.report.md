Result: done
Batch: y9-1027-group-label.md, y2c-properties.md, y2b-keys-directives-limits.md
Owner: none
Input: CUT at y8 on Found (not RED: the `RED` file is empty; CI green to 511e513a, 49569ede..7f6b8151 running). y8 done (555fe6e9, `Fixes #1026`; Changelog line in its report, version raise owed).
Found: filed #1027 (`ready`, brief form): a label on a one-element group (`n:(P)`, `((P))`, `(text(..))`) reaches rustc "Cannot find value"; the backend winnow-grammar scopes a one-alternative labelled group; fix in `emit_pattern.nika` (emit the element without the parens). Probed on 7f6b8151; `n:(P | Q)` and `m:(P)?` already work. Not needed by yaml (the y2 draft labels rules directly), batched first as a small bug fix.
Changed: y2c notes y8 done and #1027; #928 *To build* 2b and *Found* updated (labels `ready` + `wip`); builder.md: probe each shape on the old binary before the first build; architect.md: a brief marks which test shapes were traced (y8 debrief).
