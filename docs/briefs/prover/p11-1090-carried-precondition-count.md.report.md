Result: done
Files: crates/nikaia-std/src/tools/prover_passes.nika crates/nikaia-std/src/tools/lowered/prover_passes.rs tests/compiler/tests/precondition_carried_unproved.nika tests/compiler/tests/precondition_carried_unproved.stderr tests/compiler/tests/precondition_carried_proved.nika tests/compiler/tests/precondition_carried_proved.stdout
Commit: prover: a precondition carried back to a caller counts the caller's unproved calls (#1090)
Body: `count_checked_calls` counted only calls to the `assert`'s own function; a call carried back to its caller's entry (D15) records no reach, so `h`'s claim read "proved at every call" while `hb(u)` proved nothing (cause confirmed on a build as the brief read it). Now `carried_to` maps each function to the callers whose entries hold a claim carried from a call to it (`PreClaim.site.callee`), and `checked_through` sums the unproved calls over that closure, each function once (stops at cycles). Calls to a carried-to function that NK1207 already refuses are not counted again, mirroring what `check_claims_unproved` does for the function's own calls: without that, `it` test `assert::a_broken_precondition_names_the_caller` got an NK1300 on `percent` beside the NK1207 on `report(1, 0)`. Message text unchanged (it names `h`, which is right). No existing test changed: the 25 existing `tests/compiler` programs with `assert(` pass unchanged.
Fixes: #1090
Tests: new precondition_carried_unproved (NK1300) accepted by the unfixed binary (76c216e suite worktree: `Checked`, `preconditions proved at every call 1`, stops at run time), refused after; precondition_carried_proved prints `4 0` before and after; 27 `assert(` programs (the two new among them), `--both-settings`, in a scratch project: all pass; `cargo test -p nikaia --test it -- assert prove claim precondition`: 156 passed; lower-std fixpoint clean; clippy 1.97 and 1.99.1, fmt, pre-push-check clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A precondition carried back through a call is now proved only where the outer callers' calls prove it; a call that does not is refused with NK1300 instead of passing as "proved at every call" (#1090).
Owner: none
Rs2nika: none
Disk freed: 1.1 GB (clippy 1.99.1 target dir); cache target/nikaia-project-tests 3.5G, disk 9.6G free; new cache entry over 100 MB: run-bundle/3f8b93b88f7f213d 344 MB
Pushed: local 9bfe20167e4076c4eb70265de42eb4bc0d006a39
