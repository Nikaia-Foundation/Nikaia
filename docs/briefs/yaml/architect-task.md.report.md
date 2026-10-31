Result: done
Batch: y24-1070-by-value-enum-not-a-view.md y25-1071-list-of-views-literal-extend.md y26-1072-payload-view-field.md
Owner: none
Filed: #1070, #1071, #1072 (compiler, ready, wip; brief form, probes re-run and narrowed on 11f808ef): y21 run 2's three view-checker defects. (1) only a user enum by-value parameter, `i64` passes (views.nika rsrc_function / rsrc_scalar); (2) a list literal, a non-empty start or `extend` on a list handed back (views.nika local_vec_handed_back, push only); (3) a match payload binding alone, no `for` needed (text_tiers.nika bind_a_payload).
Next: y21-events.md run 3 after the batch (report moved to run2, Run 3 paragraph added; draft branch wip/y21-events pushed to origin). Tags written as the tree keeps them is no Deviation now.
Process: docs/architect.md, a pre-flight probes the planned shape of a walk over borrowed data (y21 run 2).
#928: To build step 4 updated, keeps ready + wip.
Pushed: rejected (develop moved on origin; the driver rebases)
