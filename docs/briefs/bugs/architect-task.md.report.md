Result: blocked
Batch: unchanged (red not from bugs: gate-error, `scripts/gate.nika` refused to compile with `Cannot find Stream in process` at its `process::run` calls; on 46e6179f the same binary (`cmp` identical to target/fast/nikaia) checks and runs `gate.nika` cleanly in /home/user/nikaia-suite, so no commit of this lane, nor any commit, shows the fault: re-run the suite). b12 is done (e95ba850, Fixes #1061). Ready to append after b15 once green: b16-1065-field-call-lends.md (b12's Found, filed #1065, traced to check_core_a_called_field / emit_core_a_field_call, `wip`).
Owner: none
Pushed: rejected (origin/develop moved; f38eb4b3 sits on b12's untested e95ba850, so it waits for the driver's suite and push)
