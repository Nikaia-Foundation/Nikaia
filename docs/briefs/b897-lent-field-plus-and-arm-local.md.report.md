Result: done
Files: crates/nikaia-std/src/tools/check_arm_leaves.nika crates/nikaia-std/src/tools/lowered/check_arm_leaves.rs tests/compiler/tests/an_owned_local_lent_through_arms.nika tests/compiler/tests/an_owned_local_lent_through_arms.stdout
Commit: check: an owned local in an arm of a lent argument is lent, not moved (#897 step 2)
Body: `lent_arms_view_and_new` counts a named arm that is neither a view nor a lent name as an owned local and lends it (`lent_reads`, `&t`): beside a value made there it goes into `Arm::View(&t)` (text: `either(&s)`); with only views and owned locals and no text, the arms are lent as they are (`&if c { &t } else { &u }`); text without a made arm still goes through `EitherText`, since `&String` and `&str` are two types. Step 1 is 68f83b0a.
Fixes: #897
Pushed: f76c4d862239f16514cfbe8ada38efb49e71ed50
Tests: an_owned_local_lent_through_arms (failed on the unfixed compiler: borrow of moved value `t`, `u`); a_text_plus_of_a_lent_field passes; nikaia test --both-settings tests/compiler 188 ok; it handed_over text_tiers typecheck + guard set 214 ok; clippy clean; nikaia-std unit tests not run (no Rust source changed, only lowered files)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: An owned local named in an arm of an argument to a lent parameter is lent through the call and stays usable after it.
Owner: none
Disk freed: 0
Rs2nika: none
