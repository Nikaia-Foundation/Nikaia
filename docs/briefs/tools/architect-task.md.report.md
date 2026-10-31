Result: done
Batch: j10-1019-backend-clippy.md j9-1108-panic-at-build-time.md
Owner: none
Pushed: rejected (behind origin/develop; the driver rebases)
Notes: j8 done (winnow-grammar e9de602c, local; patch docs/briefs/tools/j8-1019-cut-item-frame.patch). Its Found (backend clippy single_range_in_vec_init at src/rt.rs:711, there since 4f8e6691; the backend's CI runs clippy -D warnings) is j10, a second backend commit on e9de602c; no issue filed (no gh access to that repo), recorded in #1019 3d. Main session: after j10, push e9de602c and j10's commit to keywan-ghadami/winnow-grammar main; j6 run 2 (pin move, path markers) is queued after that push. qa-input.txt here is stale (older batch).
