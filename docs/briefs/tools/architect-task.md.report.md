Result: done
Batch: j6-1019-error-path.md, j7-1095-sync-bracket-call.md, j5-1019-comptime-field.md (run 2)
Owner: none
Decided: j5's Needs: step 2 goes through a generic value function in the build-time sub-program calling j3's field copy (no second rewriter); step 3 may edit grammar_decode.nika. j5's Found (1) filed #1095 (ready, wip: tools lane, brief j7); Found (3) filed #1096 (ready, bug lane). #1019 and #1087 updated.
Changelog: j5 run 1 (417abcf): A `comptime` in an unrolled turn over `T::fields` can read the turn's field name, `f.name`, directly or through a function (#1087).
Pushed: rejected (local is behind origin/develop and holds the untested 417abcf; the driver rebases and pushes)
