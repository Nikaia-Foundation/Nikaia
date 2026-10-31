Result: done
Batch: y27-load.md y31-merge.md y30-1092-match-moves-owned-field.md
Owner: none
Input: CUT on y29's Found (not a red CI; the suite of y29 445f6064 still running). y29 done, #1089 fixed.
Filed: #1092 (bug, ready): a by-value parameter whose field a `match` moves out is inferred lent, lowered `&A`, rustc E0507; probe checked by the architect on target/fast/nikaia (the `??` spelling runs). Fix brief y30, last in the batch (nothing in yaml needs it).
Decided: ADR-yaml-1 D6, paragraph of 2026-10-31: `merge(node: Node, input: ref Bytes) -> Node sync throws` as serde_yaml 0.9's apply_merge (own keys win, then the first merged mapping; a non-mapping `<<` value an Error; quoted "<<" ordinary). Package ADR, no toolchain version.
y27-load.md unchanged (run 2 paragraph already there, unrun).
#928 To build updated (y29 struck, #1092, eleventh batch); labels ready+wip kept.
Pushed: rejected (local commit "briefs(yaml): y31 merge ..." on top of the unpushed y29 commits; the driver rebases)
